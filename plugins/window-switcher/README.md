# Window Switcher

A plugin for [Anyrun](/../../) that lets you fuzzy-search and focus open windows across workspaces.

## Features

- Multi-backend support with auto-detection.
- Works with KWin, Hyprland, niri, sway, i3, and GNOME Wayland (via extension bridge).
- Fuzzy matches application name, app ID, and window title.
- Optional exclusion list for classes/IDs.
- Cached window list to reduce backend IPC calls.

## Usage

Trigger the plugin using the configured prefix (default: `w `).

- Type `w ` to see all windows if `show_results_immediately` is enabled.
- Type additional text to fuzzy-filter by app name/title.
- Select a result to focus that window.

## Backend Dependencies

- KWin: `kdotool`
- Hyprland: `hyprctl`
- niri: niri IPC socket (`NIRI_SOCKET`)
- sway: `swaymsg`
- i3: `i3-msg`
- GNOME Wayland: GNOME Shell extension in this directory

## GNOME Wayland Setup

Install the extension from `plugins/window-switcher/gnome-extension/anyrun-window-switcher@anyrun` to:

- `~/.local/share/gnome-shell/extensions/anyrun-window-switcher@anyrun`

Then reload GNOME Shell/session and enable it:

```bash
gnome-extensions enable anyrun-window-switcher@anyrun
```

The extension exposes:

- Service: `org.anyrun.WindowSwitcher`
- Path: `/org/anyrun/WindowSwitcher`
- Interface: `org.anyrun.WindowSwitcher1`

Methods:

- `ListWindows() -> s` (JSON array of `id`, `title`, `app_id`, `workspace`)
- `FocusWindow(id: s) -> b`

If a GNOME session is detected but this D-Bus service is missing, the plugin logs a warning and continues probing other backends.

## Configuration

The configuration is defined in `window_switcher.ron` in your Anyrun config directory.

```ron
Config(
  // Prefix to trigger this plugin
  prefix: "w ",

  // Maximum number of entries to display
  max_entries: 15,

  // Show results immediately when only prefix is typed
  show_results_immediately: false,

  // Cache time-to-live in seconds
  cache_ttl_secs: 5,

  // Force a backend by name, or use None for auto-detection
  backend: None,

  // Probe order used when backend is None
  backend_probe_order: ["kwin", "niri", "hyprland", "sway", "i3", "gnome"],

  // Classes/app IDs to exclude from results
  exclude_classes: [
    "plasmashell",
    "org.gnome.Nautilus",
  ],
)
```

### Config Fields

- `prefix` (`String`): Plugin trigger prefix. Default: `"w "`.
- `max_entries` (`usize`): Maximum results. Default: `15`.
- `show_results_immediately` (`bool`): Show windows on empty query. Default: `false`.
- `cache_ttl_secs` (`u64`): Window list cache TTL in seconds. Default: `5`.
- `backend` (`Option<String>`): Force one backend (`kwin`, `niri`, `hyprland`, `sway`, `i3`, `gnome`). Default: `None`.
- `backend_probe_order` (`Vec<String>`): Auto-detect probe order when `backend` is `None`.
- `exclude_classes` (`Vec<String>`): Classes or app IDs to exclude.
