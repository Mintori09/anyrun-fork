pub mod hyprland;
pub mod kwin;
pub mod niri;

use anyrun_helper::window::WindowBackend;
use std::process::Command;

pub fn detect_backend() -> Option<Box<dyn WindowBackend>> {
    if Command::new("kdotool").arg("--version").output().is_ok() {
        return Some(Box::new(kwin::KWinBackend));
    }

    if std::env::var("NIRI_SOCKET").is_ok() {
        return niri::NiriBackend::connect().map(|b| Box::new(b) as Box<dyn WindowBackend>);
    }

    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return Some(Box::new(hyprland::HyprlandBackend));
    }

    None
}
