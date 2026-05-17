# Applications

A desktop entry runner for [Anyrun](/../../) with fuzzy search, desktop actions, and system commands.

## Features

- **Fuzzy Search**: Matches application name, description, and keywords with Vietnamese accent stripping.
- **Desktop Actions**: Supports application-specific actions (e.g. "New Window" for browsers).
- **System Actions**: Built-in shutdown, reboot, lock screen, suspend, and logout.
- **Terminal Apps**: Automatically runs terminal applications in the configured terminal emulator.
- **Pre-process Script**: Optionally transform exec commands via an external script.

## Usage

This plugin activates automatically without a prefix. Simply type the application name.

## Dependencies

- Standard `.desktop` files (in `/usr/share/applications` and `~/.local/share/applications`).
- A terminal emulator (one of: Alacritty, Foot, Kitty, WezTerm, Ghostty) — auto-detected.

## Configuration

The configuration is defined in `applications.ron` in your Anyrun config directory.

```ron
Config(
  // Show desktop action entries (e.g. "New Window", "New Private Window")
  desktop_actions: true,

  // Maximum entries to display
  max_entries: 5,

  // Hide application descriptions
  hide_description: false,

  // Terminal emulator for terminal-requiring apps
  terminal: Some(Terminal(
    command: "alacritty",
    args: "-e {}",
  )),

  // Optional script to pre-process exec commands before running.
  // Called with: <script> <term|no-term> <original_exec>
  // stdout is used as the new exec command.
  // preprocess_exec_script: Some("/path/to/preprocess.sh"),
)
```

### Config Fields

- `desktop_actions` (bool): Show desktop action entries. Default: `false`.
- `max_entries` (usize): Maximum matches displayed. Default: `5`.
- `hide_description` (bool): Hide the description text under each match. Default: `false`.
- `terminal` (Option): Terminal emulator config. If `None`, auto-detected from available terminals.
- `preprocess_exec_script` (Option): Path to an external script that can modify exec commands before running.
