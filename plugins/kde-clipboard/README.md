# KDE Klipper Plugin for Anyrun

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to search through and copy items from your KDE Klipper clipboard history.

## Features

- **History Search**: Fuzzy search through your Klipper clipboard history.
- **DBus Integration**: Communicates directly with `org.kde.klipper` via DBus.
- **Fast Copying**: Uses `wl-copy` to instantly place selected items back into your clipboard.
- **Caching**: Caches history items to stay responsive while searching.

## Usage

Trigger the plugin using the configured prefix (default: `hf `).

- Type `hf ` followed by your query to search for clipboard history items.
- Select an item and press **Enter** to copy it to your clipboard.

## Dependencies

- KDE Plasma with **Klipper** enabled.
- `wl-clipboard`: Required to copy items back to the system clipboard on Wayland.

## Configuration

The configuration is defined in `klipper.ron` in your Anyrun configuration directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "hf ",

  // Maximum number of entries to display
  max_entries: 10,

  // Maximum characters shown in preview title (one-line, then "...")
  preview_max_chars: 120,
)
```
