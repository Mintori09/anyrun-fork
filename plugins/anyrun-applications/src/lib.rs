use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::{anyrun_interface::HandleResult, *};
use fuzzy_matcher::FuzzyMatcher;
use scrubber::DesktopEntry;
use serde::Deserialize;
use std::{env, fs, path::PathBuf, process::Command};

#[derive(Deserialize)]
pub struct Config {
    desktop_actions: bool,
    max_entries: usize,
    #[serde(default)]
    hide_description: bool,
    terminal: Option<Terminal>,
    preprocess_exec_script: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct Terminal {
    command: String,
    args: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop_actions: false,
            max_entries: 5,
            hide_description: false,
            preprocess_exec_script: None,
            terminal: None,
        }
    }
}

pub struct State {
    config: Config,
    entries: Vec<(DesktopEntry, u64)>,
}

mod scrubber;

#[handler]
pub fn handler(selection: Match, state: &State) -> HandleResult {
    let entry = state
        .entries
        .iter()
        .find_map(|(entry, id)| {
            if *id == selection.id.unwrap() {
                Some(entry)
            } else {
                None
            }
        })
        .unwrap();

    let exec = if let Some(script) = &state.config.preprocess_exec_script {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "{} {} {}",
                script.display(),
                if entry.term { "term" } else { "no-term" },
                &entry.exec
            ))
            .output()
            .unwrap_or_else(|why| {
                eprintln!("[applications] Error running preprocess script: {}", why);
                std::process::exit(1);
            });

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        entry.exec.clone()
    };

    if entry.term {
        match &state.config.terminal {
            Some(term) => {
                if let Err(why) = Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "{} {}",
                        term.command,
                        term.args.replace("{}", &exec)
                    ))
                    .spawn()
                {
                    eprintln!("[applications] Error running desktop entry: {}", why);
                }
            }
            None => {
                let sensible_terminals = &[
                    Terminal {
                        command: "alacritty".to_string(),
                        args: "-e {}".to_string(),
                    },
                    Terminal {
                        command: "foot".to_string(),
                        args: "-e \"{}\"".to_string(),
                    },
                    Terminal {
                        command: "kitty".to_string(),
                        args: "-e \"{}\"".to_string(),
                    },
                    Terminal {
                        command: "wezterm".to_string(),
                        args: "-e \"{}\"".to_string(),
                    },
                    Terminal {
                        command: "wterm".to_string(),
                        args: "-e \"{}\"".to_string(),
                    },
                    Terminal {
                        command: "ghostty".to_string(),
                        args: "-e \"{}\"".to_string(),
                    },
                ];
                for term in sensible_terminals {
                    if Command::new("which")
                        .arg(&term.command)
                        .output()
                        .is_ok_and(|output| output.status.success())
                    {
                        if let Err(why) = Command::new("sh")
                            .arg("-c")
                            .arg(format!(
                                "{} {}",
                                term.command,
                                term.args.replace("{}", &exec)
                            ))
                            .spawn()
                        {
                            eprintln!("Error running desktop entry: {}", why);
                        }
                        break;
                    }
                }
            }
        }
    } else if let Err(why) = {
        let current_dir = &env::current_dir().unwrap();

        Command::new("sh")
            .arg("-c")
            .arg(&entry.exec)
            .current_dir(match &entry.path {
                Some(path) if path.exists() => path,
                _ => current_dir,
            })
            .spawn()
    } {
        eprintln!("Error running desktop entry: {}", why);
    }

    HandleResult::Close
}

#[init]
pub fn init(config_dir: RString) -> State {
    let config: Config = match fs::read_to_string(format!("{}/applications.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!(
                "[applications] Error parsing config, using default: {}",
                why
            );
            Config::default()
        }),
        Err(why) => {
            eprintln!(
                "[applications] Error reading config, using default: {}",
                why
            );
            Config::default()
        }
    };

    let entries = scrubber::scrubber(&config).unwrap_or_else(|why| {
        eprintln!("[applicatiosn] Failed to load desktop entries: {}", why);
        Vec::new()
    });

    State { config, entries }
}

#[get_matches]
pub fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_lc = input.to_lowercase();
    let input_trimmed = input_lc.trim();

    if input_trimmed.trim().is_empty() {
        return state
            .entries
            .iter()
            .take(state.config.max_entries)
            .map(|(entry, id)| Match {
                title: entry.localized_name().into(),
                description: if state.config.hide_description {
                    ROption::RNone
                } else {
                    entry.desc.clone().map(|d| d.into()).into()
                },
                use_pango: false,
                icon: ROption::RSome(entry.icon.clone().into()),
                id: ROption::RSome(*id),
            })
            .collect();
    }

    const ACTION_VERBS: &[&str] = &["quit", "close", "exit", "kill", "stop", "restart"];
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
    let tokens: Vec<&str> = input_lc.split_whitespace().collect();

    let has_action_verb = tokens.iter().any(|t| ACTION_VERBS.contains(t));

    let mut entries = state
        .entries
        .iter()
        .filter_map(|(entry, id)| {
            let mut score = 0;

            // ===== Pre-lowercase entry fields =====
            let title_lc = entry.localized_name().to_lowercase();
            let name_lc = entry.name.to_lowercase();
            let desc_lc = entry.desc.as_ref().map(|d| d.to_lowercase());

            let keywords_lc: Vec<String> = entry
                .keywords
                .iter()
                .map(|k| k.to_lowercase())
                .chain(
                    entry
                        .localized_keywords
                        .iter()
                        .flat_map(|ks| ks.iter().map(|k| k.to_lowercase())),
                )
                .collect();

            // ===== Token-based matching =====
            for token in &tokens {
                let title_score = matcher.fuzzy_match(&title_lc, token).unwrap_or(0);

                let name_score = matcher.fuzzy_match(&name_lc, token).unwrap_or(0);

                let desc_score = desc_lc
                    .as_ref()
                    .and_then(|d| matcher.fuzzy_match(d, token))
                    .unwrap_or(0);

                let keyword_score = keywords_lc
                    .iter()
                    .filter_map(|k| matcher.fuzzy_match(k, token))
                    .max()
                    .unwrap_or(0);

                let best = title_score
                    .max(name_score)
                    .max(desc_score)
                    .max(keyword_score);

                if best == 0 {
                    return None;
                }

                // ===== Weighting =====
                score += title_score * 10;
                score += name_score * 8;
                score += desc_score * 5;
                score += keyword_score * 3;
            }

            // ===== Offset penalty =====
            score -= entry.offset;

            // ===== Action vs App prioritization =====
            if entry.is_action {
                if has_action_verb {
                    score *= 3;
                } else {
                    score /= 2;
                }
            }

            if score > 0 {
                Some((entry, *id, score))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // ===== Sort & truncate =====
    entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.name.cmp(&b.0.name)));
    entries.truncate(state.config.max_entries);

    // ===== Build result =====
    entries
        .into_iter()
        .map(|(entry, id, _)| Match {
            title: entry.localized_name().into(),
            description: if state.config.hide_description {
                ROption::RNone
            } else {
                entry.desc.clone().map(|d| d.into()).into()
            },
            use_pango: false,
            icon: ROption::RSome(entry.icon.clone().into()),
            id: ROption::RSome(id),
        })
        .collect()
}

#[info]
pub fn info() -> PluginInfo {
    PluginInfo {
        name: "Applications".into(),
        icon: "application-x-executable".into(),
    }
}
