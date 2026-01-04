# Find Files

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to quickly find files using `fd`.

## Usage

Use the configured prefix (default: `:f`) followed by your search query. The plugin uses fuzzy matching (token-based) and executes `fd` to find results.

## Dependencies

- `fd`: Required for the search engine.
- `xdg-open` (or custom command): To open the selected file.

## Configuration

The configuration is done in `findfiles.ron` located in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: ":f",

  // The default command to run on selection
  // {} is replaced with the file path
  default_command: "xdg-open {}",

  // Maximum number of entries to display
  max_entries: 10,

  // Custom search scopes
  scopes: [
    SearchScope(
      path: "/home/user/Documents",
      prefix: ":d",
      excludes: ["temp"],
      command: Some("vlc {}"), // Optional custom command for this scope
    )
  ],

  // Filter options
  options: FilterRule(
    hidden: false, // Include hidden files
    patterns: [],  // Additional patterns (unused in current impl)
  ),
)
```
