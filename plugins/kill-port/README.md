# Port Killer Plugin for Anyrun

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to identify and terminate processes listening on specific network ports.

## Features

- **Active Port Detection**: Uses `ss` to find all processes currently listening on TCP/UDP ports.
- **Process Info**: Displays the port number, protocol, and the name of the associated process.
- **Quick Termination**: Instantly kills the process associated with a port using `kill -9`.
- **Fuzzy Search**: Search by port number or process name.

## Usage

Trigger the plugin using the configured prefix (default: `port `).

- Type `port ` followed by a port number or process name to filter results.
- Select a port and press **Enter** to kill the associated process.

## Dependencies

- `iproute2` (specifically the `ss` command): Required to list active network sockets.
- `kill`: Required to terminate processes.

## Configuration

The configuration is defined in `port_killer.ron` in your Anyrun configuration directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "port ",

  // Maximum number of entries to display
  max_entries: 10,

  // Cache time-to-live in seconds
  cache_ttl_secs: 1,
)
```
