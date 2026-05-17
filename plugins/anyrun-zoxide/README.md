# Zoxide Fuzzy

An [Anyrun](https://github.com/Kirottu/anyrun) plugin for quickly jumping to directories tracked by `zoxide`.

## Usage

Use the configured prefix (default: `z `) followed by your search query. The plugin fuzzy matches against your `zoxide` database. Selecting a result opens a new terminal window in that directory.

## Dependencies

- `zoxide`: Required to query the directory database.
- A supported terminal emulator: Alacritty, Foot, Kitty, WezTerm, or Ghostty (auto-detected).

## Configuration

The configuration is done in `zoxide.ron` located in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger zoxide search
  prefix: "z ",

  // Maximum number of entries to display
  max_entries: 5,

  // Cache time-to-live in seconds (how often to refresh the zoxide database)
  cache_ttl_secs: 30,
)
```
