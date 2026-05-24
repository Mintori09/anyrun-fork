pub mod gnome;
pub mod hyprland;
pub mod i3;
pub mod kwin;
pub mod niri;
pub mod sway;

use crate::config::Config;
use anyrun_helper::window::WindowBackend;
use std::process::Command;

const DEFAULT_PROBE_ORDER: &[&str] = &["kwin", "niri", "hyprland", "sway", "i3", "gnome"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendKind {
    KWin,
    Niri,
    Hyprland,
    Sway,
    I3,
    Gnome,
}

#[derive(Default)]
struct Detector;

impl Detector {
    fn has_command(&self, command: &str, version_arg: Option<&str>) -> bool {
        let mut cmd = Command::new(command);
        if let Some(arg) = version_arg {
            cmd.arg(arg);
        }
        cmd.output().is_ok()
    }

    fn env_present(&self, key: &str) -> bool {
        std::env::var(key).is_ok()
    }

    fn gnome_session(&self) -> bool {
        std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .map(|v| v.split(':').any(|part| part.eq_ignore_ascii_case("gnome")))
            .unwrap_or(false)
            || std::env::var("DESKTOP_SESSION")
                .ok()
                .map(|v| v.eq_ignore_ascii_case("gnome"))
                .unwrap_or(false)
    }

    fn gnome_service_available(&self) -> bool {
        gnome::service_available()
    }

    fn can_connect_niri(&self) -> bool {
        niri::NiriBackend::connect().is_some()
    }

    fn supports(&self, name: &str) -> bool {
        match name {
            "kwin" => self.has_command("kdotool", Some("--version")),
            "niri" => self.env_present("NIRI_SOCKET") && self.can_connect_niri(),
            "hyprland" => self.env_present("HYPRLAND_INSTANCE_SIGNATURE"),
            "sway" => {
                self.env_present("SWAYSOCK") && self.has_command("swaymsg", Some("--version"))
            }
            "i3" => {
                !self.env_present("SWAYSOCK")
                    && self.has_command("i3-msg", Some("-v"))
                    && (self.env_present("I3SOCK")
                        || std::env::var("XDG_CURRENT_DESKTOP")
                            .ok()
                            .map(|v| v.to_ascii_lowercase().contains("i3"))
                            .unwrap_or(false))
            }
            "gnome" => {
                if !self.gnome_session() {
                    return false;
                }
                if self.gnome_service_available() {
                    true
                } else {
                    eprintln!(
                        "[window-switcher] GNOME session detected but org.anyrun.WindowSwitcher is unavailable"
                    );
                    false
                }
            }
            _ => false,
        }
    }
}

fn detect_backend_kind(config: &Config, detector: &Detector) -> Option<BackendKind> {
    if let Some(forced) = config.backend.as_deref() {
        let forced = forced.trim().to_ascii_lowercase();
        return match forced.as_str() {
            "kwin" if detector.supports("kwin") => Some(BackendKind::KWin),
            "niri" if detector.supports("niri") => Some(BackendKind::Niri),
            "hyprland" if detector.supports("hyprland") => Some(BackendKind::Hyprland),
            "sway" if detector.supports("sway") => Some(BackendKind::Sway),
            "i3" if detector.supports("i3") => Some(BackendKind::I3),
            "gnome" if detector.supports("gnome") => Some(BackendKind::Gnome),
            _ => {
                eprintln!("[window-switcher] Configured backend '{forced}' is unavailable");
                None
            }
        };
    }

    for candidate in &config.backend_probe_order {
        let name = candidate.trim().to_ascii_lowercase();
        if detector.supports(&name) {
            return match name.as_str() {
                "kwin" => Some(BackendKind::KWin),
                "niri" => Some(BackendKind::Niri),
                "hyprland" => Some(BackendKind::Hyprland),
                "sway" => Some(BackendKind::Sway),
                "i3" => Some(BackendKind::I3),
                "gnome" => Some(BackendKind::Gnome),
                _ => None,
            };
        }
    }

    None
}

pub fn detect_backend(config: &Config) -> Option<Box<dyn WindowBackend>> {
    let detector = Detector;
    match detect_backend_kind(config, &detector)? {
        BackendKind::KWin => Some(Box::new(kwin::KWinBackend)),
        BackendKind::Niri => {
            niri::NiriBackend::connect().map(|b| Box::new(b) as Box<dyn WindowBackend>)
        }
        BackendKind::Hyprland => Some(Box::new(hyprland::HyprlandBackend)),
        BackendKind::Sway => Some(Box::new(sway::SwayBackend)),
        BackendKind::I3 => Some(Box::new(i3::I3Backend)),
        BackendKind::Gnome => {
            gnome::GnomeBackend::connect().map(|b| Box::new(b) as Box<dyn WindowBackend>)
        }
    }
}

pub fn default_probe_order() -> Vec<String> {
    DEFAULT_PROBE_ORDER.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDetector {
        supported: Vec<&'static str>,
    }

    impl MockDetector {
        fn supports(&self, name: &str) -> bool {
            self.supported.contains(&name)
        }
    }

    fn detect_with_mock(config: &Config, mock: &MockDetector) -> Option<String> {
        if let Some(forced) = config.backend.as_deref() {
            let forced = forced.trim().to_ascii_lowercase();
            if mock.supports(&forced) {
                return Some(forced);
            }
            return None;
        }

        for candidate in &config.backend_probe_order {
            let c = candidate.trim().to_ascii_lowercase();
            if mock.supports(&c) {
                return Some(c);
            }
        }
        None
    }

    #[test]
    fn forced_backend_takes_precedence() {
        let config = Config {
            backend: Some("gnome".into()),
            backend_probe_order: default_probe_order(),
            ..Config::default()
        };
        let mock = MockDetector {
            supported: vec!["kwin", "gnome"],
        };

        assert_eq!(detect_with_mock(&config, &mock), Some("gnome".into()));
    }

    #[test]
    fn probe_order_falls_through() {
        let config = Config {
            backend: None,
            backend_probe_order: vec!["gnome".into(), "sway".into(), "kwin".into()],
            ..Config::default()
        };
        let mock = MockDetector {
            supported: vec!["sway", "kwin"],
        };

        assert_eq!(detect_with_mock(&config, &mock), Some("sway".into()));
    }

    #[test]
    fn probe_order_returns_none_when_no_match() {
        let config = Config {
            backend: None,
            backend_probe_order: vec!["gnome".into(), "i3".into()],
            ..Config::default()
        };
        let mock = MockDetector { supported: vec![] };

        assert_eq!(detect_with_mock(&config, &mock), None);
    }
}
