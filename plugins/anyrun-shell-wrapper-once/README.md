# Shell Wrapper

A versatile [Anyrun](https://github.com/Kirottu/anyrun) plugin that acts as a wrapper for various shell-based data sources and actions.

## Usage

The plugin uses configurable "scopes" with their own prefixes. When a prefix is matched, the plugin executes a `source` command to get a list of items, filters them based on your query, and executes an `on_select` command when an item is chosen.

## Configuration

The configuration is done in `shell_wrapper.ron` located in your Anyrun config directory.

```ron
Config(
  // Maximum number of entries to display
  max_entries: 10,
  
  // Enable logging for debugging (logs to ~/Desktop/shell_wrapper.log)
  show_log: false,

  // List of scopes
  scopes: [
    Scope(
      prefix: ":sh",
      // Command to generate the list of items (newline separated)
      source: "ls /usr/bin",
      // Command to run on selection. {} is replaced with the selected item.
      on_select: "notify-send 'Selected: {}'",
    )
  ],
)
```
