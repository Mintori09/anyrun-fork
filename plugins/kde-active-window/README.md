# KDE Window Switcher Plugin for Anyrun

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to quickly search and switch to open windows on KDE Plasma.

## Features

- **Window Search**: Search through open windows by their class name.
- **Fast Switching**: Uses `kdotool` to instantly focus the selected window.
- **Caching**: Caches the window list for a configurable duration to stay responsive.

## Usage

Trigger the plugin using the configured prefix (default: `fo `).

- Type `fo ` followed by your query to search for open windows.
- Select a window and press **Enter** to switch focus to it.

## Dependencies

- `kdotool`: Required to search for and focus windows on KDE.

## Configuration

The configuration is defined in `kde_window_switcher.ron` in your Anyrun configuration directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "fo ",

  // Maximum number of entries to display
  max_entries: 10,

  // Cache time-to-live in seconds
  cache_ttl_secs: 2,
)
```
