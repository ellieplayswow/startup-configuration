use freedesktop_desktop_entry as fde;
use freedesktop_desktop_entry::DesktopEntry;
use std::{env, fs};
use std::path::PathBuf;
use std::time::Instant;

const AUTOSTART: &'static str = "autostart";

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum DirectoryType {
    /// Directory for the current user
    User,

    /// System directories
    System,
}

impl Into<Vec<PathBuf>> for DirectoryType {
    fn into(self) -> Vec<PathBuf> {
        match self {
            DirectoryType::User => vec![dirs::config_dir()
                .expect("config dir not found")
                .join(AUTOSTART)],
            DirectoryType::System => {
                let mut vec: Vec<PathBuf> = Vec::new();

                if let Ok(xdg_dir) = env::var("XDG_CONFIG_DIRS") {
                    for dir in xdg_dir.split(':') {
                        // when running as a flatpak, /etc is mounted under /run/host/etc
                        #[cfg(feature = "flatpak")]
                        if dir.starts_with("/etc/") {
                            vec.push(PathBuf::from("/run/host/").join(dir.strip_prefix("/").expect("This should never fail")).join(AUTOSTART));
                        }
                        else {
                            vec.push(PathBuf::from(dir).join(AUTOSTART));
                        }

                        #[cfg(not(feature = "flatpak"))]
                        vec.push(PathBuf::from(dir).join(AUTOSTART));
                    }
                } else {
                    #[cfg(feature = "flatpak")]
                    vec.push(PathBuf::from("/run/host/etc/xdg/").join(AUTOSTART));

                    #[cfg(not(feature = "flatpak"))]
                    vec.push(PathBuf::from("/etc/xdg/").join(AUTOSTART));
                }

                vec
            }
        }
    }
}

pub fn get_installed_applications(locales: Vec<String>) -> Vec<DesktopEntry> {
    let mut dedup = std::collections::HashSet::new();

    let default_paths = fde::default_paths();

    let mut valid_paths = Vec::new();
    for path in default_paths {
        // when running as a flatpak, we'll find /usr/* under /run/host/usr/*
        #[cfg(feature = "flatpak")]
        if path.starts_with("/usr/") {
            valid_paths.push(PathBuf::from("/run/host/").join(path.strip_prefix("/").expect("This should never fail")));
        }
        else {
            valid_paths.push(path);
        }

        #[cfg(not(feature = "flatpak"))]
        valid_paths.push(path);
    }

    #[cfg(feature = "flatpak")]
    {
        valid_paths.push(dirs::home_dir().expect("home dir not found").join(".local/share/applications"));
        valid_paths.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    }

    let entries = fde::Iter::new(valid_paths.into_iter()).entries(Some(&locales));

    let current_desktop = env::var("XDG_SESSION_DESKTOP");

    let mut res = Vec::new();

    for entry in entries {
        let app_id = entry.flatpak().unwrap_or_else(|| entry.appid.as_ref());

        if dedup.contains(app_id) {
            continue;
        }

        if entry.exec().is_none() {
            continue;
        }

        if entry.desktop_entry("X-CosmicApplet").is_some() {
            continue;
        }

        // match based off of current desktop environment if it exists
        if let Ok(ref desktop_str) = current_desktop {
            if let Some(only_show_in) = entry.only_show_in() {
                if !only_show_in.contains(&desktop_str.as_str()) {
                    continue;
                }
            }

            if let Some(not_show_in) = entry.not_show_in() {
                if not_show_in.contains(&desktop_str.as_str()) {
                    continue;
                }
            }
        }

        res.push(entry.clone());
        dedup.insert(app_id.to_owned());
    }

    // for flatpaks, we can't follow the exports/ directory because we can only :ro the app directory
    // due to symlink funkiness. we need to do some magic to convert these app/ directories into a
    // list of directories that will contain the "correct" flatpak desktop entries
    #[cfg(feature = "flatpak")]
    {
        let flatpak_paths = vec![
            dirs::home_dir().expect("home dir not found").join(".local/share/flatpak/app"),
            PathBuf::from("/var/lib/flatpak/app")
        ];

        let mut paths_to_iter = Vec::new();

        // manually unwrap these to avoid extra thinking
        for dir in flatpak_paths {
            match fs::read_dir(&dir) {
                Ok(dir) => {
                    for entry in dir {
                        if let Ok(entry) = entry {
                            paths_to_iter.push(entry.path().join("current/active/export/share/applications/"));
                        }
                    }
                }
                Err(_) => {}
            }
        }

        let entries = fde::Iter::new(paths_to_iter.into_iter()).entries(Some(&locales));

        for entry in entries {
            let app_id = entry.flatpak().unwrap_or_else(|| entry.appid.as_ref());
            if dedup.contains(app_id) {
                continue;
            }

            if entry.exec().is_none() {
                continue;
            }

            if entry.desktop_entry("X-CosmicApplet").is_some() {
                continue;
            }

            // match based off of current desktop environment if it exists
            if let Ok(ref desktop_str) = current_desktop {
                if let Some(only_show_in) = entry.only_show_in() {
                    if !only_show_in.contains(&desktop_str.as_str()) {
                        continue;
                    }
                }

                if let Some(not_show_in) = entry.not_show_in() {
                    if not_show_in.contains(&desktop_str.as_str()) {
                        continue;
                    }
                }
            }

            res.push(entry.clone());
            dedup.insert(app_id.to_owned());
        }
    }

    res
}

pub fn get_startup_applications(
    directory_type: DirectoryType,
    locales: Vec<String>,
) -> Vec<DesktopEntry> {
    let dirs: Vec<PathBuf> = directory_type.into();

    let entries = fde::Iter::new(dirs.into_iter()).entries(Some(&locales));

    let mut vec = entries.collect::<Vec<DesktopEntry>>();
    vec.sort_by(|a, b| {
        a.name(&locales)
            .unwrap_or(a.clone().appid.into())
            .to_string()
            .cmp(
                &b.name(&locales)
                    .unwrap_or(b.clone().appid.into())
                    .to_string(),
            )
    });

    vec
}
