use freedesktop_desktop_entry as fde;
use freedesktop_desktop_entry::DesktopEntry;
use std::env;
use std::path::PathBuf;

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
