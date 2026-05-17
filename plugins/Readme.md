# Anyrun Plugins

A collection of plugins for the [Anyrun](https://github.com/Kirottu/anyrun) application launcher.

## Plugin Index

| Plugin | File | Prefix | Description |
| :--- | :--- | :--- | :--- |
| [Applications](anyrun-applications) | `libapplications.so` | (none) | Desktop entry search with fuzzy matching and system actions |
| [Bluetooth](anyrun-bluetooth) | `libbluetooth.so` | `bt ` | Bluetooth adapter and device control |
| [Browser Tabs](anyrun-browser) | `libbrowser.so` | `tab ` | Search and switch browser tabs via brotab |
| [Calc](anyrun-calc) | `libcalc.so` | `=` | Calculator using libqalculate |
| [Dictionary](dictionary) | `libdictionary.so` | `:def` | Word definitions via Free Dictionary API |
| [Find Files](anyrun-findfiles) | `libfindfiles.so` | `fd ` | File search using fd with custom scopes |
| [KDE Clipboard](kde-clipboard) | `libkde_clipboard.so` | `hf ` | Klipper clipboard history search |
| [KDE Settings](anyrun-kde-setting) | `libkde_setting.so` | `s ` | KDE System Settings panel browser |
| [Kidex](kidex) | `libkidex.so` | (none) | File search via Kidex index daemon |
| [Kill Port](kill-port) | `libkill_port.so` | `port ` | Find and kill processes on network ports |
| [Nix-run](nix-run) | `libnix_run.so` | `:nr ` | Run nixpkgs applications via `nix run` |
| [Port Killer](kill-port) | `libkill_port.so` | `port ` | Kill processes listening on specific ports |
| [Randr](randr) | `librandr.so` | `:dp` | Monitor configuration (Hyprland) |
| [Shell Wrapper](anyrun-shell-wrapper) | `libshell_wrapper.so` | configurable | Execute commands from list sources with fuzzy filtering |
| [Shell Wrapper Once](anyrun-shell-wrapper-once) | `libshell_wrapper_once.so` | configurable | One-shot command execution with user input |
| [Stdin](stdin) | `libstdin.so` | (none) | dmenu-like fuzzy selector from stdin |
| [Symbols](symbols) | `libsymbols.so` | `icon` | Unicode and icon symbol lookup |
| [Sync Manager](anyrun-sync-manager) | `libsync_manager.so` | `sy ` | Run custom sync scripts and commands |
| [Translate](translate) | `libtranslate.so` | `:` | Google Translate integration |
| [Universal Action](anyrun-universal-action) | `libuniversal_action.so` | `:ua ` | Context-aware actions on clipboard content |
| [Web Search](anyrun-websearch) | `libwebsearch.so` | configurable | Multi-engine web search |
| [Window Switcher](window-switcher) | `libwindow_switcher.so` | `w ` | Focus open windows (Hyprland/niri) |
| [Zoxide](anyrun-zoxide) | `libzoxide.so` | `z ` | Fuzzy directory jumping via zoxide |
| [Sample Template](plugin-sample) | — | `ex ` | Plugin development starter template |

## Installation

### Prerequisites

- [Anyrun](/../../) installed and configured
- [Rust](https://www.rust-lang.org/) toolchain

### Building

From the repository root:

```bash
cargo build --release --workspace
```

Or build individual plugins:

```bash
cargo build --release -p anyrun-applications
```

The compiled `.so` files are placed in `target/release/`. Copy them to your Anyrun plugins directory:

```bash
cp target/release/*.so ~/.config/anyrun/plugins/
```

## Configuration

Each plugin has its own configuration file (`.ron` format) in the Anyrun config directory (`~/.config/anyrun/`). See individual plugin READMEs for specific options.
