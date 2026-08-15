// SPDX-License-Identifier: GPL-3.0-or-later

import Meta from "gi://Meta";
import Shell from "gi://Shell";
import St from "gi://St";

import * as DND from "resource:///org/gnome/shell/ui/dnd.js";
import * as Layout from "resource:///org/gnome/shell/ui/layout.js";
import * as Main from "resource:///org/gnome/shell/ui/main.js";
import * as PopupMenu from "resource:///org/gnome/shell/ui/popupMenu.js";

import {chooseMonitorForConnector} from "./model.js";

// GNOME Shell 50 compatibility boundary. No other module imports Shell UI resources
// or reads the Shell global object directly.

export function appSystem() {
  return Shell.AppSystem.get_default();
}

export function addChrome(actor, options) {
  Main.layoutManager.addChrome(actor, options);
}

export function removeChrome(actor) {
  Main.layoutManager.removeChrome(actor);
}

export function connectShellSignal(source, signal, callback) {
  const object = {
    display: global.display,
    workspaceManager: global.workspace_manager,
    layoutManager: Main.layoutManager,
    appearance: St.Settings.get(),
  }[source];
  if (!object)
    throw new Error(`Unknown Shell signal source ${source}`);
  const id = object.connect(signal, callback);
  return () => object.disconnect(id);
}

export function monitorGeometry(index) {
  return Main.layoutManager.monitors[index] ?? Main.layoutManager.primaryMonitor;
}

export function primaryMonitorIndex() {
  return Main.layoutManager.primaryIndex;
}

export function monitorWorkArea(index) {
  return Main.layoutManager.getWorkAreaForMonitor(index);
}

export function monitorIndexForConnector(connector, fallback) {
  return chooseMonitorForConnector(logicalMonitorDescriptors(), connector, fallback);
}

export function monitorConnectorIds() {
  return logicalMonitorDescriptors().flatMap(logical => logical.connectors);
}

export function monitorIsFullscreen(index) {
  return global.display.get_monitor_in_fullscreen(index);
}

export function activeWorkspaceIndex() {
  return global.workspace_manager.get_active_workspace_index();
}

export function dodgeWindowDescriptors() {
  return global.get_window_actors().map(actor => {
    const window = actor.meta_window;
    const rect = window.get_frame_rect();
    return {
      rect: {x: rect.x, y: rect.y, width: rect.width, height: rect.height},
      workspace: window.get_workspace()?.index() ?? -1,
      monitor: window.get_monitor(),
      normal: !window.minimized && window.get_window_type() === Meta.WindowType.NORMAL,
    };
  });
}

export function focusedWindow() {
  return global.display.focus_window;
}

export function currentTime() {
  return global.get_current_time();
}

export function createAppLaunchContext() {
  return global.create_app_launch_context(currentTime(), -1);
}

export function keyFocus() {
  return global.stage.get_key_focus();
}

export function clearKeyFocus() {
  global.stage.set_key_focus(null);
}

export function animationDuration(milliseconds = 160) {
  const settings = St.Settings.get();
  return settings.enable_animations && !settings.reduced_motion ? milliseconds : 0;
}

export function systemPrefersLight() {
  const scheme = St.Settings.get().color_scheme;
  return St.SystemColorScheme && scheme === St.SystemColorScheme.PREFER_LIGHT;
}

export function dragMotionResult(accepted) {
  return accepted ? DND.DragMotionResult.MOVE_DROP : DND.DragMotionResult.NO_DROP;
}

export function makeDraggable(actor) {
  return DND.makeDraggable(actor);
}

export function createPopupMenu(sourceActor, edge) {
  const side = {
    top: St.Side.BOTTOM,
    bottom: St.Side.TOP,
    left: St.Side.RIGHT,
    right: St.Side.LEFT,
  }[edge];
  return new PopupMenu.PopupMenu(sourceActor, 0.5, side);
}

export function createPopupMenuItem(label) {
  return new PopupMenu.PopupMenuItem(label);
}

export function addMenuActor(actor) {
  Main.uiGroup.add_child(actor);
}

export function removeKeybinding(name) {
  Main.wm.removeKeybinding(name);
}

export function addKeybinding(name, settings, callback) {
  Main.wm.addKeybinding(
    name,
    settings,
    Meta.KeyBindingFlags.NONE,
    Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
    callback,
  );
}

export function createPressureBarrier(monitor, edge, callback) {
  if (!Layout.PressureBarrier)
    return null;
  const pressure = new Layout.PressureBarrier(100, 1000, Shell.ActionMode.NORMAL);
  const coordinates = barrierCoordinates(monitor, edge);
  const barrier = new Meta.Barrier({
    backend: global.backend,
    ...coordinates,
  });
  pressure.addBarrier(barrier);
  const triggerId = pressure.connect("trigger", callback);
  return {
    destroy() {
      pressure.disconnect(triggerId);
      pressure.removeBarrier(barrier);
      barrier.destroy();
      pressure.destroy();
    },
  };
}

function logicalMonitorDescriptors() {
  try {
    return global.backend.get_monitor_manager().get_logical_monitors().map(logical => ({
      number: logical.get_number(),
      connectors: logical.get_monitors().map(monitor => monitor.get_connector()),
    }));
  } catch (error) {
    console.warn(`Prismatic could not enumerate monitor connectors: ${error.message}`);
    return [];
  }
}

function barrierCoordinates(monitor, edge) {
  switch (edge) {
  case "top":
    return {x1: monitor.x, x2: monitor.x + monitor.width, y1: monitor.y, y2: monitor.y, directions: Meta.BarrierDirection.POSITIVE_Y};
  case "left":
    return {x1: monitor.x, x2: monitor.x, y1: monitor.y, y2: monitor.y + monitor.height, directions: Meta.BarrierDirection.POSITIVE_X};
  case "right":
    return {x1: monitor.x + monitor.width, x2: monitor.x + monitor.width, y1: monitor.y, y2: monitor.y + monitor.height, directions: Meta.BarrierDirection.NEGATIVE_X};
  default:
    return {x1: monitor.x, x2: monitor.x + monitor.width, y1: monitor.y + monitor.height, y2: monitor.y + monitor.height, directions: Meta.BarrierDirection.NEGATIVE_Y};
  }
}
