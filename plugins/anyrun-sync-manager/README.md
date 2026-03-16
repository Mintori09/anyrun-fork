# Sync Manager Plugin for Anyrun

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to quickly trigger custom sync scripts or shell commands.

## Features

- **Custom Scopes**: Define multiple sync tasks or scripts in your configuration.
- **Fuzzy Search**: Search through your configured sync tasks by name or script path.
- **Easy Execution**: Runs the selected script via `sh`.

## Usage

Trigger the plugin using the configured prefix (default: `sy `).

- Type `sy ` to see the list of configured sync tasks.
- Type your query to filter the tasks.
- Press **Enter** to execute the selected script.

## Configuration

The configuration is defined in `sync_manager.ron` in your Anyrun configuration directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "sy ",

  // Maximum number of entries to display
  max_entries: 10,

  // List of sync tasks
  scopes: [
    (
      name: "Sync Dotfiles",
      source: "~/bin/sync-dotfiles.sh",
      icon: "system-run-symbolic",
    ),
    (
      name: "Update Packages",
      source: "yay -Syu",
      icon: "software-update-available-symbolic",
    ),
  ],
)
```

### SyncManager Fields

- `name`: The display name of the task.
- `source`: The shell command or path to the script to execute.
- `icon`: The icon to display for this task.
