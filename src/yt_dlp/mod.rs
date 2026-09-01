use crate::domain::{
    Authentication, DownloadMode, JobProgress, MediaMetadata, Quality, SubtitleFormat,
    SubtitleTrack,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const PROGRESS_PREFIX: &str = "FETCHDECK_PROGRESS:";
pub const OUTPUT_PREFIX: &str = "FETCHDECK_OUTPUT:";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YtDlpPaths {
    pub yt_dlp: PathBuf,
    pub ffmpeg: Option<PathBuf>,
}

impl Default for YtDlpPaths {
    fn default() -> Self {
        Self {
            yt_dlp: PathBuf::from("yt-dlp"),
            ffmpeg: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ProbeParseError {
    #[error("invalid yt-dlp JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("probe response is missing {0}")]
    MissingField(&'static str),
    #[error("playlist responses are not supported")]
    Playlist,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YtDlpErrorKind {
    Authentication,
    GeoRestricted,
    VideoUnavailable,
    Network,
    MissingFfmpeg,
    PermissionDenied,
    InvalidUrl,
    Unknown,
}

pub fn build_probe_command(
    paths: &YtDlpPaths,
    url: &str,
    authentication: &Authentication,
    cookie_jar: Option<&Path>,
    import_browser_cookies: bool,
) -> CommandSpec {
    let mut args = vec![
        "--dump-single-json".to_owned(),
        "--skip-download".to_owned(),
        "--no-playlist".to_owned(),
    ];
    if import_browser_cookies {
        add_browser_authentication(&mut args, authentication);
    }
    add_cookie_jar(&mut args, cookie_jar);
    args.push("--".to_owned());
    args.push(url.to_owned());
    CommandSpec {
        program: paths.yt_dlp.clone(),
        args,
    }
}

pub fn build_download_command(
    paths: &YtDlpPaths,
    url: &str,
    output_directory: &Path,
    mode: &DownloadMode,
    cookie_jar: Option<&Path>,
) -> CommandSpec {
    let output_template = output_directory
        .join("%(title)s [%(id)s].%(ext)s")
        .to_string_lossy()
        .into_owned();
    let mut args = vec![
        "--no-playlist".to_owned(),
        "--no-overwrites".to_owned(),
        "--continue".to_owned(),
        "--part".to_owned(),
        "--newline".to_owned(),
        "--progress".to_owned(),
        "--progress-template".to_owned(),
        format!(
            "download:{PROGRESS_PREFIX}%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(progress.status)s"
        ),
        "--output".to_owned(),
        output_template,
        "--print".to_owned(),
        format!("after_move:{OUTPUT_PREFIX}%(filepath)s"),
    ];
    if let Some(ffmpeg) = &paths.ffmpeg {
        let location = ffmpeg
            .parent()
            .unwrap_or(ffmpeg)
            .to_string_lossy()
            .into_owned();
        args.extend(["--ffmpeg-location".to_owned(), location]);
    }
    match mode {
        DownloadMode::Video { quality } => {
            args.extend([
                "--format".to_owned(),
                video_format_selector(*quality),
                "--merge-output-format".to_owned(),
                "mp4".to_owned(),
                "--remux-video".to_owned(),
                "mp4".to_owned(),
            ]);
        }
        DownloadMode::Audio => {
            args.extend([
                "--extract-audio".to_owned(),
                "--audio-format".to_owned(),
                "m4a".to_owned(),
            ]);
        }
        DownloadMode::Subtitles { language, format } => {
            let preferred_formats = match format {
                SubtitleFormat::Srt => "srt/vtt/best",
                SubtitleFormat::Vtt => "vtt/best",
            };
            args.extend([
                "--skip-download".to_owned(),
                "--write-subs".to_owned(),
                "--sub-langs".to_owned(),
                language.clone(),
                "--sub-format".to_owned(),
                preferred_formats.to_owned(),
                "--convert-subs".to_owned(),
                format.as_yt_dlp_name().to_owned(),
            ]);
        }
    }
    add_cookie_jar(&mut args, cookie_jar);
    args.push("--".to_owned());
    args.push(url.to_owned());
    CommandSpec {
        program: paths.yt_dlp.clone(),
        args,
    }
}

fn add_browser_authentication(args: &mut Vec<String>, authentication: &Authentication) {
    if let Some(source) = authentication.browser_cookie_source() {
        args.extend(["--cookies-from-browser".to_owned(), source]);
    }
}

fn add_cookie_jar(args: &mut Vec<String>, cookie_jar: Option<&Path>) {
    if let Some(cookie_jar) = cookie_jar {
        args.extend([
            "--cookies".to_owned(),
            cookie_jar.to_string_lossy().into_owned(),
        ]);
    }
}

fn video_format_selector(quality: Quality) -> String {
    match quality.height() {
        None => "bestvideo+bestaudio/best".to_owned(),
        Some(height) => {
            format!("bestvideo[height<={height}]+bestaudio/best[height<={height}]")
        }
    }
}

pub fn parse_probe_json(input: &str) -> Result<MediaMetadata, ProbeParseError> {
    let value: Value = serde_json::from_str(input)?;
    if value.get("_type").and_then(Value::as_str) == Some("playlist")
        || value.get("entries").is_some()
    {
        return Err(ProbeParseError::Playlist);
    }
    let id = string_field(&value, "id")?;
    let title = string_field(&value, "title")?;
    let heights: Vec<u64> = value
        .get("formats")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|format| format.get("height").and_then(Value::as_u64))
        .collect();
    let supports_2160p = heights.iter().any(|height| *height >= 2160);
    let mut available_qualities = vec![Quality::Best];
    for (quality, minimum, maximum) in [
        (Quality::P2160, 2160, u64::MAX),
        (Quality::P1080, 1080, 2159),
        (Quality::P720, 720, 1079),
        (Quality::P480, 480, 719),
    ] {
        if heights
            .iter()
            .any(|height| (*height >= minimum) && (*height <= maximum))
        {
            available_qualities.push(quality);
        }
    }
    Ok(MediaMetadata {
        id,
        title,
        duration_seconds: value
            .get("duration")
            .and_then(Value::as_f64)
            .map(|seconds| seconds.round() as u64),
        thumbnail_url: value
            .get("thumbnail")
            .and_then(Value::as_str)
            .map(str::to_owned),
        subtitles: parse_subtitles(&value),
        available_qualities,
        supports_2160p,
    })
}

fn string_field(value: &Value, field: &'static str) -> Result<String, ProbeParseError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(ProbeParseError::MissingField(field))
}

fn parse_subtitles(value: &Value) -> Vec<SubtitleTrack> {
    let mut tracks: BTreeMap<String, SubtitleTrack> = BTreeMap::new();
    if let Some(languages) = value.get("subtitles").and_then(Value::as_object) {
        for (language, entries) in languages {
            let track = tracks
                .entry(language.clone())
                .or_insert_with(|| SubtitleTrack {
                    language: language.clone(),
                    name: entries
                        .as_array()
                        .and_then(|items| items.first())
                        .and_then(|item| item.get("name"))
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    formats: Vec::new(),
                });
            for entry in entries.as_array().into_iter().flatten() {
                let format = match entry.get("ext").and_then(Value::as_str) {
                    Some("srt") => Some(SubtitleFormat::Srt),
                    Some("vtt") => Some(SubtitleFormat::Vtt),
                    _ => None,
                };
                if let Some(format) = format.filter(|format| !track.formats.contains(format)) {
                    track.formats.push(format);
                }
            }
        }
    }
    tracks.into_values().collect()
}

pub fn parse_progress_line(line: &str) -> Option<JobProgress> {
    let payload = line.strip_prefix(PROGRESS_PREFIX)?;
    let mut fields = payload.splitn(6, '|');
    Some(JobProgress {
        downloaded_bytes: parse_optional(fields.next()?),
        total_bytes: parse_optional(fields.next()?),
        estimated_total_bytes: parse_optional(fields.next()?),
        speed_bytes_per_second: parse_optional(fields.next()?),
        eta_seconds: parse_optional(fields.next()?),
        status: optional_text(fields.next()?),
    })
}

pub fn parse_output_line(line: &str) -> Option<PathBuf> {
    line.strip_prefix(OUTPUT_PREFIX)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

fn parse_optional<T: std::str::FromStr>(value: &str) -> Option<T> {
    if value.is_empty() || value.eq_ignore_ascii_case("NA") || value.eq_ignore_ascii_case("N/A") {
        None
    } else {
        value.parse().ok()
    }
}

fn optional_text(value: &str) -> Option<String> {
    if value.is_empty() || value.eq_ignore_ascii_case("NA") || value.eq_ignore_ascii_case("N/A") {
        None
    } else {
        Some(value.to_owned())
    }
}

pub fn classify_error(stderr: &str) -> YtDlpErrorKind {
    let text = stderr.to_ascii_lowercase();
    if text.contains("sign in") || text.contains("login") || text.contains("cookies") {
        YtDlpErrorKind::Authentication
    } else if text.contains("not available in your country") || text.contains("geo-restricted") {
        YtDlpErrorKind::GeoRestricted
    } else if text.contains("video unavailable") || text.contains("has been removed") {
        YtDlpErrorKind::VideoUnavailable
    } else if text.contains("ffmpeg")
        && (text.contains("not found") || text.contains("not installed"))
    {
        YtDlpErrorKind::MissingFfmpeg
    } else if text.contains("permission denied") {
        YtDlpErrorKind::PermissionDenied
    } else if text.contains("unsupported url") || text.contains("invalid url") {
        YtDlpErrorKind::InvalidUrl
    } else if text.contains("network")
        || text.contains("timed out")
        || text.contains("unable to download")
        || text.contains("connection")
    {
        YtDlpErrorKind::Network
    } else {
        YtDlpErrorKind::Unknown
    }
}

pub fn detect_binary(name_or_path: impl AsRef<Path>) -> Option<PathBuf> {
    let requested = name_or_path.as_ref();
    if requested.components().count() > 1 {
        return is_executable(requested).then(|| requested.to_path_buf());
    }
    env::var_os("PATH").and_then(|path| {
        env::split_paths(&path)
            .map(|directory| directory.join(requested))
            .find(|candidate| is_executable(candidate))
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cookie_auth(browser: crate::domain::Browser, profile: Option<&str>) -> Authentication {
        Authentication::BrowserCookies {
            browser,
            profile: profile.map(str::to_owned),
        }
    }

    #[test]
    fn browser_profile_args_match_yt_dlp_syntax() {
        for (browser, expected) in [
            (crate::domain::Browser::Brave, "brave:Profile 1"),
            (crate::domain::Browser::Chrome, "chrome:Profile 1"),
            (crate::domain::Browser::Firefox, "firefox:Profile 1"),
        ] {
            let command = build_probe_command(
                &YtDlpPaths::default(),
                "https://example.test/video",
                &cookie_auth(browser, Some("Profile 1")),
                Some(Path::new("cookies.txt")),
                true,
            );
            let index = command
                .args
                .iter()
                .position(|arg| arg == "--cookies-from-browser")
                .unwrap();
            assert_eq!(command.args[index + 1], expected);
        }
    }

    #[test]
    fn probe_and_download_disable_playlists() {
        let auth = cookie_auth(crate::domain::Browser::Chrome, None);
        let jar = Path::new("cookies.txt");
        let probe = build_probe_command(&YtDlpPaths::default(), "url", &auth, Some(jar), true);
        let download = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("downloads"),
            &DownloadMode::default(),
            Some(jar),
        );
        for command in [probe, download] {
            assert!(command.args.iter().any(|arg| arg == "--no-playlist"));
            assert!(
                command
                    .args
                    .windows(2)
                    .any(|args| args == ["--cookies", "cookies.txt"])
            );
        }
    }

    #[test]
    fn browser_cookie_workflow_imports_once_then_reuses_one_session_jar() {
        let auth = cookie_auth(crate::domain::Browser::Brave, Some("Profile 1"));
        let jar = Path::new("session-cookies.txt");
        let probe = build_probe_command(&YtDlpPaths::default(), "url", &auth, Some(jar), true);
        let download = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("downloads"),
            &DownloadMode::default(),
            Some(jar),
        );
        let retry = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("downloads"),
            &DownloadMode::default(),
            Some(jar),
        );
        let commands = [&probe, &download, &retry];

        assert_eq!(
            commands
                .iter()
                .flat_map(|command| command.args.iter())
                .filter(|arg| arg.as_str() == "--cookies-from-browser")
                .count(),
            1
        );
        for command in commands {
            assert!(
                command
                    .args
                    .windows(2)
                    .any(|args| args == ["--cookies", jar.to_str().unwrap()])
            );
        }
    }

    #[test]
    fn download_command_enables_progress_when_printing_output_path() {
        let command = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("downloads"),
            &DownloadMode::default(),
            None,
        );

        assert!(command.args.iter().any(|arg| arg == "--print"));
        assert!(command.args.iter().any(|arg| arg == "--progress"));
    }

    #[test]
    fn probe_detects_2160p() {
        let metadata = parse_probe_json(
            r#"{"id":"abc","title":"Title","formats":[{"height":1080},{"height":2160}]}"#,
        )
        .unwrap();
        assert!(metadata.supports_2160p);
        assert!(metadata.available_qualities.contains(&Quality::P2160));
    }

    #[test]
    fn target_quality_never_exceeds_requested_height() {
        let command = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("."),
            &DownloadMode::Video {
                quality: Quality::P1080,
            },
            None,
        );
        assert!(command.args.windows(2).any(|args| {
            args == [
                "--format",
                "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
            ]
        }));
        assert!(
            command
                .args
                .windows(2)
                .any(|args| args == ["--remux-video", "mp4"])
        );
    }

    #[test]
    fn audio_and_subtitle_modes_have_required_args() {
        let audio = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("."),
            &DownloadMode::Audio,
            None,
        );
        assert!(
            audio
                .args
                .windows(2)
                .any(|args| args == ["--audio-format", "m4a"])
        );
        let subtitles = build_download_command(
            &YtDlpPaths::default(),
            "url",
            Path::new("."),
            &DownloadMode::Subtitles {
                language: "en".to_owned(),
                format: SubtitleFormat::Srt,
            },
            None,
        );
        assert!(
            subtitles
                .args
                .windows(2)
                .any(|args| args == ["--sub-langs", "en"])
        );
        assert!(
            subtitles
                .args
                .windows(2)
                .any(|args| args == ["--convert-subs", "srt"])
        );
        assert!(
            subtitles
                .args
                .windows(2)
                .any(|args| args == ["--sub-format", "srt/vtt/best"])
        );
        assert!(!subtitles.args.iter().any(|arg| arg == "--write-auto-subs"));
    }

    #[test]
    fn progress_parser_treats_na_as_missing() {
        let progress =
            parse_progress_line("FETCHDECK_PROGRESS:12|NA|N/A|3.5||downloading").unwrap();
        assert_eq!(progress.downloaded_bytes, Some(12));
        assert_eq!(progress.total_bytes, None);
        assert_eq!(progress.estimated_total_bytes, None);
        assert_eq!(progress.speed_bytes_per_second, Some(3.5));
        assert_eq!(progress.eta_seconds, None);
        assert_eq!(progress.status.as_deref(), Some("downloading"));
    }

    #[test]
    fn output_parser_only_accepts_marked_lines() {
        assert_eq!(
            parse_output_line("FETCHDECK_OUTPUT:/tmp/Title [abc].mp4"),
            Some(PathBuf::from("/tmp/Title [abc].mp4"))
        );
        assert_eq!(parse_output_line("ordinary log line"), None);
    }
}
