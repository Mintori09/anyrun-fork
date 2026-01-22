# Anyrun Universal Action

An [Anyrun](https://github.com/Kirottu/anyrun) plugin that provides context-aware actions based on your clipboard content. It detects types like Code (with language scoring), JSON data, Colors, and System paths to show only relevant tools.

## Features

- **Contextual Awareness**: Automatically filters actions based on whether your clipboard contains a URL, Path, IP Address, Color, Code, or JSON.
- **Any-Language Support**: Define generic code actions (like formatting or gist creation) that apply to all detected programming languages.
- **Smart Scoring**: Priority-based sorting for specific programming languages.
- **Custom Validators**: Support for internal Rust functions to validate clipboard content before showing an action.

## Usage

Use the configured prefix (default: `""` or `:ua `) followed by a query to filter actions.

1. Copy something to your clipboard (e.g., a JSON string or a CSS Hex color).
2. Open Anyrun.
3. Type the action name (e.g., "pretty" for JSON or "preview" for colors).
4. Press **Enter** to execute the command.

## Dependencies

- `wl-clipboard`: Required to interact with the Wayland clipboard (`wl-copy`, `wl-paste`).
- `sh`: Required for executing shell commands.
- Specific commands used in your config (e.g., `jq`, `yq`, `curl`, `prettier`, `convert`).

## Configuration

The configuration is defined in `universal-action.ron`. This plugin uses a specialized enum system for high-precision matching.

### Input Categories

Actions are mapped to the following categories:

- `Code { lang: String, score: f64 }`: Use `lang: "any"` for generic code tools.
- `Data { format: String }`: e.g., `format: "json"`.
- `System { kind: String }`: e.g., `kind: "URL"`, `"Path"`, or `"IpAddress"`.
- `Design { kind: String }`: e.g., `kind: "Color"`.
- `PlainText`: Standard text transformations.
- `All`: Always visible.

### Example `universal-action.ron`

```ron
Config(
  prefix: "",
  max_entries: 10,
  actions: [
    // Matches only when JSON is detected
    (
      name: "JSON Pretty Print",
      category: Data(format: "json"),
      target: Shell("echo '{clip}' | jq ."),
    ),

    // Matches any detected programming language
    (
      name: "Count Lines",
      category: Code(lang: "any", score: 0.0),
      target: Shell("echo '{clip}' | wc -l | wl-copy"),
    ),

    // Matches System Paths
    (
      name: "Open in File Manager",
      category: System(kind: "Path"),
      target: Shell("xdg-open \"$(dirname '{clip}')\""),
    ),

    // Always available
    (
      name: "Ask Gemini",
      category: All,
      target: Shell("wl-paste | rust-script gemini-trigger.rs"),
    ),
  ],
)

```

## Internal Architecture

The plugin evaluates matches using the following Rust logic:

- **Category Match**: Checks if the clipboard content type matches the action's required category. `lang: "any"` acts as a wildcard for all `Code` types.
- **Validator**: If a `validator` function is provided in the `UniversalAction` struct, it must return `true` for the action to appear.
- **Execution**: Commands are executed via `sh -c`, replacing `{clip}` with the current clipboard content.
