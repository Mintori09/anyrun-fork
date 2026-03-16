# Bluetooth Control Plugin for Anyrun

A plugin for [Anyrun](https://github.com/Kirottu/anyrun) that allows you to control Bluetooth adapters and devices directly from the runner.

## Features

- **Toggle Power**: Quickly enable or disable your Bluetooth adapter.
- **Manage Devices**: Connect or disconnect from paired Bluetooth devices.
- **Discovery**: Launch the system's device discovery wizard (e.g., `bluedevil-wizard`).
- **Dynamic Status**: Shows connection status and device addresses.

## Usage

Trigger the plugin using the configured prefix (default: `bt `).

- Type `bt ` to see the list of adapter actions and paired devices.
- Select "Enable Bluetooth" to power on the adapter.
- Select a paired device to toggle its connection state.
- Select "Find and match new devices" to start a discovery wizard.

## Dependencies

- `bluez`: The Linux Bluetooth stack.
- `bluedevil-wizard` (Optional): For the discovery action on KDE systems.

## Configuration

The configuration is defined in `libbluetooth.so.ron` (the filename matches the compiled library name) in your Anyrun configuration directory.

```ron
Config(
  // The prefix to trigger this plugin
  prefix: "bt ",
)
```
