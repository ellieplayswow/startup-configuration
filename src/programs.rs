use std::collections::HashSet;
use std::iter::Map;
use freedesktop_desktop_entry as fde;
use freedesktop_desktop_entry::DesktopEntry;

#[derive(Clone, Debug)]
pub struct ProgramSettings {
    pub exec: String,
    pub args: Vec<String>,
    pub app_id: String
}

#[derive(Clone, Debug)]
pub struct Program {
    pub(crate) name: String,
    pub(crate) icon: String,
    pub(crate) settings: ProgramSettings,
}

pub fn get_installed_programs() -> Vec<Program> {
    let mut dedup = std::collections::HashSet::new();

    let locales = fde::get_languages_from_env();
    let paths = fde::Iter::new(fde::default_paths());

    let desktop_entries = DesktopEntry::from_paths(paths, &locales).map(|de| {
        return de.ok().and_then(|de| {
            // check if we've already indexed the app
            let app_id = de.flatpak().unwrap_or_else(|| de.appid.as_ref());

            if dedup.contains(app_id) {
                return None;
            }

            dedup.insert(app_id.to_owned());

            // we need an exec path
            if !de.exec().is_some() {
                return None;
            }

            // render name
            let mut name = String::new();
            name.push_str(de.name(&[] as &[&str]).unwrap().as_ref());

            let is_flatpak = de.flatpak().is_some();
            if is_flatpak {
                name.push_str(" (Flatpak)");
            }

            // build data
            let program = Program {
                name: name,
                icon: String::from(de.icon().unwrap_or("")),
                settings: ProgramSettings {
                    exec: String::from(de.exec().unwrap_or("")),
                    args: Vec::new(),
                    app_id: String::from(app_id)
                }
            };
            return Some(program);
        });
    }).flatten().collect::<Vec<_>>();

    return desktop_entries;
}

pub fn get_program_list(programs: &Vec<ProgramSettings>) -> Vec<Program> {
    let mut res = Vec::new();
    let installed_programs = get_installed_programs();

    let ignored_app_ids: HashSet<&str> = HashSet::new();

    // loop thru all installed programs
    for program in installed_programs {
        if ignored_app_ids.contains(&*program.settings.app_id) {
            continue;
        }

        // loop through every program we want
        for search in programs {
            if search.app_id == program.settings.app_id {
                res.push(Program {
                    name: program.name.clone(),
                    icon: program.icon.clone(),
                    settings: ProgramSettings {
                        exec: program.settings.exec.clone(),
                        args: search.args.clone(),
                        app_id: search.app_id.clone(),
                    }
                });
            }
        }
    }

    res
}