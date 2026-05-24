import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

const SERVICE = 'org.anyrun.WindowSwitcher';
const PATH = '/org/anyrun/WindowSwitcher';
const IFACE = 'org.anyrun.WindowSwitcher1';

const XML = `<node>
  <interface name="${IFACE}">
    <method name="ListWindows">
      <arg direction="out" type="s" name="windows"/>
    </method>
    <method name="FocusWindow">
      <arg direction="in" type="s" name="id"/>
      <arg direction="out" type="b" name="ok"/>
    </method>
  </interface>
</node>`;

class WindowSwitcherBridge {
  constructor() {
    this._dbus = Gio.DBusExportedObject.wrapJSObject(XML, this);
    this._dbus.export(Gio.DBus.session, PATH);
    this._owner = Gio.bus_own_name_on_connection(
      Gio.DBus.session,
      SERVICE,
      Gio.BusNameOwnerFlags.REPLACE,
      null,
      null,
    );
  }

  destroy() {
    if (this._owner) {
      Gio.bus_unown_name(this._owner);
      this._owner = null;
    }
    if (this._dbus) {
      this._dbus.unexport();
      this._dbus = null;
    }
  }

  _windowToObject(window) {
    const app = Shell.WindowTracker.get_default().get_window_app(window);
    const appId = app ? app.get_id() : null;
    const workspace = window.get_workspace();
    const workspaceIndex = workspace ? workspace.index() + 1 : null;

    return {
      id: String(window.get_id()),
      title: window.get_title() ?? '',
      app_id: appId,
      workspace: workspaceIndex ? `Workspace ${workspaceIndex}` : null,
    };
  }

  ListWindows() {
    const windows = global.display
      .get_tab_list(Meta.TabList.NORMAL_ALL, null)
      .map(window => this._windowToObject(window));

    return [JSON.stringify(windows)];
  }

  FocusWindow(id) {
    const numericId = Number(id);
    if (Number.isNaN(numericId)) {
      return [false];
    }

    const window = global.display
      .get_tab_list(Meta.TabList.NORMAL_ALL, null)
      .find(w => w.get_id() === numericId);

    if (!window) {
      return [false];
    }

    const timestamp = global.get_current_time();
    const workspace = window.get_workspace();
    if (workspace) {
      workspace.activate(timestamp);
    }
    window.activate(timestamp);
    return [true];
  }
}

let bridge = null;

export default class Extension {
  enable() {
    bridge = new WindowSwitcherBridge();
  }

  disable() {
    if (bridge) {
      bridge.destroy();
      bridge = null;
    }
  }
}
