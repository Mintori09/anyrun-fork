# Anyrun Plugin Template

A generic template for creating new [Anyrun](https://github.com/Kirottu/anyrun) plugins. This provides a basic structure with configuration handling, state management, and fuzzy searching.

## Features

- **Configuration Loading**: Automatically reads configuration from a `.ron` file.
- **Fuzzy Search**: Implements a basic fuzzy search using `fuzzy-matcher`.
- **Boilerplate**: Ready-to-use `init`, `info`, `get_matches`, and `handler` functions.

## Usage

Use this as a starting point for your own plugins.

1. Copy the `plugin-sample` directory.
2. Rename the directory and update `Cargo.toml`.
3. Implement `fetch_data` and `execute_action` in `src/lib.rs`.
4. Update `PLUGIN_NAME` and `info()` metadata.

## Configuration

The default configuration is defined in the plugin's `.ron` file.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "ex ",

  // Maximum number of entries to display
  max_entries: 5,

  // Whether to show all results when the query is empty
  show_results_immediately: false,
)
```
