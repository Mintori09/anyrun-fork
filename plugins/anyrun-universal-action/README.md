# Universal Action

An [Anyrun](https://github.com/Kirottu/anyrun) plugin that provides contextual actions based on your current clipboard content.

## Usage

Use the configured prefix (default: `:ua `) followed by a query to filter available actions. The plugin automatically detects the type of content in your clipboard (URL, File, Text) and shows relevant actions.

## Dependencies

- `wl-paste`: Required to get the clipboard content.

## Configuration

The configuration is done in `universal-action.ron` located in your Anyrun config directory.

```ron
Config(
  // The prefix to trigger universal actions
  prefix: ":ua ",

  // Enable logging (logs to ~/Desktop/universal-action.log)
  show_log: false,

  // List of actions
  actions: [
    Action(
      name: "Open in Browser",
      command: "xdg-open {clip}",
      data_type: "Url", // Options: "Url", "File", "Text", "Nothing", "Any"
    ),
    Action(
      name: "Copy to Phone",
      command: "kdeconnect-cli --share {clip}",
      data_type: "Any",
    ),
  ],
)
```
