# Zoxide Fuzzy

An [Anyrun](https://github.com/Kirottu/anyrun) plugin for quickly jumping to directories tracked by `zoxide`.

## Usage

Use the configured prefix (default: `zo `) followed by your search query. The plugin fuzzy matches against your `zoxide` database. Selecting a result will open a new `kitty` terminal window in that directory.

## Dependencies

- `zoxide`: Required to query the directory database.
- `kitty`: Used to open the selected directory.

## Configuration

The configuration is done in `zoxide.ron` located in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger zoxide search
  prefix: "zo ",

  // Maximum number of entries to display
  max_entries: 5,
)
```
