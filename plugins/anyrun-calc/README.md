# Calculator

A high-performance calculator plugin for [Anyrun](https://github.com/Kirottu/anyrun) powered by `qalc`.

## Usage

Use the configured prefix (default: `=`) followed by your mathematical expression. The result will be displayed as a match. Selecting the result will copy it to your clipboard using `wl-copy`.

## Dependencies

- `qalc` (part of [libqalculate](https://github.com/Qalculate/libqalculate)): Required for calculations.
- `wl-copy`: Required for copying results to the clipboard.

## Configuration

The configuration is done in `calc.ron` located in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger the calculator
  prefix: "=",
)
```
