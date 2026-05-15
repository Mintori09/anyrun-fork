use std::sync::Mutex;

use anyrun_helper::window::{WindowBackend, WindowInfo};
use niri_ipc::{Action, Request, Window, socket::Socket};

pub struct NiriBackend {
    socket: Mutex<Socket>,
}

fn resolve_app(
    entries: &[freedesktop_desktop_entry::DesktopEntry],
    locales: &[impl AsRef<str>],
    app_id: &str,
) -> (Option<String>, Option<String>) {
    let entry = freedesktop_desktop_entry::find_app_by_id(
        entries,
        freedesktop_desktop_entry::unicase::Ascii::new(app_id),
    );
    match entry {
        Some(e) => (
            e.name(locales).map(|n| n.to_string()),
            e.icon().map(|i| i.to_string()),
        ),
        None => (None, None),
    }
}

impl NiriBackend {
    pub fn connect() -> Option<Self> {
        let Ok(socket) = Socket::connect() else {
            eprintln!("[window-switcher] Failed to connect to niri socket");
            return None;
        };

        Some(Self {
            socket: Mutex::new(socket),
        })
    }
}

impl WindowBackend for NiriBackend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let mut socket = match self.socket.lock() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[window-switcher] Niri lock poisoned: {e}");
                return Vec::new();
            }
        };

        let windows: Vec<Window> = match socket.send(Request::Windows) {
            Ok(Ok(niri_ipc::Response::Windows(windows))) => windows,
            _ => {
                eprintln!("[window-switcher] Failed to get window list from niri");
                return Vec::new();
            }
        };

        drop(socket);

        let locales = freedesktop_desktop_entry::get_languages_from_env();
        let entries = freedesktop_desktop_entry::desktop_entries(&locales);

        windows
            .into_iter()
            .map(|window| {
                let (app_name, icon) = window
                    .app_id
                    .as_deref()
                    .map(|id| resolve_app(&entries, &locales, id))
                    .unwrap_or((None, None));

                WindowInfo {
                    id: window.id.to_string(),
                    title: window
                        .title
                        .clone()
                        .unwrap_or_else(|| window.app_id.clone().unwrap_or_default()),
                    app_id: window.app_id.clone(),
                    workspace: None,
                    icon,
                    app_name,
                }
            })
            .collect()
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        let niri_id: u64 = id.parse().map_err(|e| format!("Invalid window ID: {e}"))?;
        let mut socket = self
            .socket
            .lock()
            .map_err(|e| format!("Lock poisoned: {e}"))?;
        socket
            .send(Request::Action(Action::FocusWindow { id: niri_id }))
            .map_err(|e| format!("Failed to focus window: {e}"))?
            .map_err(|e| format!("Niri error: {e:?}"))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Niri"
    }
}
