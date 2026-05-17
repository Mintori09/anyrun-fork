# Window Switcher

A plugin for [Anyrun](/../../) that lets you fuzzy-search and focus open windows across workspaces.

## Features

- **Multi-Backend**: Supports Hyprland and niri compositors (auto-detected).
- **Workspace Labels**: Shows workspace information for each window (niri).
- **App Name + Title Search**: Fuzzy matches against application name, app ID, and window title.
- **Exclusion Filter**: Exclude specific app classes from results.
- **Caching**: Efficiently caches window lists to reduce IPC overhead.

## Usage

Trigger the plugin using the configured prefix (default: `w `).

- Type `w ` to see all open windows.
- Type additional text to fuzzy-filter by application name or window title.
- Select a window to focus it.

## Dependencies

- **Hyprland**: Uses `hyprctl` (included with Hyprland).
- **niri**: Uses `niri-ipc` (included with niri).

## Configuration

The configuration is defined in `window_switcher.ron` in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "w ",

  // Maximum number of entries to display
  max_entries: 10,

  // Cache time-to-live in seconds (minimum interval between window list refreshes)
  cache_ttl_secs: 2,

  // Window classes to exclude from results (matched against app_id and app_name)
  exclude_classes: [
    "firefox",
    "org.gnome.Nautilus",
  ],
)
```

### Config Fields

- `prefix` (string): Prefix to trigger the plugin. Default: `"w "`.
- `max_entries` (usize): Maximum results shown. Default: `10`.
- `cache_ttl_secs` (u64): How long to cache the window list before refreshing. Default: `2`.
- `exclude_classes` (list of strings): App IDs or names to exclude from results.
