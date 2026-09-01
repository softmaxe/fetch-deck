use std::{collections::VecDeque, path::PathBuf, process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::{mpsc, watch},
    time::timeout,
};

use crate::{
    domain::{JobProgress, MediaMetadata},
    yt_dlp::{
        CommandSpec, YtDlpErrorKind, classify_error, parse_output_line, parse_probe_json,
        parse_progress_line,
    },
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug)]
pub enum RuntimeCommand {
    Probe {
        request_id: u64,
        command: CommandSpec,
    },
    Enqueue {
        job_id: String,
        command: CommandSpec,
    },
    Cancel {
        job_id: String,
    },
    Shutdown,
}

#[derive(Debug)]
pub enum RuntimeEvent {
    ProbeFinished {
        request_id: u64,
        result: Result<MediaMetadata, String>,
    },
    JobStarted {
        job_id: String,
    },
    JobProgress {
        job_id: String,
        progress: JobProgress,
    },
    JobLog {
        job_id: String,
        line: String,
    },
    JobOutput {
        job_id: String,
        path: PathBuf,
    },
    JobFinished {
        job_id: String,
    },
    JobFailed {
        job_id: String,
        kind: YtDlpErrorKind,
        message: String,
    },
    JobCancelled {
        job_id: String,
    },
    Stopped,
}

pub struct RuntimeHandle {
    pub commands: mpsc::UnboundedSender<RuntimeCommand>,
    pub events: mpsc::UnboundedReceiver<RuntimeEvent>,
}

#[derive(Debug)]
struct DownloadRequest {
    job_id: String,
    command: CommandSpec,
}

struct ActiveDownload {
    job_id: String,
    cancel: watch::Sender<bool>,
}

pub fn spawn() -> RuntimeHandle {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    tokio::spawn(queue_actor(command_rx, event_tx));
    RuntimeHandle {
        commands: command_tx,
        events: event_rx,
    }
}

async fn queue_actor(
    mut commands: mpsc::UnboundedReceiver<RuntimeCommand>,
    events: mpsc::UnboundedSender<RuntimeEvent>,
) {
    let (finished_tx, mut finished_rx) = mpsc::unbounded_channel::<String>();
    let mut pending = VecDeque::new();
    let mut active: Option<ActiveDownload> = None;
    let mut shutting_down = false;

    loop {
        tokio::select! {
            command = commands.recv(), if !shutting_down => {
                match command {
                    Some(RuntimeCommand::Probe { request_id, command }) => {
                        tokio::spawn(run_probe(request_id, command, events.clone()));
                    }
                    Some(RuntimeCommand::Enqueue { job_id, command }) => {
                        pending.push_back(DownloadRequest { job_id, command });
                    }
                    Some(RuntimeCommand::Cancel { job_id }) => {
                        if let Some(current) = active.as_ref().filter(|current| current.job_id == job_id) {
                            let _ = current.cancel.send(true);
                        } else if let Some(index) = pending.iter().position(|request| request.job_id == job_id) {
                            pending.remove(index);
                            let _ = events.send(RuntimeEvent::JobCancelled { job_id });
                        }
                    }
                    Some(RuntimeCommand::Shutdown) | None => {
                        shutting_down = true;
                        for request in pending.drain(..) {
                            let _ = events.send(RuntimeEvent::JobCancelled { job_id: request.job_id });
                        }
                        if let Some(current) = &active {
                            let _ = current.cancel.send(true);
                        }
                    }
                }
            }
            finished = finished_rx.recv(), if active.is_some() => {
                if finished.as_deref() == active.as_ref().map(|download| download.job_id.as_str()) {
                    active = None;
                }
            }
        }

        if active.is_none()
            && !shutting_down
            && let Some(request) = pending.pop_front()
        {
            let (cancel_tx, cancel_rx) = watch::channel(false);
            active = Some(ActiveDownload {
                job_id: request.job_id.clone(),
                cancel: cancel_tx,
            });
            tokio::spawn(run_download(
                request,
                events.clone(),
                finished_tx.clone(),
                cancel_rx,
            ));
        }

        if shutting_down && active.is_none() {
            break;
        }
    }
    let _ = events.send(RuntimeEvent::Stopped);
}

async fn run_probe(
    request_id: u64,
    spec: CommandSpec,
    events: mpsc::UnboundedSender<RuntimeEvent>,
) {
    run_probe_with_timeout(request_id, spec, events, PROBE_TIMEOUT).await;
}

async fn run_probe_with_timeout(
    request_id: u64,
    spec: CommandSpec,
    events: mpsc::UnboundedSender<RuntimeEvent>,
    limit: Duration,
) {
    let mut command = command_from_spec(&spec);
    command.kill_on_drop(true);
    let result = match timeout(limit, command.output()).await {
        Ok(Ok(output)) if output.status.success() => String::from_utf8(output.stdout)
            .map_err(|error| format!("yt-dlp returned invalid UTF-8: {error}"))
            .and_then(|json| parse_probe_json(&json).map_err(|error| error.to_string())),
        Ok(Ok(output)) => Err(user_error_message(&String::from_utf8_lossy(&output.stderr))),
        Ok(Err(error)) => Err(format!("Could not start yt-dlp: {error}")),
        Err(_) => Err(format!(
            "Probe timed out after {} seconds; close the selected browser and retry",
            limit.as_secs()
        )),
    };
    let _ = events.send(RuntimeEvent::ProbeFinished { request_id, result });
}

async fn run_download(
    request: DownloadRequest,
    events: mpsc::UnboundedSender<RuntimeEvent>,
    finished: mpsc::UnboundedSender<String>,
    mut cancel: watch::Receiver<bool>,
) {
    let job_id = request.job_id;
    let mut command = command_from_spec(&request.command);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    configure_process_group(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = events.send(RuntimeEvent::JobFailed {
                job_id: job_id.clone(),
                kind: YtDlpErrorKind::Unknown,
                message: format!("Could not start yt-dlp: {error}"),
            });
            let _ = finished.send(job_id);
            return;
        }
    };
    let process_id = child.id();
    let _ = events.send(RuntimeEvent::JobStarted {
        job_id: job_id.clone(),
    });

    let (line_tx, mut line_rx) = mpsc::unbounded_channel();
    if let Some(stdout) = child.stdout.take() {
        spawn_line_reader(stdout, false, line_tx.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_line_reader(stderr, true, line_tx);
    }

    let mut stderr_tail = VecDeque::with_capacity(80);
    let mut wait = Box::pin(child.wait());
    let outcome = loop {
        tokio::select! {
            changed = cancel.changed() => {
                if changed.is_ok() && *cancel.borrow() {
                    if let Some(process_id) = process_id {
                        interrupt_process_group(process_id);
                    }
                    match timeout(Duration::from_secs(3), &mut wait).await {
                        Ok(_) => {}
                        Err(_) => {
                            if let Some(process_id) = process_id {
                                kill_process_group(process_id);
                            }
                            let _ = wait.await;
                        }
                    }
                    break DownloadOutcome::Cancelled;
                }
            }
            line = line_rx.recv() => {
                if let Some((is_stderr, line)) = line {
                    handle_child_line(&job_id, is_stderr, line, &events, &mut stderr_tail);
                }
            }
            status = &mut wait => {
                while let Some((is_stderr, line)) = line_rx.recv().await {
                    handle_child_line(&job_id, is_stderr, line, &events, &mut stderr_tail);
                }
                match status {
                    Ok(status) if status.success() => break DownloadOutcome::Finished,
                    Ok(_) => break DownloadOutcome::Failed(stderr_tail.into_iter().collect::<Vec<_>>().join("\n")),
                    Err(error) => break DownloadOutcome::Failed(format!("Could not wait for yt-dlp: {error}")),
                }
            }
        }
    };

    match outcome {
        DownloadOutcome::Finished => {
            let _ = events.send(RuntimeEvent::JobFinished {
                job_id: job_id.clone(),
            });
        }
        DownloadOutcome::Cancelled => {
            let _ = events.send(RuntimeEvent::JobCancelled {
                job_id: job_id.clone(),
            });
        }
        DownloadOutcome::Failed(stderr) => {
            let _ = events.send(RuntimeEvent::JobFailed {
                job_id: job_id.clone(),
                kind: classify_error(&stderr),
                message: user_error_message(&stderr),
            });
        }
    }
    let _ = finished.send(job_id);
}

fn handle_child_line(
    job_id: &str,
    is_stderr: bool,
    line: String,
    events: &mpsc::UnboundedSender<RuntimeEvent>,
    stderr_tail: &mut VecDeque<String>,
) {
    if let Some(progress) = parse_progress_line(&line) {
        let _ = events.send(RuntimeEvent::JobProgress {
            job_id: job_id.to_owned(),
            progress,
        });
    } else if let Some(path) = parse_output_line(&line) {
        let _ = events.send(RuntimeEvent::JobOutput {
            job_id: job_id.to_owned(),
            path,
        });
    } else {
        if is_stderr {
            if stderr_tail.len() == 80 {
                stderr_tail.pop_front();
            }
            stderr_tail.push_back(line.clone());
        }
        let _ = events.send(RuntimeEvent::JobLog {
            job_id: job_id.to_owned(),
            line,
        });
    }
}

enum DownloadOutcome {
    Finished,
    Cancelled,
    Failed(String),
}

fn command_from_spec(spec: &CommandSpec) -> Command {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args).stdin(Stdio::null());
    command
}

fn spawn_line_reader<R>(reader: R, is_stderr: bool, lines: mpsc::UnboundedSender<(bool, String)>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = BufReader::new(reader).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if lines.send((is_stderr, line)).is_err() {
                break;
            }
        }
    });
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn interrupt_process_group(process_id: u32) {
    // The child starts a new process group, so a negative PID reaches yt-dlp and ffmpeg.
    unsafe {
        libc::kill(-(process_id as i32), libc::SIGINT);
    }
}

#[cfg(not(unix))]
fn interrupt_process_group(_process_id: u32) {}

#[cfg(unix)]
fn kill_process_group(process_id: u32) {
    // SIGKILL is only used after the graceful cancellation window expires.
    unsafe {
        libc::kill(-(process_id as i32), libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_process_group(_process_id: u32) {}

fn user_error_message(stderr: &str) -> String {
    stderr
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .unwrap_or("yt-dlp failed without an error message")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_returns_parsed_metadata() {
        let handle = spawn();
        let RuntimeHandle {
            commands,
            mut events,
        } = handle;
        commands
            .send(RuntimeCommand::Probe {
                request_id: 7,
                command: CommandSpec {
                    program: PathBuf::from("/bin/sh"),
                    args: vec![
                        "-c".into(),
                        "printf '%s' '{\"id\":\"v1\",\"title\":\"Clip\",\"formats\":[{\"height\":2160}]}'".into(),
                    ],
                },
            })
            .unwrap();

        match events.recv().await.unwrap() {
            RuntimeEvent::ProbeFinished { request_id, result } => {
                assert_eq!(request_id, 7);
                assert!(result.unwrap().supports_2160p);
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[tokio::test]
    async fn probe_timeout_returns_an_actionable_error() {
        let (events_tx, mut events_rx) = mpsc::unbounded_channel();
        run_probe_with_timeout(
            9,
            CommandSpec {
                program: PathBuf::from("/bin/sh"),
                args: vec!["-c".into(), "exec sleep 1".into()],
            },
            events_tx,
            Duration::from_millis(20),
        )
        .await;

        match events_rx.recv().await.unwrap() {
            RuntimeEvent::ProbeFinished { request_id, result } => {
                assert_eq!(request_id, 9);
                assert!(result.unwrap_err().contains("close the selected browser"));
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[tokio::test]
    async fn queue_emits_progress_before_completion() {
        let RuntimeHandle {
            commands,
            mut events,
        } = spawn();
        commands
            .send(RuntimeCommand::Enqueue {
                job_id: "job-1".into(),
                command: CommandSpec {
                    program: PathBuf::from("/bin/sh"),
                    args: vec![
                        "-c".into(),
                        format!(
                            "printf '%s\\n' '{}1|2|NA|3|4|downloading' '{}{}'",
                            crate::yt_dlp::PROGRESS_PREFIX,
                            crate::yt_dlp::OUTPUT_PREFIX,
                            "/tmp/video.mp4"
                        ),
                    ],
                },
            })
            .unwrap();

        let mut saw_progress = false;
        let mut saw_output = false;
        loop {
            match timeout(Duration::from_secs(2), events.recv())
                .await
                .unwrap()
                .unwrap()
            {
                RuntimeEvent::JobProgress { progress, .. } => {
                    saw_progress = progress.downloaded_bytes == Some(1);
                }
                RuntimeEvent::JobOutput { path, .. } => {
                    saw_output = path.as_path() == std::path::Path::new("/tmp/video.mp4");
                }
                RuntimeEvent::JobFinished { .. } => break,
                _ => {}
            }
        }
        assert!(saw_progress);
        assert!(saw_output);
    }

    #[tokio::test]
    async fn queue_starts_only_one_job_at_a_time() {
        let RuntimeHandle {
            commands,
            mut events,
        } = spawn();
        for (job_id, script) in [("job-1", "sleep 0.05"), ("job-2", "exit 0")] {
            commands
                .send(RuntimeCommand::Enqueue {
                    job_id: job_id.into(),
                    command: CommandSpec {
                        program: PathBuf::from("/bin/sh"),
                        args: vec!["-c".into(), script.into()],
                    },
                })
                .unwrap();
        }

        let mut sequence: Vec<String> = Vec::new();
        while sequence
            .iter()
            .filter(|event| event.starts_with("finished"))
            .count()
            < 2
        {
            match timeout(Duration::from_secs(2), events.recv())
                .await
                .unwrap()
                .unwrap()
            {
                RuntimeEvent::JobStarted { job_id } => sequence.push(format!("started:{job_id}")),
                RuntimeEvent::JobFinished { job_id } => sequence.push(format!("finished:{job_id}")),
                _ => {}
            }
        }

        let first_finished = sequence
            .iter()
            .position(|event| event == "finished:job-1")
            .unwrap();
        let second_started = sequence
            .iter()
            .position(|event| event == "started:job-2")
            .unwrap();
        assert!(first_finished < second_started, "event order: {sequence:?}");
    }
}
