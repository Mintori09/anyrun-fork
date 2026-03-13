# Symbols

Look up unicode symbols and custom user defined symbols. This plugin also includes a large collection of icons (e.g. from Coder, Devicon, etc.).

## Usage

Simply search for the symbol's name. Use the prefix if configured.

## Configuration

```ron
// <Anyrun config dir>/symbols.ron
Config(
  // The prefix that the search needs to begin with to yield symbol results.
  // Set to an empty string to always search for symbols.
  prefix: "i",
  // Custom user defined symbols to be included along the unicode symbols
  symbols: {
    // "name": "text to be copied"
    "shrug": "¯\\_(ツ)_/¯",
  },
  // Maximum number of entries to show
  max_entries: 10,
)
```
