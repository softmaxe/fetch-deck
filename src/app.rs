use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use url::Url;

use crate::{
    domain::{
        AppConfig, Authentication, Browser, DownloadJob, DownloadMode, HistoryEntry, JobProgress,
        JobStatus, MediaMetadata, Quality, SubtitleFormat,
    },
    platform::{BrowserProfile, discover_browser_profiles, open_in_finder},
    runtime::{self, RuntimeCommand, RuntimeEvent, RuntimeHandle},
    storage::{ConfigStore, HistoryStore},
    terminal::Tui,
    ui::{
        self, AddJobView, AddStage, DependencySummary, HistoryRow, JobDetails, Page, QueueRow,
        SettingField, UiModel,
    },
    yt_dlp::{
        YtDlpErrorKind, YtDlpPaths, build_download_command, build_probe_command, detect_binary,
    },
};

const MAX_LOG_LINES: usize = 300;

pub struct App {
    page: Page,
    config: AppConfig,
    config_store: ConfigStore,
    history_store: HistoryStore,
    history: Vec<HistoryEntry>,
    dependencies: Dependencies,
    runtime: RuntimeHandle,
    jobs: Vec<DownloadJob>,
    logs: HashMap<String, VecDeque<String>>,
    log_offsets: HashMap<String, u16>,
    selected_job: usize,
    selected_setting: usize,
    settings_values: Vec<String>,
    editing_setting: bool,
    add: AddForm,
    next_job_id: u64,
    next_probe_id: u64,
    status_message: Option<String>,
    cookie_notice_pending: bool,
    quit_armed: bool,
    shutting_down: bool,
    should_quit: bool,
}

#[derive(Clone)]
struct Dependencies {
    paths: YtDlpPaths,
    yt_dlp_ready: bool,
    ffmpeg_ready: bool,
    yt_dlp_summary: String,
    ffmpeg_summary: String,
}

struct AddForm {
    stage: AddStage,
    source_focus: usize,
    option_focus: usize,
    url: String,
    authentication_index: usize,
    profiles: Vec<BrowserProfile>,
    profile_index: usize,
    metadata: Option<MediaMetadata>,
    probe_request_id: Option<u64>,
    mode_index: usize,
    quality_index: usize,
    subtitle_index: usize,
    subtitle_format_index: usize,
    output: String,
    probe_error: Option<String>,
}

impl AddForm {
    fn new(output_directory: &Path) -> Self {
        Self {
            stage: AddStage::Source,
            source_focus: 0,
            option_focus: 0,
            url: String::new(),
            authentication_index: 0,
            profiles: Vec::new(),
            profile_index: 0,
            metadata: None,
            probe_request_id: None,
            mode_index: 0,
            quality_index: 0,
            subtitle_index: 0,
            subtitle_format_index: 1,
            output: output_directory.to_string_lossy().into_owned(),
            probe_error: None,
        }
    }

    fn selected_browser(&self) -> Option<Browser> {
        match self.authentication_index {
            1 => Some(Browser::Chrome),
            2 => Some(Browser::Firefox),
            3 => Some(Browser::Brave),
            _ => None,
        }
    }

    fn authentication(&self) -> Authentication {
        self.selected_browser()
            .map(|browser| Authentication::BrowserCookies {
                browser,
                profile: self
                    .profiles
                    .get(self.profile_index)
                    .map(|profile| profile.value.clone()),
            })
            .unwrap_or(Authentication::None)
    }

    fn authentication_label(&self) -> String {
        match self.selected_browser() {
            None => "None".to_owned(),
            Some(Browser::Chrome) => "Chrome cookies".to_owned(),
            Some(Browser::Firefox) => "Firefox cookies".to_owned(),
            Some(Browser::Brave) => "Brave cookies".to_owned(),
        }
    }

    fn profile_label(&self) -> String {
        if self.selected_browser().is_none() {
            "Not used".to_owned()
        } else {
            self.profiles
                .get(self.profile_index)
                .map(|profile| profile.label.clone())
                .unwrap_or_else(|| "Default profile".to_owned())
        }
    }

    fn refresh_profiles(&mut self) {
        self.profiles = self
            .selected_browser()
            .map(|browser| discover_browser_profiles(&browser))
            .unwrap_or_default();
        self.profile_index = 0;
    }

    fn qualities(&self) -> Vec<Quality> {
        self.metadata
            .as_ref()
            .map(|metadata| metadata.available_qualities.clone())
            .filter(|qualities| !qualities.is_empty())
            .unwrap_or_else(|| vec![Quality::Best])
    }

    fn selected_quality(&self) -> Quality {
        let qualities = self.qualities();
        qualities
            .get(self.quality_index.min(qualities.len().saturating_sub(1)))
            .copied()
            .unwrap_or(Quality::Best)
    }

    fn selected_mode(&self) -> Option<DownloadMode> {
        match self.mode_index {
            0 => Some(DownloadMode::Video {
                quality: self.selected_quality(),
            }),
            1 => Some(DownloadMode::Audio),
            2 => self
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.subtitles.get(self.subtitle_index))
                .map(|subtitle| DownloadMode::Subtitles {
                    language: subtitle.language.clone(),
                    format: if self.subtitle_format_index == 0 {
                        SubtitleFormat::Srt
                    } else {
                        SubtitleFormat::Vtt
                    },
                }),
            _ => None,
        }
    }

    fn mode_label(&self) -> &'static str {
        match self.mode_index {
            1 => "Audio / M4A",
            2 => "Subtitles",
            _ => "Video / MP4",
        }
    }

    fn quality_label(&self) -> String {
        if self.mode_index != 0 {
            return "Not used".to_owned();
        }
        quality_label(self.selected_quality()).to_owned()
    }

    fn subtitle_label(&self) -> String {
        if self.mode_index != 2 {
            return "Not used".to_owned();
        }
        let language = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.subtitles.get(self.subtitle_index))
            .map(|subtitle| subtitle.language.as_str())
            .unwrap_or("No manual subtitles");
        let format = if self.subtitle_format_index == 0 {
            "SRT"
        } else {
            "VTT"
        };
        format!("{language} / {format}")
    }

    fn output_focus(&self) -> usize {
        match self.mode_index {
            0 => 2,
            1 => 1,
            _ => 3,
        }
    }

    fn option_count(&self) -> usize {
        match self.mode_index {
            0 => 3,
            1 => 2,
            _ => 4,
        }
    }

    fn option_display_focus(&self) -> usize {
        match self.mode_index {
            0 if self.option_focus == 2 => 3,
            1 if self.option_focus == 1 => 3,
            _ => self.option_focus,
        }
    }
}

impl App {
    pub fn new() -> Result<Self> {
        let config_store = ConfigStore::for_default_location()
            .context("could not determine the config location")?;
        let history_store = HistoryStore::for_default_location()
            .context("could not determine the history location")?;
        let (config, config_error) = match config_store.load() {
            Ok(config) => (config, None),
            Err(error) => (
                AppConfig::default(),
                Some(format!("Config could not be loaded: {error}")),
            ),
        };
        let (history, history_error) = match history_store.load() {
            Ok(history) => (history, None),
            Err(error) => (
                Vec::new(),
                Some(format!("History could not be loaded: {error}")),
            ),
        };
        let dependencies = Dependencies::detect(&config);
        let settings_values = settings_values(&config);
        let output_directory = config.output_directory.clone();

        Ok(Self {
            page: Page::Queue,
            config,
            config_store,
            history_store,
            history,
            dependencies,
            runtime: runtime::spawn(),
            jobs: Vec::new(),
            logs: HashMap::new(),
            log_offsets: HashMap::new(),
            selected_job: 0,
            selected_setting: 0,
            settings_values,
            editing_setting: false,
            add: AddForm::new(&output_directory),
            next_job_id: 1,
            next_probe_id: 1,
            status_message: config_error.or(history_error),
            cookie_notice_pending: false,
            quit_armed: false,
            shutting_down: false,
            should_quit: false,
        })
    }

    pub async fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        while !self.should_quit {
            self.drain_runtime_events();
            let model = self.ui_model();
            terminal.draw(|frame| ui::draw(frame, &model))?;

            if event::poll(Duration::from_millis(80))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key(key),
                    Event::Paste(text) => self.handle_paste(&text),
                    Event::Mouse(mouse) => self.handle_mouse(mouse.kind),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.cookie_notice_pending {
            match key.code {
                KeyCode::Enter => {
                    self.cookie_notice_pending = false;
                    self.config.cookie_notice_acknowledged = true;
                    self.save_config();
                    self.status_message = Some("Browser cookie access enabled for this job".into());
                }
                KeyCode::Esc => {
                    self.cookie_notice_pending = false;
                    self.add.authentication_index = 0;
                    self.add.refresh_profiles();
                    self.status_message = Some("Browser cookie access disabled".into());
                }
                KeyCode::Left => {
                    self.cycle_source_choice(-1);
                    if self.add.authentication_index == 0 {
                        self.cookie_notice_pending = false;
                    }
                }
                KeyCode::Right => {
                    self.cycle_source_choice(1);
                    if self.add.authentication_index == 0 {
                        self.cookie_notice_pending = false;
                    }
                }
                _ => {}
            }
            return;
        }

        if key.code != KeyCode::Char('q') {
            self.quit_armed = false;
        }

        match self.page {
            Page::AddJob => self.handle_add_key(key),
            Page::Settings if self.editing_setting => self.handle_settings_edit_key(key),
            Page::Queue => self.handle_queue_key(key),
            Page::History => self.handle_history_key(key),
            Page::Settings => self.handle_settings_key(key),
            Page::Help => self.handle_help_key(key),
        }
    }

    fn handle_queue_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('a') => self.open_add_job(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next_job(),
            KeyCode::Up | KeyCode::Char('k') => self.select_previous_job(),
            KeyCode::PageDown => self.scroll_log(1),
            KeyCode::PageUp => self.scroll_log(-1),
            KeyCode::Char('c') => self.cancel_selected_job(),
            KeyCode::Char('r') => self.retry_selected_job(),
            KeyCode::Char('o') => self.open_selected_output(),
            _ => self.handle_global_key(key),
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('1') => self.page = Page::Queue,
            KeyCode::Char('x') => {
                if let Err(error) = self.history_store.clear() {
                    self.status_message = Some(format!("History could not be cleared: {error}"));
                } else {
                    self.history.clear();
                    self.status_message =
                        Some("History cleared; downloaded files were not changed".into());
                }
            }
            _ => self.handle_global_key(key),
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.selected_setting = (self.selected_setting + 1) % self.settings_values.len();
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.selected_setting = self
                    .selected_setting
                    .checked_sub(1)
                    .unwrap_or(self.settings_values.len() - 1);
            }
            KeyCode::Enter | KeyCode::Char('e') => self.editing_setting = true,
            KeyCode::Char('s') => self.save_settings(),
            KeyCode::Esc => self.page = Page::Queue,
            _ => self.handle_global_key(key),
        }
    }

    fn handle_settings_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.editing_setting = false;
                self.save_settings();
            }
            KeyCode::Esc => {
                self.editing_setting = false;
                self.settings_values = settings_values(&self.config);
                self.status_message = Some("Setting edit cancelled".into());
            }
            KeyCode::Backspace => {
                self.settings_values[self.selected_setting].pop();
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.settings_values[self.selected_setting].push(character);
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('1') => self.page = Page::Queue,
            _ => self.handle_global_key(key),
        }
    }

    fn handle_global_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('1') => self.page = Page::Queue,
            KeyCode::Char('2') => self.page = Page::History,
            KeyCode::Char('3') => {
                self.settings_values = settings_values(&self.config);
                self.page = Page::Settings;
            }
            KeyCode::Char('?') => self.page = Page::Help,
            KeyCode::Char('q') => self.request_quit(),
            _ => {}
        }
    }

    fn handle_add_key(&mut self, key: KeyEvent) {
        match self.add.stage {
            AddStage::Source => self.handle_source_key(key),
            AddStage::Probe => {
                if key.code == KeyCode::Esc {
                    self.add.probe_request_id = None;
                    self.add.stage = AddStage::Source;
                    self.status_message = Some("Probe result ignored".into());
                }
            }
            AddStage::Options => self.handle_options_key(key),
            AddStage::Review => match key.code {
                KeyCode::Enter => self.enqueue_add_form(),
                KeyCode::Esc => self.add.stage = AddStage::Options,
                _ => {}
            },
        }
    }

    fn handle_source_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.page = Page::Queue,
            KeyCode::Tab => self.add.source_focus = (self.add.source_focus + 1) % 3,
            KeyCode::BackTab => {
                self.add.source_focus = self.add.source_focus.checked_sub(1).unwrap_or(2)
            }
            KeyCode::Left => self.cycle_source_choice(-1),
            KeyCode::Right => self.cycle_source_choice(1),
            KeyCode::Enter => self.start_probe(),
            KeyCode::Backspace if self.add.source_focus == 0 => {
                self.add.url.pop();
            }
            KeyCode::Char('u')
                if self.add.source_focus == 0 && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.add.url.clear();
            }
            KeyCode::Char(character)
                if self.add.source_focus == 0
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.add.url.push(character);
            }
            _ => {}
        }
    }

    fn cycle_source_choice(&mut self, delta: isize) {
        match self.add.source_focus {
            1 => {
                self.add.authentication_index = cycle(self.add.authentication_index, 4, delta);
                self.add.refresh_profiles();
                if self.add.authentication_index > 0 && !self.config.cookie_notice_acknowledged {
                    self.cookie_notice_pending = true;
                    self.status_message =
                        Some("Cookies stay local. Enter enables access; Esc disables it".into());
                }
            }
            2 if !self.add.profiles.is_empty() => {
                self.add.profile_index =
                    cycle(self.add.profile_index, self.add.profiles.len(), delta)
            }
            _ => {}
        }
    }

    fn start_probe(&mut self) {
        if !self.dependencies.yt_dlp_ready {
            self.status_message = Some("yt-dlp is missing; set its path in Settings".into());
            return;
        }
        if let Err(error) = validate_video_url(&self.add.url) {
            self.status_message = Some(error);
            return;
        }
        let request_id = self.next_probe_id;
        self.next_probe_id += 1;
        let command = build_probe_command(
            &self.dependencies.paths,
            &self.add.url,
            &self.add.authentication(),
        );
        if self
            .runtime
            .commands
            .send(RuntimeCommand::Probe {
                request_id,
                command,
            })
            .is_err()
        {
            self.status_message = Some("Download runtime is unavailable".into());
            return;
        }
        self.add.probe_request_id = Some(request_id);
        self.add.probe_error = None;
        self.add.stage = AddStage::Probe;
        self.status_message = Some("Reading title, formats, and manual subtitles".into());
    }

    fn handle_options_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.add.stage = AddStage::Source,
            KeyCode::Tab => {
                self.add.option_focus = (self.add.option_focus + 1) % self.add.option_count()
            }
            KeyCode::BackTab => {
                self.add.option_focus = self
                    .add
                    .option_focus
                    .checked_sub(1)
                    .unwrap_or(self.add.option_count() - 1)
            }
            KeyCode::Left => self.cycle_option(-1),
            KeyCode::Right => self.cycle_option(1),
            KeyCode::Enter => {
                if self.add.selected_mode().is_none() {
                    self.status_message = Some("No manual subtitle track is available".into());
                } else if self.add.output.trim().is_empty() {
                    self.status_message = Some("Choose an output directory".into());
                } else {
                    self.add.stage = AddStage::Review;
                    self.status_message =
                        Some("Review the job, then press Enter to queue it".into());
                }
            }
            KeyCode::Backspace if self.add.option_focus == self.add.output_focus() => {
                self.add.output.pop();
            }
            KeyCode::Char(character)
                if self.add.option_focus == self.add.output_focus()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.add.output.push(character);
            }
            _ => {}
        }
    }

    fn cycle_option(&mut self, delta: isize) {
        match (self.add.mode_index, self.add.option_focus) {
            (_, 0) => {
                self.add.mode_index = cycle(self.add.mode_index, 3, delta);
                self.add.option_focus = 0;
            }
            (0, 1) => {
                let count = self.add.qualities().len();
                self.add.quality_index = cycle(self.add.quality_index, count, delta);
            }
            (2, 1) => {
                let count = self
                    .add
                    .metadata
                    .as_ref()
                    .map(|metadata| metadata.subtitles.len())
                    .unwrap_or(0);
                if count > 0 {
                    self.add.subtitle_index = cycle(self.add.subtitle_index, count, delta);
                }
            }
            (2, 2) => {
                self.add.subtitle_format_index = cycle(self.add.subtitle_format_index, 2, delta)
            }
            _ => {}
        }
    }

    fn enqueue_add_form(&mut self) {
        let Some(mode) = self.add.selected_mode() else {
            self.status_message = Some("The selected download mode is unavailable".into());
            return;
        };
        if matches!(mode, DownloadMode::Video { .. } | DownloadMode::Audio)
            && !self.dependencies.ffmpeg_ready
        {
            self.status_message = Some("ffmpeg is required for MP4 and M4A output".into());
            return;
        }

        let output_directory = expand_user_path(self.add.output.trim());
        let job_id = format!("job-{}", self.next_job_id);
        self.next_job_id += 1;
        let authentication = self.add.authentication();
        let command = build_download_command(
            &self.dependencies.paths,
            &self.add.url,
            &output_directory,
            &mode,
            &authentication,
        );
        let job = DownloadJob {
            id: job_id.clone(),
            url: self.add.url.clone(),
            mode,
            authentication,
            output_directory: output_directory.clone(),
            status: JobStatus::Queued,
            progress: JobProgress::default(),
            metadata: self.add.metadata.clone(),
            output_path: None,
            error: None,
        };
        self.jobs.push(job);
        self.logs.insert(job_id.clone(), VecDeque::new());
        self.selected_job = self.jobs.len().saturating_sub(1);
        if self
            .runtime
            .commands
            .send(RuntimeCommand::Enqueue {
                job_id: job_id.clone(),
                command,
            })
            .is_err()
            && let Some(job) = self.jobs.last_mut()
        {
            job.status = JobStatus::Failed;
            job.error = Some("Download runtime is unavailable".into());
        }

        self.config.output_directory = output_directory;
        self.save_config();
        self.add = AddForm::new(&self.config.output_directory);
        self.page = Page::Queue;
        self.status_message = Some("Job added to the queue".into());
    }

    fn handle_paste(&mut self, text: &str) {
        if self.page == Page::AddJob
            && self.add.stage == AddStage::Source
            && self.add.source_focus == 0
        {
            self.add.url.push_str(text.trim());
        } else if self.page == Page::AddJob
            && self.add.stage == AddStage::Options
            && self.add.option_focus == self.add.output_focus()
        {
            self.add.output.push_str(text.trim());
        } else if self.page == Page::Settings && self.editing_setting {
            self.settings_values[self.selected_setting].push_str(text.trim());
        }
    }

    fn handle_mouse(&mut self, kind: MouseEventKind) {
        match kind {
            MouseEventKind::ScrollDown if self.page == Page::Queue => self.select_next_job(),
            MouseEventKind::ScrollUp if self.page == Page::Queue => self.select_previous_job(),
            _ => {}
        }
    }

    fn open_add_job(&mut self) {
        self.add = AddForm::new(&self.config.output_directory);
        self.page = Page::AddJob;
        self.status_message = Some("Paste a video URL; Tab moves between fields".into());
    }

    fn select_next_job(&mut self) {
        if !self.jobs.is_empty() {
            self.selected_job = (self.selected_job + 1).min(self.jobs.len() - 1);
        }
    }

    fn select_previous_job(&mut self) {
        self.selected_job = self.selected_job.saturating_sub(1);
    }

    fn scroll_log(&mut self, delta: isize) {
        let Some(job) = self.jobs.get(self.selected_job) else {
            return;
        };
        let offset = self.log_offsets.entry(job.id.clone()).or_default();
        *offset = if delta > 0 {
            offset.saturating_add(delta as u16)
        } else {
            offset.saturating_sub(delta.unsigned_abs() as u16)
        };
    }

    fn cancel_selected_job(&mut self) {
        let Some(job) = self.jobs.get(self.selected_job) else {
            return;
        };
        if matches!(
            job.status,
            JobStatus::Queued | JobStatus::Downloading | JobStatus::Merging
        ) {
            let _ = self.runtime.commands.send(RuntimeCommand::Cancel {
                job_id: job.id.clone(),
            });
            self.status_message = Some("Cancelling the selected job".into());
        }
    }

    fn retry_selected_job(&mut self) {
        let Some(job) = self.jobs.get_mut(self.selected_job) else {
            return;
        };
        if !matches!(job.status, JobStatus::Failed | JobStatus::Cancelled) {
            self.status_message = Some("Only failed or cancelled jobs can be retried".into());
            return;
        }
        let command = build_download_command(
            &self.dependencies.paths,
            &job.url,
            &job.output_directory,
            &job.mode,
            &job.authentication,
        );
        job.status = JobStatus::Queued;
        job.progress = JobProgress::default();
        job.error = None;
        job.output_path = None;
        push_log(&mut self.logs, &job.id, "Retry queued".into());
        let _ = self.runtime.commands.send(RuntimeCommand::Enqueue {
            job_id: job.id.clone(),
            command,
        });
        self.status_message = Some("Retry added to the queue".into());
    }

    fn open_selected_output(&mut self) {
        let Some(job) = self.jobs.get(self.selected_job) else {
            return;
        };
        let path = job.output_path.as_deref().unwrap_or(&job.output_directory);
        match open_in_finder(path) {
            Ok(()) => self.status_message = Some("Opened output in Finder".into()),
            Err(error) => self.status_message = Some(format!("Could not open output: {error}")),
        }
    }

    fn request_quit(&mut self) {
        if self.shutting_down {
            return;
        }
        let has_pending = self.jobs.iter().any(|job| {
            matches!(
                job.status,
                JobStatus::Queued | JobStatus::Downloading | JobStatus::Merging
            )
        });
        if has_pending && !self.quit_armed {
            self.quit_armed = true;
            self.status_message = Some(
                "Downloads are active. Press q again to cancel them and quit; any other action keeps running"
                    .into(),
            );
            return;
        }
        self.shutting_down = true;
        self.status_message = Some("Stopping downloads before exit".into());
        let _ = self.runtime.commands.send(RuntimeCommand::Shutdown);
    }

    fn save_settings(&mut self) {
        let output = self.settings_values[0].trim();
        if output.is_empty() {
            self.status_message = Some("Output directory cannot be empty".into());
            return;
        }
        self.config.output_directory = expand_user_path(output);
        self.config.yt_dlp_path = optional_path(&self.settings_values[1]);
        self.config.ffmpeg_path = optional_path(&self.settings_values[2]);
        self.dependencies = Dependencies::detect(&self.config);
        self.save_config();
        self.settings_values = settings_values(&self.config);
        self.status_message = Some("Settings saved".into());
    }

    fn save_config(&mut self) {
        if let Err(error) = self.config_store.save(&self.config) {
            self.status_message = Some(format!("Config could not be saved: {error}"));
        }
    }

    fn drain_runtime_events(&mut self) {
        while let Ok(event) = self.runtime.events.try_recv() {
            self.handle_runtime_event(event);
        }
    }

    fn handle_runtime_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::ProbeFinished { request_id, result }
                if self.add.probe_request_id == Some(request_id) =>
            {
                self.add.probe_request_id = None;
                match result {
                    Ok(metadata) => {
                        let quality_count = metadata.available_qualities.len();
                        self.add.metadata = Some(metadata);
                        self.add.stage = AddStage::Options;
                        self.add.quality_index = 0;
                        self.status_message = Some(format!(
                            "Probe complete; {quality_count} video quality choices available"
                        ));
                    }
                    Err(error) => {
                        self.add.probe_error = Some(error.clone());
                        self.add.stage = AddStage::Source;
                        self.status_message = Some(format!("Probe failed: {error}"));
                    }
                }
            }
            RuntimeEvent::ProbeFinished { .. } => {}
            RuntimeEvent::JobStarted { job_id } => {
                if let Some(job) = self.job_mut(&job_id) {
                    job.status = JobStatus::Downloading;
                }
                push_log(&mut self.logs, &job_id, "Download started".into());
            }
            RuntimeEvent::JobProgress { job_id, progress } => {
                if let Some(job) = self.job_mut(&job_id) {
                    if progress.status.as_deref() == Some("finished") {
                        job.status = JobStatus::Merging;
                    }
                    job.progress = progress;
                }
            }
            RuntimeEvent::JobLog { job_id, line } => {
                let sanitized = self.sanitize_log(&job_id, &line);
                if indicates_post_processing(&line)
                    && let Some(job) = self.job_mut(&job_id)
                {
                    job.status = JobStatus::Merging;
                }
                push_log(&mut self.logs, &job_id, sanitized);
            }
            RuntimeEvent::JobOutput { job_id, path } => {
                if let Some(job) = self.job_mut(&job_id) {
                    job.output_path = Some(path);
                }
            }
            RuntimeEvent::JobFinished { job_id } => {
                let completed = if let Some(job) = self.job_mut(&job_id) {
                    job.status = JobStatus::Completed;
                    job.progress.status = Some("finished".into());
                    Some(job.clone())
                } else {
                    None
                };
                push_log(&mut self.logs, &job_id, "Download completed".into());
                if let Some(job) = completed {
                    self.record_history(&job);
                }
                self.status_message = Some("Download completed".into());
            }
            RuntimeEvent::JobFailed {
                job_id,
                kind,
                message,
            } => {
                let guidance = error_guidance(kind);
                let combined = format!("{message}. {guidance}");
                let failed = if let Some(job) = self.job_mut(&job_id) {
                    job.status = JobStatus::Failed;
                    job.error = Some(combined.clone());
                    Some(job.clone())
                } else {
                    None
                };
                push_log(&mut self.logs, &job_id, combined.clone());
                if let Some(job) = failed {
                    self.record_history(&job);
                }
                self.status_message = Some(combined);
            }
            RuntimeEvent::JobCancelled { job_id } => {
                let cancelled = if let Some(job) = self.job_mut(&job_id) {
                    job.status = JobStatus::Cancelled;
                    Some(job.clone())
                } else {
                    None
                };
                push_log(
                    &mut self.logs,
                    &job_id,
                    "Cancelled; partial files were kept for retry".into(),
                );
                if let Some(job) = cancelled {
                    self.record_history(&job);
                }
                self.status_message = Some("Job cancelled".into());
            }
            RuntimeEvent::Stopped => self.should_quit = true,
        }
    }

    fn job_mut(&mut self, job_id: &str) -> Option<&mut DownloadJob> {
        self.jobs.iter_mut().find(|job| job.id == job_id)
    }

    fn sanitize_log(&self, job_id: &str, line: &str) -> String {
        let mut sanitized = directories::UserDirs::new()
            .map(|directories| {
                line.replace(&directories.home_dir().to_string_lossy().into_owned(), "~")
            })
            .unwrap_or_else(|| line.to_owned());
        if let Some(Authentication::BrowserCookies {
            profile: Some(profile),
            ..
        }) = self
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .map(|job| &job.authentication)
        {
            sanitized = sanitized.replace(profile, "<profile>");
        }
        sanitized
    }

    fn record_history(&mut self, job: &DownloadJob) {
        let entry = HistoryEntry {
            url: job.url.clone(),
            title: job
                .metadata
                .as_ref()
                .map(|metadata| metadata.title.clone())
                .unwrap_or_else(|| job.url.clone()),
            status: job.status,
            output_path: job.output_path.clone(),
            timestamp_unix_seconds: unix_time(),
        };
        self.history.push(entry.clone());
        if self.history.len() > 100 {
            self.history.drain(..self.history.len() - 100);
        }
        if let Err(error) = self.history_store.append(entry) {
            self.status_message = Some(format!("History could not be saved: {error}"));
        }
    }

    fn ui_model(&self) -> UiModel {
        let selected_job = self.jobs.get(self.selected_job).map(|job| {
            let progress = progress_percent(job);
            JobDetails {
                title: job_title(job),
                source: job.url.clone(),
                format: mode_label(&job.mode),
                output: job
                    .output_path
                    .as_ref()
                    .unwrap_or(&job.output_directory)
                    .to_string_lossy()
                    .into_owned(),
                status: status_label(job.status).to_owned(),
                progress_percent: progress,
                downloaded: format_bytes(job.progress.downloaded_bytes),
                total: format_bytes(
                    job.progress
                        .total_bytes
                        .or(job.progress.estimated_total_bytes),
                ),
                speed: job
                    .progress
                    .speed_bytes_per_second
                    .map(|speed| format!("{}/s", format_bytes(Some(speed as u64))))
                    .unwrap_or_else(|| "--".into()),
                eta: job
                    .progress
                    .eta_seconds
                    .map(format_duration)
                    .unwrap_or_else(|| "--".into()),
                transport_stage: transport_stage(job.status),
                log_lines: self
                    .logs
                    .get(&job.id)
                    .map(|lines| lines.iter().cloned().collect())
                    .unwrap_or_default(),
                log_offset: *self.log_offsets.get(&job.id).unwrap_or(&0),
            }
        });

        UiModel {
            active_page: self.page,
            dependencies: DependencySummary {
                yt_dlp: self.dependencies.yt_dlp_summary.clone(),
                ffmpeg: self.dependencies.ffmpeg_summary.clone(),
            },
            queue_rows: self
                .jobs
                .iter()
                .map(|job| QueueRow {
                    title: job_title(job),
                    status: status_label(job.status).to_owned(),
                    progress_percent: progress_percent(job),
                    speed: job
                        .progress
                        .speed_bytes_per_second
                        .map(|speed| format!("{}/s", format_bytes(Some(speed as u64))))
                        .unwrap_or_default(),
                    eta: job
                        .progress
                        .eta_seconds
                        .map(format_duration)
                        .unwrap_or_default(),
                })
                .collect(),
            selected_queue: self.selected_job,
            selected_job,
            add_job: self.add_view(),
            history_rows: self
                .history
                .iter()
                .rev()
                .map(|entry| HistoryRow {
                    title: entry.title.clone(),
                    result: status_label(entry.status).to_owned(),
                    finished_at: relative_time(entry.timestamp_unix_seconds),
                    output: entry
                        .output_path
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "--".into()),
                })
                .collect(),
            settings_fields: vec![
                SettingField {
                    name: "Output directory".into(),
                    value: self.settings_values[0].clone(),
                    hint: "Default folder for new jobs".into(),
                },
                SettingField {
                    name: "yt-dlp path".into(),
                    value: self.settings_values[1].clone(),
                    hint: self.dependencies.yt_dlp_summary.clone(),
                },
                SettingField {
                    name: "ffmpeg path".into(),
                    value: self.settings_values[2].clone(),
                    hint: self.dependencies.ffmpeg_summary.clone(),
                },
            ],
            selected_setting: self.selected_setting,
            settings_editing: self.editing_setting,
            status_message: self.status_message.clone(),
        }
    }

    fn add_view(&self) -> AddJobView {
        let metadata = self.add.metadata.as_ref();
        let probe_summary = if self.add.stage == AddStage::Probe {
            vec![
                "Reading source metadata...".into(),
                "Esc ignores this result".into(),
            ]
        } else {
            self.add
                .probe_error
                .as_ref()
                .map(|error| vec![error.clone()])
                .unwrap_or_default()
        };
        let mut review_lines = Vec::new();
        if let Some(metadata) = metadata {
            review_lines.push(format!("Title     {}", metadata.title));
            review_lines.push(format!(
                "Duration  {}",
                metadata
                    .duration_seconds
                    .map(format_duration)
                    .unwrap_or_else(|| "Unknown".into())
            ));
            if metadata.supports_2160p {
                review_lines.push("4K        Available".into());
            }
        }
        AddJobView {
            stage: self.add.stage,
            focused_field: match self.add.stage {
                AddStage::Source => self.add.source_focus,
                AddStage::Options => self.add.option_display_focus(),
                _ => 0,
            },
            source: self.add.url.clone(),
            authentication: self.add.authentication_label(),
            profile: self.add.profile_label(),
            probe_summary,
            mode: self.add.mode_label().into(),
            quality: self.add.quality_label(),
            subtitle: self.add.subtitle_label(),
            output: self.add.output.clone(),
            review_lines,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if !self.shutting_down {
            let _ = self.runtime.commands.send(RuntimeCommand::Shutdown);
        }
    }
}

impl Dependencies {
    fn detect(config: &AppConfig) -> Self {
        let yt_dlp = configured_binary(config.yt_dlp_path.as_deref(), "yt-dlp");
        let ffmpeg = configured_binary(config.ffmpeg_path.as_deref(), "ffmpeg");
        let yt_dlp_ready = yt_dlp.is_some();
        let ffmpeg_ready = ffmpeg.is_some();
        let yt_dlp_summary = binary_summary(yt_dlp.as_deref(), "--version", "missing");
        let ffmpeg_summary = binary_summary(ffmpeg.as_deref(), "-version", "missing");
        Self {
            paths: YtDlpPaths {
                yt_dlp: yt_dlp.unwrap_or_else(|| {
                    config
                        .yt_dlp_path
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("yt-dlp"))
                }),
                ffmpeg,
            },
            yt_dlp_ready,
            ffmpeg_ready,
            yt_dlp_summary,
            ffmpeg_summary,
        }
    }
}

fn configured_binary(configured: Option<&Path>, fallback: &str) -> Option<PathBuf> {
    configured
        .map(detect_binary)
        .unwrap_or_else(|| detect_binary(fallback))
}

fn binary_summary(path: Option<&Path>, version_flag: &str, missing: &str) -> String {
    let Some(path) = path else {
        return missing.to_owned();
    };
    let version = std::process::Command::new(path)
        .arg(version_flag)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| output.lines().next().map(str::to_owned))
        .unwrap_or_else(|| "version unknown".into());
    if version.chars().count() > 24 {
        format!("{}...", version.chars().take(21).collect::<String>())
    } else {
        version
    }
}

fn settings_values(config: &AppConfig) -> Vec<String> {
    vec![
        config.output_directory.to_string_lossy().into_owned(),
        config
            .yt_dlp_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
        config
            .ffmpeg_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
    ]
}

fn optional_path(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    (!value.is_empty()).then(|| expand_user_path(value))
}

fn expand_user_path(value: &str) -> PathBuf {
    if value == "~" {
        return directories::UserDirs::new()
            .map(|directories| directories.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(relative) = value.strip_prefix("~/") {
        return directories::UserDirs::new()
            .map(|directories| directories.home_dir().join(relative))
            .unwrap_or_else(|| PathBuf::from(value));
    }
    PathBuf::from(value)
}

fn validate_video_url(value: &str) -> Result<(), String> {
    let url = Url::parse(value.trim()).map_err(|_| "Enter a valid http or https URL".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Enter a valid http or https URL".into());
    }
    let path = url.path().to_ascii_lowercase();
    let looks_like_collection = url.query_pairs().any(|(key, _)| key == "list")
        || path.contains("/playlist")
        || path.contains("/channel");
    if looks_like_collection {
        return Err("Playlist and channel downloads are not supported yet".into());
    }
    Ok(())
}

fn cycle(current: usize, count: usize, delta: isize) -> usize {
    if count == 0 {
        return 0;
    }
    if delta >= 0 {
        (current + delta as usize) % count
    } else {
        (current + count - (delta.unsigned_abs() % count)) % count
    }
}

fn push_log(logs: &mut HashMap<String, VecDeque<String>>, job_id: &str, line: String) {
    let lines = logs.entry(job_id.to_owned()).or_default();
    if lines.len() == MAX_LOG_LINES {
        lines.pop_front();
    }
    lines.push_back(line);
}

fn indicates_post_processing(line: &str) -> bool {
    [
        "[Merger]",
        "[ExtractAudio]",
        "[VideoRemuxer]",
        "[SubtitleConvertor]",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

fn error_guidance(kind: YtDlpErrorKind) -> &'static str {
    match kind {
        YtDlpErrorKind::Authentication => {
            "Close the selected browser, confirm its profile, then retry"
        }
        YtDlpErrorKind::GeoRestricted => "The source is not available in this region",
        YtDlpErrorKind::VideoUnavailable => "The source is private, removed, or unavailable",
        YtDlpErrorKind::Network => "Check the network connection, then retry",
        YtDlpErrorKind::MissingFfmpeg => "Set a valid ffmpeg path in Settings",
        YtDlpErrorKind::PermissionDenied => "Choose a writable output directory",
        YtDlpErrorKind::InvalidUrl => "Check the URL and supported site",
        YtDlpErrorKind::Unknown => "Open the job log for the original yt-dlp output",
    }
}

fn quality_label(quality: Quality) -> &'static str {
    match quality {
        Quality::Best => "Best available",
        Quality::P2160 => "4K",
        Quality::P1080 => "1080p",
        Quality::P720 => "720p",
        Quality::P480 => "480p",
    }
}

fn mode_label(mode: &DownloadMode) -> String {
    match mode {
        DownloadMode::Video { quality } => format!("Video / MP4 / {}", quality_label(*quality)),
        DownloadMode::Audio => "Audio / M4A".into(),
        DownloadMode::Subtitles { language, format } => {
            format!("Subtitles / {language} / {format:?}")
        }
    }
}

fn status_label(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "Queued",
        JobStatus::Probing => "Probing",
        JobStatus::Downloading => "Downloading",
        JobStatus::Merging => "Merging",
        JobStatus::Completed => "Completed",
        JobStatus::Failed => "Failed",
        JobStatus::Cancelled => "Cancelled",
    }
}

fn transport_stage(status: JobStatus) -> usize {
    match status {
        JobStatus::Probing => 1,
        JobStatus::Queued => 2,
        JobStatus::Downloading => 3,
        JobStatus::Merging => 4,
        JobStatus::Completed => 5,
        JobStatus::Failed | JobStatus::Cancelled => 3,
    }
}

fn progress_percent(job: &DownloadJob) -> u16 {
    if job.status == JobStatus::Completed {
        return 100;
    }
    let total = job
        .progress
        .total_bytes
        .or(job.progress.estimated_total_bytes);
    match (job.progress.downloaded_bytes, total) {
        (Some(downloaded), Some(total)) if total > 0 => {
            ((downloaded.saturating_mul(100) / total).min(100)) as u16
        }
        _ => 0,
    }
}

fn job_title(job: &DownloadJob) -> String {
    job.metadata
        .as_ref()
        .map(|metadata| metadata.title.clone())
        .unwrap_or_else(|| job.url.clone())
}

fn format_bytes(value: Option<u64>) -> String {
    let Some(bytes) = value else {
        return "--".into();
    };
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut amount = bytes as f64;
    let mut unit = 0;
    while amount >= 1024.0 && unit < units.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", units[unit])
    } else {
        format!("{amount:.1} {}", units[unit])
    }
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn relative_time(timestamp: i64) -> String {
    let age = unix_time().saturating_sub(timestamp);
    match age {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", age / 60),
        3600..=86_399 => format!("{}h ago", age / 3600),
        _ => format!("{}d ago", age / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cookie_notice_allows_browser_selection_to_keep_cycling() {
        let mut app = App::new().unwrap();
        app.open_add_job();
        app.add.source_focus = 1;
        app.config.cookie_notice_acknowledged = false;

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.add.selected_browser(), Some(Browser::Chrome));
        assert!(app.cookie_notice_pending);

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.add.selected_browser(), Some(Browser::Firefox));
        assert!(app.cookie_notice_pending);

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.add.selected_browser(), Some(Browser::Chrome));
        assert!(app.cookie_notice_pending);

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.add.selected_browser(), None);
        assert!(!app.cookie_notice_pending);
    }

    #[test]
    fn rejects_common_collection_urls() {
        assert!(validate_video_url("https://youtube.com/watch?v=1&list=abc").is_err());
        assert!(validate_video_url("https://example.com/channel/demo").is_err());
        assert!(validate_video_url("https://example.com/video/1").is_ok());
    }

    #[test]
    fn four_k_label_is_only_available_from_probe_data() {
        let mut form = AddForm::new(Path::new("downloads"));
        assert!(!form.qualities().contains(&Quality::P2160));
        form.metadata = Some(MediaMetadata {
            id: "id".into(),
            title: "Title".into(),
            duration_seconds: None,
            thumbnail_url: None,
            subtitles: Vec::new(),
            available_qualities: vec![Quality::Best, Quality::P2160],
            supports_2160p: true,
        });
        assert!(form.qualities().contains(&Quality::P2160));
    }

    #[test]
    fn byte_and_duration_formatters_are_stable() {
        assert_eq!(format_bytes(Some(1_048_576)), "1.0 MiB");
        assert_eq!(format_duration(65), "01:05");
    }
}
