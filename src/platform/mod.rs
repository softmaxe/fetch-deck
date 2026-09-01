use std::{fs, path::Path};

use directories::UserDirs;

use crate::domain::Browser;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserProfile {
    pub label: String,
    pub value: String,
}

pub fn discover_browser_profiles(browser: &Browser) -> Vec<BrowserProfile> {
    UserDirs::new()
        .map(|directories| discover_profiles_in_home(directories.home_dir(), browser))
        .unwrap_or_default()
}

fn discover_profiles_in_home(home: &Path, browser: &Browser) -> Vec<BrowserProfile> {
    let root = match browser {
        Browser::Chrome => home.join("Library/Application Support/Google/Chrome"),
        Browser::Brave => home.join("Library/Application Support/BraveSoftware/Brave-Browser"),
        Browser::Firefox => home.join("Library/Application Support/Firefox/Profiles"),
    };

    let mut profiles = fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let include = match browser {
                Browser::Chrome | Browser::Brave => {
                    name == "Default" || name.starts_with("Profile ")
                }
                Browser::Firefox => true,
            };
            include.then_some(BrowserProfile {
                label: name.clone(),
                value: name,
            })
        })
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| profile_rank(&left.label).cmp(&profile_rank(&right.label)));
    profiles
}

fn profile_rank(profile: &str) -> (u8, &str) {
    if profile == "Default" {
        (0, profile)
    } else if profile.contains("default-release") {
        (1, profile)
    } else {
        (2, profile)
    }
}

pub fn open_in_finder(path: &Path) -> std::io::Result<()> {
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    std::process::Command::new("open").arg(target).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_and_orders_chromium_profiles() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory
            .path()
            .join("Library/Application Support/BraveSoftware/Brave-Browser");
        fs::create_dir_all(root.join("Profile 2")).unwrap();
        fs::create_dir_all(root.join("Default")).unwrap();
        fs::create_dir_all(root.join("System Profile")).unwrap();

        let profiles = discover_profiles_in_home(directory.path(), &Browser::Brave);
        assert_eq!(
            profiles
                .iter()
                .map(|profile| profile.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Default", "Profile 2"]
        );
    }
}
