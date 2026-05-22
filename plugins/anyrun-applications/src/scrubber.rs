use std::{collections::HashMap, env, ffi::OsStr, fs, path::PathBuf};

use crate::Config;

#[derive(Clone, Debug)]
pub struct DesktopEntry {
    pub exec: String,
    pub path: Option<PathBuf>,
    pub name: String,
    pub localized_name: Option<String>,
    pub keywords: Vec<String>,
    pub localized_keywords: Option<Vec<String>>,
    pub desc: Option<String>,
    pub icon: String,
    pub term: bool,
    pub offset: i64,
    pub is_action: bool,
}

const FIELD_CODE_LIST: &[&str] = &[
    "%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%k", "%v", "%m",
];

impl DesktopEntry {
    pub fn localized_name(&self) -> String {
        self.localized_name
            .clone()
            .unwrap_or_else(|| self.name.clone())
    }

    fn from_dir_entry(
        entry: &fs::DirEntry,
        config: &Config,
        lang_choices: &LangChoices,
    ) -> Vec<Self> {
        if entry.path().extension() == Some(OsStr::new("desktop")) {
            let content = match fs::read_to_string(entry.path()) {
                Ok(content) => content,
                Err(_) => return Vec::new(),
            };

            let lines = content.lines().collect::<Vec<_>>();

            let sections = lines
                .split_inclusive(|line| line.starts_with('['))
                .collect::<Vec<_>>();

            let mut line = None;
            let mut new_sections = Vec::new();

            for (i, section) in sections.iter().enumerate() {
                if let Some(line) = line {
                    let mut section = section.to_vec();
                    section.insert(0, line);

                    // Only pop the last redundant entry if it isn't the last item
                    if i < sections.len() - 1 {
                        section.pop();
                    }
                    new_sections.push(section);
                }
                line = Some(section.last().unwrap_or(&""));
            }

            let mut ret = Vec::new();

            let entry = match new_sections.iter().find_map(|section| {
                if section[0].starts_with("[Desktop Entry]") {
                    let mut map = HashMap::new();

                    for line in section.iter().skip(1) {
                        if let Some((key, val)) = line.split_once('=') {
                            map.insert(key, val);
                        }
                    }

                    if map.get("Type")? == &"Application"
                        && match map.get("NoDisplay") {
                            Some(no_display) => !no_display.parse::<bool>().unwrap_or(true),
                            None => true,
                        }
                    {
                        Some(DesktopEntry {
                            exec: {
                                let mut exec = map.get("Exec")?.to_string();

                                for field_code in FIELD_CODE_LIST {
                                    exec = exec.replace(field_code, "");
                                }
                                exec
                            },
                            path: map.get("Path").map(PathBuf::from),
                            name: map.get("Name")?.to_string(),
                            localized_name: lang_choices
                                .localized_keys("Name")
                                .find_map(|key| map.get(&*key))
                                .map(ToString::to_string),
                            keywords: map
                                .get("Keywords")
                                .map(|keywords| {
                                    keywords
                                        .split(';')
                                        .map(|s| s.to_owned())
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default(),
                            localized_keywords: lang_choices
                                .localized_keys("Keywords")
                                .find_map(|key| map.get(&*key))
                                .map(|keywords| {
                                    keywords
                                        .split(';')
                                        .map(|s| s.to_owned())
                                        .collect::<Vec<_>>()
                                }),
                            desc: lang_choices
                                .localized_keys("Comment")
                                .find_map(|key| map.get(&*key))
                                .or_else(|| map.get("Comment"))
                                .map(ToString::to_string),
                            icon: map
                                .get("Icon")
                                .unwrap_or(&"application-x-executable")
                                .to_string(),
                            term: map
                                .get("Terminal")
                                .map(|val| val.to_lowercase() == "true")
                                .unwrap_or(false),
                            offset: 0,
                            is_action: false,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                Some(entry) => entry,
                None => return Vec::new(),
            };

            if config.desktop_actions {
                for (i, section) in new_sections.iter().enumerate() {
                    let mut map = HashMap::new();

                    for line in section.iter().skip(1) {
                        if let Some((key, val)) = line.split_once('=') {
                            map.insert(key, val);
                        }
                    }

                    if section[0].starts_with("[Desktop Action") {
                        ret.push(DesktopEntry {
                            exec: match map.get("Exec") {
                                Some(exec) => {
                                    let mut exec = exec.to_string();

                                    for field_code in FIELD_CODE_LIST {
                                        exec = exec.replace(field_code, "");
                                    }
                                    exec
                                }
                                None => continue,
                            },
                            path: entry.path.clone(),
                            name: match map.get("Name") {
                                Some(name) => name.to_string(),
                                None => continue,
                            },
                            localized_name: lang_choices
                                .localized_keys("Name")
                                .find_map(|key| map.get(&*key))
                                .map(ToString::to_string),
                            keywords: map
                                .get("Keywords")
                                .map(|keywords| {
                                    keywords
                                        .split(';')
                                        .map(|s| s.to_owned())
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default(),
                            localized_keywords: lang_choices
                                .localized_keys("Keywords")
                                .find_map(|key| map.get(&*key))
                                .map(|keywords| {
                                    keywords
                                        .split(';')
                                        .map(|s| s.to_owned())
                                        .collect::<Vec<_>>()
                                }),
                            desc: Some(entry.localized_name()),
                            icon: entry.icon.clone(),
                            term: map
                                .get("Terminal")
                                .map(|val| val.to_lowercase() == "true")
                                .unwrap_or(false),
                            offset: i as i64,
                            is_action: true,
                        })
                    }
                }
            }

            ret.push(entry);
            ret
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug, Default)]
struct LangChoices<'a> {
    whole: Option<&'a str>,
    prefix: Option<&'a str>,
    short: Option<&'a str>,
}

impl<'a> LangChoices<'a> {
    fn new(lang: Option<&'a str>) -> Self {
        let mut ret = Self::default();

        // example: en_US.UTF-8
        let Some(whole) = lang else {
            return ret;
        };
        ret.whole = Some(whole);

        // example: en_US
        let Some((prefix, _)) = whole.split_once('.') else {
            return ret;
        };
        ret.prefix = Some(prefix);

        // example: en
        let Some((short, _)) = prefix.split_once('_') else {
            return ret;
        };
        ret.short = Some(short);

        ret
    }

    fn localized_keys(&self, key: &'a str) -> impl Iterator<Item = String> + 'a {
        let choices = (self.whole.into_iter())
            .chain(self.prefix)
            .chain(self.short);
        choices.map(move |choice| format!("{key}[{choice}]"))
    }
}

fn xdg_application_dirs() -> Vec<PathBuf> {
    let Some(data_dirs) = env::var_os("XDG_DATA_DIRS") else {
        return vec![PathBuf::from("/usr/share/applications")];
    };

    let mut ret = Vec::new();
    for dir in env::split_paths(&data_dirs) {
        let apps_dir = dir.join("applications");
        if !ret.contains(&apps_dir) {
            ret.push(apps_dir);
        }
    }
    ret
}

pub fn scrubber(config: &Config) -> Result<Vec<(DesktopEntry, u64)>, Box<dyn std::error::Error>> {
    // Create iterator over all the files in the XDG_DATA_DIRS
    // XDG compliancy is cool
    let user_path = match env::var("XDG_DATA_HOME") {
        Ok(data_home) => {
            format!("{}/applications/", data_home)
        }
        Err(_) => {
            format!(
                "{}/.local/share/applications/",
                env::var("HOME").expect("Unable to determine home directory!")
            )
        }
    };

    let lang = env::var("LANG").ok();
    let lang_choices = LangChoices::new(lang.as_deref());

    let mut paths = Vec::new();
    for dir in xdg_application_dirs() {
        if !dir.exists() {
            continue;
        }
        match fs::read_dir(&dir) {
            Ok(entries) => {
                paths.extend(entries);
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => {
                eprintln!(
                    "[applications] Error reading directory {}: {}",
                    dir.display(),
                    why
                );
            }
        }
    }
    if paths.is_empty() {
        return Err("No valid desktop file dirs found!".into());
    }

    let mut entries: HashMap<String, DesktopEntry> = paths
        .into_iter()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_why) => return None,
            };
            let entries = DesktopEntry::from_dir_entry(&entry, config, &lang_choices);
            Some(
                entries
                    .into_iter()
                    .map(|entry| (format!("{}{}", entry.name, entry.icon), entry)),
            )
        })
        .flatten()
        .collect();

    // Go through user directory desktop files for overrides
    match fs::read_dir(&user_path) {
        Ok(dir_entries) => entries.extend(
            dir_entries
                .into_iter()
                .filter_map(|entry| {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_why) => return None,
                    };
                    let entries = DesktopEntry::from_dir_entry(&entry, config, &lang_choices);
                    Some(
                        entries
                            .into_iter()
                            .map(|entry| (format!("{}{}", entry.name, entry.icon), entry)),
                    )
                })
                .flatten(),
        ),
        Err(why) => eprintln!(
            "[applications] Error reading directory {}: {}",
            user_path, why
        ),
    }

    Ok(entries
        .into_iter()
        .enumerate()
        .map(|(i, (_, entry))| (entry, i as u64))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::xdg_application_dirs;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn xdg_application_dirs_deduplicates_and_appends_applications() {
        let _guard = env_lock();
        let original = std::env::var_os("XDG_DATA_DIRS");
        unsafe { std::env::set_var("XDG_DATA_DIRS", "/usr/share:/usr/share:/opt/share") };

        let dirs = xdg_application_dirs();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], std::path::PathBuf::from("/usr/share/applications"));
        assert_eq!(dirs[1], std::path::PathBuf::from("/opt/share/applications"));

        if let Some(val) = original {
            unsafe { std::env::set_var("XDG_DATA_DIRS", val) };
        } else {
            unsafe { std::env::remove_var("XDG_DATA_DIRS") };
        }
    }

    #[test]
    fn xdg_application_dirs_falls_back_to_usr_share() {
        let _guard = env_lock();
        let original = std::env::var_os("XDG_DATA_DIRS");
        unsafe { std::env::remove_var("XDG_DATA_DIRS") };

        let dirs = xdg_application_dirs();
        assert_eq!(
            dirs,
            vec![std::path::PathBuf::from("/usr/share/applications")]
        );

        if let Some(val) = original {
            unsafe { std::env::set_var("XDG_DATA_DIRS", val) };
        }
    }
}
