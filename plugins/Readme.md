# Anyrun-Plugins

A collection of custom plugins for the [Anyrun](https://github.com/Kirottu/anyrun) runner.

## Available Plugins

| Plugin | Description | Key Dependencies |
| :--- | :--- | :--- |
| [Applications](anyrun-applications) | Desktop entry runner with fuzzy matching | None |
| [Calc](anyrun-calc) | High-performance calculator | `qalc` (libqalculate) |
| [Find Files](anyrun-findfiles) | Fast file searcher | `fd` |
| [Shell Wrapper](anyrun-shell-wrapper) | Custom shell command executor | None |
| [Universal Action](anyrun-universal-action) | Contextual actions on clipboard data | `wl-paste` |
| [Web Search](anyrun-websearch) | Search the web with custom engines | Browser |
| [Zoxide](anyrun-zoxide) | Fuzzy jump to Zoxide directories | `zoxide` |

## Installation

### Prerequisites

- [Anyrun](https://github.com/Kirottu/anyrun) installed and configured.
- [Rust](https://www.rust-lang.org/) toolchain.

### Building

1. Clone this repository:
   ```bash
   git clone https://github.com/Mintori09/anyrun-findfiles.git
   cd anyrun-findfiles
   ```
2. Build the plugins:
   ```bash
   cargo build --release
   ```
3. Copy the compiled `.so` files to your Anyrun plugin directory (usually `~/.config/anyrun/plugins/`):
   ```bash
   cp target/release/libanyrun_*.so ~/.config/anyrun/plugins/
   ```

## Configuration

Each plugin has its own configuration file (usually in `.ron` format) located in your Anyrun configuration directory (typically `~/.config/anyrun/`). See the individual plugin READMEs for specific configuration options.
