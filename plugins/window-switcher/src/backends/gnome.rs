use anyrun_helper::window::{WindowBackend, WindowInfo};
use serde::Deserialize;
use zbus::blocking::{Connection, Proxy};

const SERVICE: &str = "org.anyrun.WindowSwitcher";
const PATH: &str = "/org/anyrun/WindowSwitcher";
const INTERFACE: &str = "org.anyrun.WindowSwitcher1";

#[derive(Deserialize)]
struct GnomeWindow {
    id: String,
    title: Option<String>,
    app_id: Option<String>,
    workspace: Option<String>,
}

pub struct GnomeBackend {
    connection: Connection,
}

impl GnomeBackend {
    pub fn connect() -> Option<Self> {
        let connection = Connection::session().ok()?;
        let backend = Self { connection };
        if backend.list_windows_inner().is_ok() {
            Some(backend)
        } else {
            None
        }
    }

    fn proxy(&self) -> Result<Proxy<'_>, String> {
        Proxy::new(&self.connection, SERVICE, PATH, INTERFACE)
            .map_err(|e| format!("Failed to create D-Bus proxy: {e}"))
    }

    fn list_windows_inner(&self) -> Result<String, String> {
        let proxy = self.proxy()?;
        proxy
            .call("ListWindows", &())
            .map_err(|e| format!("Failed to call ListWindows: {e}"))
    }

    fn parse_windows(payload: &str) -> Vec<WindowInfo> {
        let windows: Vec<GnomeWindow> = match serde_json::from_str(payload) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[window-switcher] Failed to parse GNOME JSON payload: {e}");
                return Vec::new();
            }
        };

        windows
            .into_iter()
            .map(|w| WindowInfo {
                id: w.id,
                title: w.title.unwrap_or_default(),
                app_id: w.app_id.clone(),
                workspace: w.workspace,
                icon: w.app_id.clone(),
                app_name: None,
            })
            .collect()
    }
}

pub fn service_available() -> bool {
    let Ok(conn) = Connection::session() else {
        return false;
    };

    let Ok(proxy) = zbus::blocking::fdo::DBusProxy::new(&conn) else {
        return false;
    };

    match proxy.list_names() {
        Ok(names) => names.iter().any(|name| name.as_str() == SERVICE),
        Err(_) => false,
    }
}

impl WindowBackend for GnomeBackend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        match self.list_windows_inner() {
            Ok(payload) => Self::parse_windows(&payload),
            Err(e) => {
                eprintln!("[window-switcher] {e}");
                Vec::new()
            }
        }
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        let proxy = self.proxy()?;
        let focused: bool = proxy
            .call("FocusWindow", &(id.to_string()))
            .map_err(|e| format!("Failed to call FocusWindow: {e}"))?;

        if focused {
            Ok(())
        } else {
            Err("FocusWindow returned false".to_string())
        }
    }

    fn name(&self) -> &'static str {
        "GNOME"
    }
}

#[cfg(test)]
mod tests {
    use super::GnomeBackend;

    #[test]
    fn parse_gnome_payload() {
        let payload = r#"[
          {"id":"w1","title":"Terminal","app_id":"org.gnome.Terminal","workspace":"2"},
          {"id":"w2","title":"","app_id":"org.gnome.Nautilus","workspace":null}
        ]"#;

        let windows = GnomeBackend::parse_windows(payload);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].id, "w1");
        assert_eq!(windows[0].title, "Terminal");
        assert_eq!(windows[1].id, "w2");
    }

    #[test]
    fn parse_invalid_gnome_payload_returns_empty() {
        let windows = GnomeBackend::parse_windows("not-json");
        assert!(windows.is_empty());
    }
}
