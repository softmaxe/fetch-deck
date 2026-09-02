use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    #[default]
    Chrome,
    Firefox,
    Brave,
}

impl Browser {
    pub fn as_yt_dlp_name(&self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Brave => "brave",
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Authentication {
    #[default]
    None,
    BrowserCookies {
        browser: Browser,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        profile: Option<String>,
    },
}

impl Authentication {
    pub fn browser_cookie_source(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::BrowserCookies { browser, profile } => {
                let mut source = browser.as_yt_dlp_name().to_owned();
                if let Some(profile) = profile.as_deref().filter(|value| !value.is_empty()) {
                    source.push(':');
                    source.push_str(profile);
                }
                Some(source)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    #[default]
    Best,
    P2160,
    P1080,
    P720,
    P480,
}

impl Quality {
    pub fn height(self) -> Option<u32> {
        match self {
            Self::Best => None,
            Self::P2160 => Some(2160),
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::P480 => Some(480),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    Srt,
    #[default]
    Vtt,
}

impl SubtitleFormat {
    pub fn as_yt_dlp_name(self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DownloadMode {
    Video {
        quality: Quality,
    },
    Audio,
    Subtitles {
        language: String,
        format: SubtitleFormat,
    },
}

impl Default for DownloadMode {
    fn default() -> Self {
        Self::Video {
            quality: Quality::Best,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    #[default]
    Queued,
    Probing,
    Downloading,
    Merging,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Probing => "Probing",
            Self::Downloading => "Downloading",
            Self::Merging => "Merging",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Downloading | Self::Merging)
    }

    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct JobProgress {
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub estimated_total_bytes: Option<u64>,
    pub speed_bytes_per_second: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub language: String,
    pub name: Option<String>,
    pub formats: Vec<SubtitleFormat>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub id: String,
    pub title: String,
    pub duration_seconds: Option<u64>,
    pub thumbnail_url: Option<String>,
    pub subtitles: Vec<SubtitleTrack>,
    pub available_qualities: Vec<Quality>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: String,
    pub url: String,
    pub mode: DownloadMode,
    pub authentication: Authentication,
    pub output_directory: PathBuf,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub metadata: Option<MediaMetadata>,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub status: JobStatus,
    pub output_path: Option<PathBuf>,
    pub timestamp_unix_seconds: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub output_directory: PathBuf,
    pub yt_dlp_path: Option<PathBuf>,
    pub ffmpeg_path: Option<PathBuf>,
    pub cookie_notice_acknowledged: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let output_directory = directories::UserDirs::new()
            .and_then(|directories| directories.download_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            output_directory,
            yt_dlp_path: None,
            ffmpeg_path: None,
            cookie_notice_acknowledged: false,
        }
    }
}
