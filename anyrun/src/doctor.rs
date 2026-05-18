use crate::config::Config;
use anyrun_interface::PluginRef;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct DoctorReport {
    pub required_failures: usize,
    pub lines: Vec<String>,
}

pub fn run(config_dir: Option<&str>) -> i32 {
    let config_dir = config_dir
        .map(PathBuf::from)
        .unwrap_or_else(default_config_dir);
    let report = inspect_config_dir(&config_dir);
    for line in report.lines {
        println!("{line}");
    }
    if report.required_failures == 0 {
        0
    } else {
        1
    }
}

fn inspect_config_dir(config_dir: &Path) -> DoctorReport {
    let mut report = DoctorReport {
        required_failures: 0,
        lines: Vec::new(),
    };

    if !config_dir.exists() {
        report.required_failures += 1;
        report
            .lines
            .push(format!("config dir missing: {}", config_dir.display()));
        return report;
    }

    let config_path = config_dir.join("config.ron");
    let config = match fs::read(&config_path) {
        Ok(content) => match ron::de::from_bytes::<Config>(&content) {
            Ok(config) => {
                report
                    .lines
                    .push(format!("config ok: {}", config_path.display()));
                config
            }
            Err(why) => {
                report.required_failures += 1;
                report.lines.push(format!(
                    "config parse failed: {}: {why}",
                    config_path.display()
                ));
                return report;
            }
        },
        Err(why) => {
            report.required_failures += 1;
            report.lines.push(format!(
                "config read failed: {}: {why}",
                config_path.display()
            ));
            return report;
        }
    };

    let provider = expand_tilde(&config.provider);
    if resolve_executable(&provider).is_some() {
        report
            .lines
            .push(format!("provider ok: {}", config.provider.display()));
    } else {
        report.required_failures += 1;
        report
            .lines
            .push(format!("provider missing: {}", config.provider.display()));
    }

    let plugin_dirs = plugin_dirs(config_dir);
    for plugin in &config.plugins {
        match find_plugin(plugin, &plugin_dirs) {
            Some(path) => match abi_stable::library::lib_header_from_path(&path)
                .and_then(|header| header.init_root_module::<PluginRef>())
            {
                Ok(_) => report.lines.push(format!("plugin ok: {}", path.display())),
                Err(why) => {
                    report.required_failures += 1;
                    report
                        .lines
                        .push(format!("plugin load failed: {}: {why}", path.display()));
                }
            },
            None => {
                report.required_failures += 1;
                report
                    .lines
                    .push(format!("plugin missing: {}", plugin.display()));
            }
        }
    }

    report
}

fn default_config_dir() -> PathBuf {
    let user_dir = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".into()));
            p.push(".config");
            p
        })
        .join("anyrun");

    if user_dir.exists() {
        user_dir
    } else {
        anyrun_provider_ipc::CONFIG_DIRS
            .iter()
            .map(PathBuf::from)
            .find(|path| path.exists())
            .unwrap_or_else(|| PathBuf::from(anyrun_provider_ipc::CONFIG_DIRS[0]))
    }
}

fn plugin_dirs(config_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![config_dir.join("plugins")];
    if let Ok(path) = env::var("ANYRUN_PLUGINS") {
        dirs.push(PathBuf::from(path));
    }
    dirs.extend(anyrun_provider_ipc::PLUGIN_PATHS.iter().map(PathBuf::from));
    dirs
}

fn find_plugin(name: &Path, dirs: &[PathBuf]) -> Option<PathBuf> {
    let name = expand_tilde(name);
    if name.is_absolute() && name.exists() {
        return Some(name);
    }
    for dir in dirs {
        let p = dir.join(&name);
        if p.exists() {
            return Some(p);
        }

        let lib_name = format!("lib{}.so", name.to_string_lossy().replace('-', "_"));
        let p = dir.join(lib_name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn resolve_executable(path: &Path) -> Option<PathBuf> {
    if path.components().count() > 1 {
        return path.exists().then(|| path.to_path_buf());
    }

    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|dir| dir.join(path))
        .find(|candidate| candidate.exists())
}

fn expand_tilde(path: &Path) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") {
            if let Ok(home) = env::var("HOME") {
                return PathBuf::from(path_str.replacen('~', &home, 1));
            }
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn doctor_reports_missing_provider() {
        let root = std::env::temp_dir().join(format!(
            "anyrun-doctor-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("plugins")).unwrap();
        fs::write(
            root.join("config.ron"),
            r#"(provider: "/definitely/missing/anyrun-provider", plugins: [])"#,
        )
        .unwrap();

        let report = inspect_config_dir(&root);
        assert!(report.required_failures > 0);
        assert!(report
            .lines
            .iter()
            .any(|line| line.contains("provider missing")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn doctor_accepts_empty_valid_config() {
        let root = std::env::temp_dir().join(format!(
            "anyrun-doctor-valid-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("plugins")).unwrap();
        let provider = std::env::current_exe().unwrap();
        fs::write(
            root.join("config.ron"),
            format!(r#"(provider: "{}", plugins: [])"#, provider.display()),
        )
        .unwrap();

        let report = inspect_config_dir(&root);
        assert_eq!(report.required_failures, 0);
        assert!(report.lines.iter().any(|line| line.contains("config ok")));

        let _ = fs::remove_dir_all(root);
    }
}
