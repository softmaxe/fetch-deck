use crate::domain::{AppConfig, HistoryEntry};
use directories::ProjectDirs;
use serde::de::DeserializeOwned;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid config: {0}")]
    ConfigDecode(#[from] toml::de::Error),
    #[error("cannot encode config: {0}")]
    ConfigEncode(#[from] toml::ser::Error),
    #[error("invalid history: {0}")]
    HistoryDecode(#[from] serde_json::Error),
    #[error("OS project directories are unavailable")]
    ProjectDirectoriesUnavailable,
}

#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> Result<PathBuf, StorageError> {
        Ok(project_dirs()?.config_dir().join("config.toml"))
    }

    pub fn for_default_location() -> Result<Self, StorageError> {
        Ok(Self::new(Self::default_path()?))
    }

    pub fn load(&self) -> Result<AppConfig, StorageError> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), StorageError> {
        atomic_write(&self.path, toml::to_string_pretty(config)?.as_bytes())
    }
}

#[derive(Clone, Debug)]
pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> Result<PathBuf, StorageError> {
        Ok(project_dirs()?.data_local_dir().join("history.json"))
    }

    pub fn for_default_location() -> Result<Self, StorageError> {
        Ok(Self::new(Self::default_path()?))
    }

    pub fn load(&self) -> Result<Vec<HistoryEntry>, StorageError> {
        load_json_or_default(&self.path)
    }

    pub fn save(&self, entries: &[HistoryEntry]) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(entries)?;
        atomic_write(&self.path, &bytes)
    }

    pub fn clear(&self) -> Result<(), StorageError> {
        self.save(&[])
    }
}

fn project_dirs() -> Result<ProjectDirs, StorageError> {
    ProjectDirs::from("com", "softmaxe", "fetchdeck")
        .ok_or(StorageError::ProjectDirectoriesUnavailable)
}

fn load_json_or_default<T>(path: &Path) -> Result<T, StorageError>
where
    T: DeserializeOwned + Default,
{
    match fs::read(path) {
        Ok(contents) => Ok(serde_json::from_slice(&contents)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(error.into()),
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), StorageError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{HistoryEntry, JobStatus};

    #[test]
    fn config_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(directory.path().join("config.toml"));
        let config = AppConfig {
            output_directory: PathBuf::from("downloads"),
            yt_dlp_path: Some(PathBuf::from("/opt/homebrew/bin/yt-dlp")),
            ffmpeg_path: Some(PathBuf::from("/opt/homebrew/bin/ffmpeg")),
            cookie_notice_acknowledged: true,
        };
        store.save(&config).unwrap();
        assert_eq!(store.load().unwrap(), config);
    }

    #[test]
    fn history_round_trip_and_clear() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.json");
        let store = HistoryStore::new(&path);
        let entry = HistoryEntry {
            url: "https://example.test/video".to_owned(),
            title: "Title".to_owned(),
            status: JobStatus::Completed,
            output_path: Some(PathBuf::from("downloads/Title [abc].mp4")),
            timestamp_unix_seconds: 42,
        };
        store.save(std::slice::from_ref(&entry)).unwrap();
        assert_eq!(store.load().unwrap(), vec![entry]);
        let serialized = fs::read_to_string(path).unwrap();
        for forbidden in ["cookie", "browser", "profile", "command", "Brave"] {
            assert!(!serialized
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase()));
        }
        store.clear().unwrap();
        assert!(store.load().unwrap().is_empty());
    }
}
