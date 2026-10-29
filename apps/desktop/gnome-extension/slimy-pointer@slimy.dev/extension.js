import Gio from 'gi://Gio';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const INTERFACE = `
<node>
  <interface name="org.slimy.Pointer">
    <method name="GetPosition">
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
    </method>
    <method name="GetActiveApp">
      <arg type="s" direction="out" name="app"/>
    </method>
  </interface>
</node>`;

export default class SlimyPointer extends Extension {
  enable() {
    this._service = Gio.DBusExportedObject.wrapJSObject(INTERFACE, this);
    this._service.export(Gio.DBus.session, '/org/slimy/Pointer');
  }

  disable() {
    this._service?.unexport();
    this._service = null;
  }

  GetPosition() {
    const [x, y] = global.get_pointer();
    return [x, y];
  }

  /** The window class of the focused window, nothing more: no title, no content. Empty when nothing is focused. */
  GetActiveApp() {
    return global.display.focus_window?.get_wm_class() ?? '';
  }
}
