use std::process::Command;

use anyrun_helper::window::{WindowBackend, WindowInfo};
use serde::Deserialize;

#[derive(Deserialize)]
struct HyprctlClient {
    address: String,
    class: Option<String>,
    title: Option<String>,
    workspace: Option<HyprctlWorkspace>,
}

#[derive(Deserialize)]
struct HyprctlWorkspace {
    id: i64,
    name: Option<String>,
}

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

pub struct HyprlandBackend;

impl HyprlandBackend {
    fn run_hyprctl(args: &[&str]) -> Option<String> {
        let output = Command::new("hyprctl").args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }
}

impl WindowBackend for HyprlandBackend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let json = match Self::run_hyprctl(&["clients", "-j"]) {
            Some(j) => j,
            None => return Vec::new(),
        };

        let clients: Vec<HyprctlClient> = match serde_json::from_str(&json) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[window-switcher] Failed to parse hyprctl output: {e}");
                return Vec::new();
            }
        };

        let locales = freedesktop_desktop_entry::get_languages_from_env();
        let entries = freedesktop_desktop_entry::desktop_entries(&locales);

        let mut results = Vec::new();

        for client in clients {
            let (app_name, icon, app_id) = client
                .class
                .as_deref()
                .map(|class| resolve_app(&entries, &locales, class))
                .unwrap_or((None, None, None));

            let workspace = client
                .workspace
                .and_then(|ws| ws.name.or_else(|| Some(format!("Workspace {}", ws.id))));

            results.push(WindowInfo {
                id: client.address.clone(),
                title: client.title.unwrap_or_default(),
                app_id,
                workspace,
                icon,
                app_name,
            });
        }

        results
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        let output = Command::new("hyprctl")
            .args(["dispatch", "focuswindow", &format!("address:{id}")])
            .output()
            .map_err(|e| format!("Failed to run hyprctl: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn name(&self) -> &'static str {
        "Hyprland"
    }
}
