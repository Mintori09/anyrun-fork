# KDE Settings

A plugin for [Anyrun](/../../) that lets you browse and open KDE System Settings panels from the runner.

## Features

- **Panel Browser**: Lists all available KDE System Settings panels via `kcmshell6`.
- **Fuzzy Search**: Search panel names with fuzzy matching.
- **Custom Entries**: Add your own settings shortcuts in the configuration.
- **Quick Access**: Opens settings directly with `kcmshell6`.

## Usage

Trigger the plugin using the configured prefix (default: `s `).

- Type `s ` to see all available KDE settings panels.
- Type a query to filter by name or description.
- Select a panel to open it.

## Dependencies

- `kcmshell6`: Lists and opens KDE System Settings panels (part of `kf6-kservice`).

## Configuration

The configuration is defined in `kde_setting.ron` in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "s ",

  // Whether to show all results immediately when the prefix is matched
  show_results_immediately: true,

  // Custom settings entries to include
  custom_settings: [
    KDESetting(
      name: "My Custom Setting",
      description: "kcm_my_module",
    ),
  ],
)
```

### Config Fields

- `prefix` (string): Prefix to trigger the plugin. Default: `"s "`.
- `show_results_immediately` (bool): Show all panels when prefix is matched without additional query.
- `custom_settings` (list): Additional settings entries to append to the auto-detected list.
