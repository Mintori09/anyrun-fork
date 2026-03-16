# Browser Tabs Plugin for Anyrun

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to search and switch between open browser tabs using `brotab`.

## Features

- **Search Tabs**: Fuzzy search through titles and URLs of open browser tabs.
- **Fast Switching**: Instantly switch to the selected tab.
- **Multi-Browser Support**: Works with any browser supported by `brotab` (Firefox, Chrome, etc.).
- **Caching**: Efficiently caches tab lists to reduce overhead.

## Usage

Trigger the plugin using the configured prefix (default: `tab `).

- Type `tab ` followed by your query to search for open tabs.
- Select a tab and press **Enter** to switch to it.

## Dependencies

- `brotab`: Required to list and activate browser tabs.
- Browser extension for `brotab`: Must be installed in your browser.

## Configuration

The configuration is defined in `browser.ron` in your Anyrun configuration directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "tab ",

  // Path to the brotab binary
  source: "~/.local/bin/brotab",

  // Maximum number of entries to display
  max_entries: 10,

  // Cache time-to-live in seconds
  cache_ttl_secs: 5,
)
```
