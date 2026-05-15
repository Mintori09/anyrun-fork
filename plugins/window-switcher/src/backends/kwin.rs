use std::process::Command;

use anyrun_helper::window::{WindowBackend, WindowInfo};

fn resolve_app(
    entries: &[freedesktop_desktop_entry::DesktopEntry],
    locales: &[impl AsRef<str>],
    class: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    let entry = freedesktop_desktop_entry::find_app_by_id(
        entries,
        freedesktop_desktop_entry::unicase::Ascii::new(class),
    );
    match entry {
        Some(e) => (
            e.name(locales).map(|n| n.to_string()),
            e.icon().map(|i| i.to_string()),
            Some(class.to_string()),
        ),
        None => (None, None, Some(class.to_string())),
    }
}

pub struct KWinBackend;

impl KWinBackend {
    fn run_kdotool(args: &[&str]) -> Option<String> {
        let output = Command::new("kdotool").args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }
}

impl WindowBackend for KWinBackend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let ids_output = match Self::run_kdotool(&["search", "."]) {
            Some(out) => out,
            None => return Vec::new(),
        };

        let ids: Vec<&str> = ids_output
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if ids.is_empty() {
            return Vec::new();
        }

        let script = ids
            .iter()
            .map(|id| {
                format!(
                    "echo \"$(kdotool getwindowclassname {id})|$(kdotool getwindowname {id})|{id}\""
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let output = match Command::new("sh").args(["-c", &script]).output() {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return Vec::new(),
        };

        let locales = freedesktop_desktop_entry::get_languages_from_env();
        let entries = freedesktop_desktop_entry::desktop_entries(&locales);

        let mut results = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() < 3 {
                continue;
            }
            let class = parts[0].trim();
            let title = parts[1].trim().to_string();
            let id_str = parts[2].trim();

            let (app_name, icon, app_id) = if class.is_empty() {
                (None, None, None)
            } else {
                resolve_app(&entries, &locales, class)
            };

            let title = if title.is_empty() {
                app_name.clone().unwrap_or_else(|| class.to_string())
            } else {
                title
            };

            results.push(WindowInfo {
                id: id_str.to_string(),
                title,
                app_id,
                workspace: None,
                icon,
                app_name,
            });
        }

        results
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        let output = Command::new("kdotool")
            .args(["windowactivate", id])
            .output()
            .map_err(|e| format!("Failed to run kdotool: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn name(&self) -> &'static str {
        "KWin"
    }
}
