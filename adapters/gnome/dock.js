// SPDX-License-Identifier: GPL-3.0-or-later

import Clutter from "gi://Clutter";
import GLib from "gi://GLib";
import St from "gi://St";

import {renderDockContent} from "./dockContent.js";
import {buildAppModel, calculateGeometry, isInstalledDesktopUri, shouldDodge} from "./model.js";
import * as ShellCompat from "./shellCompat.js";

export class DockController {
  constructor(client, config) {
    this._client = client;
    this._config = config;
    this._signals = [];
    this._shellSignalCleanups = [];
    this._hideTimer = 0;
    this._revealTimer = 0;
    this._barrier = null;
    this._fullscreenOverride = false;
    this._buttons = [];
    this._appSystem = ShellCompat.appSystem();
    this.actor = new St.BoxLayout({
      name: "prismaticDock",
      style_class: "prismatic-dock",
      reactive: true,
      can_focus: true,
      track_hover: true,
    });
    this.actor._delegate = this;
    this._scrollView = new St.ScrollView({
      overlay_scrollbars: true,
      enable_mouse_scrolling: true,
      enable_touch_scrolling: true,
      x_expand: true,
      y_expand: true,
    });
    this._content = new St.BoxLayout({style_class: "prismatic-content"});
    this._scrollView.set_child(this._content);
    this.actor.add_child(this._scrollView);
    this._connect(this._appSystem, "app-state-changed", () => this.refresh());
    this._connectShell("display", "window-created", () => this.refresh());
    this._connectShell("display", "restacked", () => this._updateVisibility());
    this._connectShell("workspaceManager", "active-workspace-changed", () => {
      this.refresh();
      this._updateVisibility();
    });
    this._connectShell("layoutManager", "monitors-changed", () => {
      this._rebuildBarrier();
      this._layout();
    });
    this._connectShell("display", "in-fullscreen-changed", () => this._updateVisibility());
    this._connectShell("appearance", "notify::color-scheme", () => this._applyAppearance());
    this._connectShell("appearance", "notify::accent-color", () => this.refresh());
    this._connect(this.actor, "enter-event", () => this.show());
    this._connect(this.actor, "leave-event", () => this._scheduleHide());
    this._connect(this.actor, "key-press-event", (_actor, event) => this._onKeyPress(event));
    ShellCompat.addChrome(this.actor, this._chromeOptions());
    this.applyConfig(config);
  }

  destroy() {
    this._destroyed = true;
    if (this._hideTimer)
      GLib.source_remove(this._hideTimer);
    if (this._revealTimer)
      GLib.source_remove(this._revealTimer);
    this._barrier?.destroy();
    this._barrier = null;
    for (const [object, id] of this._signals)
      object.disconnect(id);
    this._signals = [];
    for (const disconnect of this._shellSignalCleanups)
      disconnect();
    this._shellSignalCleanups = [];
    for (const button of this._buttons)
      button.menu?.destroy();
    this._buttons = [];
    ShellCompat.removeChrome(this.actor);
    this.actor.destroy();
  }

  applyConfig(config) {
    this._config = config;
    const vertical = ["left", "right"].includes(config.geometry.edge);
    this.actor.vertical = vertical;
    this._content.vertical = vertical;
    this.actor.toggle_style_class_name("vertical", vertical);
    this._scrollView.set_policy(
      vertical ? St.PolicyType.NEVER : St.PolicyType.AUTOMATIC,
      vertical ? St.PolicyType.AUTOMATIC : St.PolicyType.NEVER,
    );
    this._applyAppearance();
    ShellCompat.removeChrome(this.actor);
    ShellCompat.addChrome(this.actor, this._chromeOptions());
    this._rebuildBarrier();
    this.refresh();
  }

  refresh() {
    for (const button of this._buttons)
      button.menu?.destroy();
    this._buttons = [];
    const installed = new Set(this._appSystem.get_installed().map(app => app.get_id()));
    const runningApps = this._appSystem.get_running().map(app => ({
      id: app.get_id(),
      windows: app.get_windows(),
    }));
    const model = buildAppModel(this._config.favorites, installed, runningApps);
    const visibleModel = model.filter(item => item.divider || this._appSystem.lookup_app(item.id));
    renderDockContent(
      this._content,
      visibleModel,
      item => this._createAppButton(this._appSystem.lookup_app(item.id), item),
      () => new St.Widget({style_class: "prismatic-divider"}),
    );
    this._layout();
    this._updateVisibility();
  }

  handleDragOver(_source, _actor, _x, _y, _time) {
    const source = _source;
    return ShellCompat.dragMotionResult(Boolean(source?.desktopId || source?.uri));
  }

  acceptDrop(source, _actor, x, y, _time) {
    try {
      const beforeId = this._favoriteBeforeAt(x, y, source.desktopId);
      if (source.desktopId) {
        const mutation = this._config.favorites.includes(source.desktopId)
          ? this._client.move(source.desktopId, beforeId)
          : this._client.pin(source.desktopId, beforeId);
        this._applyMutation(mutation);
        return true;
      }
      if (source.uri && isInstalledDesktopUri(source.uri, applicationRoots())) {
        const desktopId = decodeURIComponent(source.uri).split("/").pop();
        this._applyMutation(this._client.pin(desktopId, beforeId));
        return true;
      }
    } catch (error) {
      console.warn(`Prismatic drop rejected: ${error.message}`);
    }
    return false;
  }

  show(explicit = false) {
    if (ShellCompat.monitorIsFullscreen(this._monitorIndex()) && !explicit)
      return;
    if (explicit)
      this._fullscreenOverride = true;
    if (this._hideTimer) {
      GLib.source_remove(this._hideTimer);
      this._hideTimer = 0;
    }
    if (this._revealTimer) {
      GLib.source_remove(this._revealTimer);
      this._revealTimer = 0;
    }
    this.actor.ease({
      translation_x: 0,
      translation_y: 0,
      duration: ShellCompat.animationDuration(),
      mode: Clutter.AnimationMode.EASE_OUT_QUAD,
    });
  }

  _scheduleHide() {
    if (this._config.behavior.visibility === "fixed")
      return;
    if (this._hasKeyFocus())
      return;
    if (this._hideTimer)
      GLib.source_remove(this._hideTimer);
    this._hideTimer = GLib.timeout_add(
      GLib.PRIORITY_DEFAULT,
      this._config.behavior.hideDelayMs,
      () => {
        this._hideTimer = 0;
        this._hide();
        return GLib.SOURCE_REMOVE;
      },
    );
  }

  _hide() {
    this._fullscreenOverride = false;
    const edge = this._config.geometry.edge;
    const translation = this._config.geometry.iconSize + 16;
    this.actor.ease({
      translation_x: edge === "left" ? -translation : edge === "right" ? translation : 0,
      translation_y: edge === "top" ? -translation : edge === "bottom" ? translation : 0,
      duration: ShellCompat.animationDuration(),
      mode: Clutter.AnimationMode.EASE_OUT_QUAD,
    });
  }

  _scheduleReveal() {
    if (ShellCompat.monitorIsFullscreen(this._monitorIndex()))
      return;
    if (this._revealTimer)
      GLib.source_remove(this._revealTimer);
    this._revealTimer = GLib.timeout_add(
      GLib.PRIORITY_DEFAULT,
      this._config.behavior.revealDelayMs,
      () => {
        this._revealTimer = 0;
        if (!ShellCompat.monitorIsFullscreen(this._monitorIndex()))
          this.show();
        return GLib.SOURCE_REMOVE;
      },
    );
  }

  _updateVisibility() {
    if (ShellCompat.monitorIsFullscreen(this._monitorIndex()) && !this._fullscreenOverride) {
      this._hide();
      return;
    }
    if (this._config.behavior.visibility === "fixed") {
      this.show();
      return;
    }
    if (this._config.behavior.visibility === "autohide") {
      if (!this.actor.hover && !this._hasKeyFocus())
        this._scheduleHide();
      return;
    }
    const workspace = ShellCompat.activeWorkspaceIndex();
    const monitorIndex = this._monitorIndex();
    const dockRect = {
      x: this.actor.x,
      y: this.actor.y,
      width: this.actor.width,
      height: this.actor.height,
    };
    const windows = ShellCompat.dodgeWindowDescriptors();
    if (shouldDodge(windows, dockRect, workspace, monitorIndex) &&
        !this.actor.hover && !this._hasKeyFocus())
      this._scheduleHide();
    else
      this.show();
  }

  _createAppButton(app, item) {
    const box = new St.BoxLayout({vertical: true});
    box.add_child(app.create_icon_texture(this._config.geometry.iconSize));
    if (item.running) {
      const indicator = new St.Widget({
        style_class: "prismatic-running-indicator",
        accessible_name: `${item.windowCount} running window${item.windowCount === 1 ? "" : "s"}`,
      });
      if (this._config.appearance.accent.mode === "custom")
        indicator.set_style(`background-color: ${this._config.appearance.accent.color};`);
      box.add_child(indicator);
    }
    const button = new St.Button({
      style_class: "prismatic-app-button",
      child: box,
      reactive: true,
      can_focus: true,
      accessible_name: app.get_name(),
      button_mask: St.ButtonMask.ONE | St.ButtonMask.TWO | St.ButtonMask.THREE,
    });
    button.desktopId = app.get_id();
    button.pinned = item.pinned;
    button._delegate = button;
    button.connect("button-press-event", (_button, event) => {
      const mouseButton = event.get_button();
      if (mouseButton === 2)
        app.open_new_window(-1);
      else if (mouseButton === 3)
        button.menu.toggle();
      else
        this._activateApp(app);
      return Clutter.EVENT_STOP;
    });
    const draggable = ShellCompat.makeDraggable(button);
    draggable.connect("drag-begin", () => this.actor.add_style_pseudo_class("drop"));
    draggable.connect("drag-end", () => this.actor.remove_style_pseudo_class("drop"));
    button.menu = this._createAppMenu(button, app, item.pinned);
    this._buttons.push(button);
    return button;
  }

  _createAppMenu(button, app, pinned) {
    const menu = ShellCompat.createPopupMenu(button, this._config.geometry.edge);
    ShellCompat.addMenuActor(menu.actor);
    menu.actor.hide();
    for (const window of app.get_windows()) {
      const item = ShellCompat.createPopupMenuItem(window.get_title() || app.get_name());
      item.connect("activate", () => window.activate(ShellCompat.currentTime()));
      menu.addMenuItem(item);
    }
    const newWindow = ShellCompat.createPopupMenuItem("New Window");
    newWindow.connect("activate", () => app.open_new_window(-1));
    menu.addMenuItem(newWindow);
    const pin = ShellCompat.createPopupMenuItem(pinned ? "Unpin from Dock" : "Pin to Dock");
    pin.connect("activate", () => {
      const mutation = pinned ? this._client.unpin(app.get_id()) : this._client.pin(app.get_id());
      this._applyMutation(mutation);
    });
    menu.addMenuItem(pin);
    if (app.get_windows().length > 0) {
      const quit = ShellCompat.createPopupMenuItem("Quit");
      quit.connect("activate", () => app.request_quit());
      menu.addMenuItem(quit);
    }
    const appInfo = app.get_app_info();
    for (const action of appInfo?.list_actions() ?? []) {
      const actionItem = ShellCompat.createPopupMenuItem(appInfo.get_action_name(action));
      actionItem.connect("activate", () => {
        appInfo.launch_action(action, ShellCompat.createAppLaunchContext());
      });
      menu.addMenuItem(actionItem);
    }
    return menu;
  }

  _activateApp(app) {
    const windows = app.get_windows();
    if (windows.length === 0)
      app.activate();
    else {
      const focused = ShellCompat.focusedWindow();
      const focusedIndex = windows.indexOf(focused);
      const nextIndex = focusedIndex >= 0 ? (focusedIndex + 1) % windows.length : 0;
      windows[nextIndex].activate(ShellCompat.currentTime());
    }
  }

  _onKeyPress(event) {
    const symbol = event.get_key_symbol();
    if ([Clutter.KEY_Left, Clutter.KEY_Up].includes(symbol))
      return this._focusRelative(-1);
    if ([Clutter.KEY_Right, Clutter.KEY_Down].includes(symbol))
      return this._focusRelative(1);
    if (symbol === Clutter.KEY_Escape) {
      ShellCompat.clearKeyFocus();
      if (ShellCompat.monitorIsFullscreen(this._monitorIndex()))
        this._hide();
      else
        this._scheduleHide();
      return Clutter.EVENT_STOP;
    }
    return Clutter.EVENT_PROPAGATE;
  }

  _focusRelative(delta) {
    if (this._buttons.length === 0)
      return Clutter.EVENT_STOP;
    const focused = ShellCompat.keyFocus();
    const current = this._buttons.indexOf(focused);
    const next = (current + delta + this._buttons.length) % this._buttons.length;
    this._buttons[next].grab_key_focus();
    return Clutter.EVENT_STOP;
  }

  _layout() {
    const monitorIndex = this._monitorIndex();
    const monitor = ShellCompat.monitorGeometry(monitorIndex);
    const workArea = ShellCompat.monitorWorkArea(monitorIndex);
    const vertical = ["left", "right"].includes(this._config.geometry.edge);
    const layoutArea = vertical
      ? {...monitor, y: workArea.y, height: workArea.height}
      : {...monitor, x: workArea.x, width: workArea.width};
    const geometry = calculateGeometry(layoutArea, this._config.geometry, this._buttons.length);
    this.actor.set_position(geometry.x, geometry.y);
    this.actor.set_size(geometry.width, geometry.height);
  }

  _monitorIndex() {
    if (this._config.display.mode !== "selected")
      return ShellCompat.primaryMonitorIndex();
    const connector = this._config.display.connectorByAdapter.gnome;
    return ShellCompat.monitorIndexForConnector(connector, ShellCompat.primaryMonitorIndex());
  }

  _chromeOptions() {
    return {
      affectsStruts: this._config.behavior.visibility === "fixed",
      trackFullscreen: false,
    };
  }

  _rebuildBarrier() {
    this._barrier?.destroy();
    this._barrier = null;
    if (this._config.behavior.visibility === "fixed")
      return;
    const monitor = ShellCompat.monitorGeometry(this._monitorIndex());
    this._barrier = ShellCompat.createPressureBarrier(
      monitor,
      this._config.geometry.edge,
      () => this._scheduleReveal(),
    );
  }

  _favoriteBeforeAt(x, y, movingId) {
    const vertical = ["left", "right"].includes(this._config.geometry.edge);
    const [dockX, dockY] = this.actor.get_transformed_position();
    const pointer = vertical ? dockY + y : dockX + x;
    for (const button of this._buttons) {
      if (!button.pinned || button.desktopId === movingId)
        continue;
      const [buttonX, buttonY] = button.get_transformed_position();
      const midpoint = vertical
        ? buttonY + button.height / 2
        : buttonX + button.width / 2;
      if (pointer < midpoint)
        return button.desktopId;
    }
    return "";
  }

  _hasKeyFocus() {
    const focused = ShellCompat.keyFocus();
    return focused === this.actor || (focused && this.actor.contains(focused));
  }

  async _applyMutation(mutation) {
    try {
      const config = await mutation;
      if (this._destroyed)
        return;
      this._config = config;
      this.refresh();
    } catch (error) {
      console.warn(`Prismatic configuration update rejected: ${error.message}`);
    }
  }

  _connect(object, signal, callback) {
    this._signals.push([object, object.connect(signal, callback)]);
  }

  _connectShell(source, signal, callback) {
    this._shellSignalCleanups.push(ShellCompat.connectShellSignal(source, signal, callback));
  }

  _applyAppearance() {
    const configured = this._config.appearance.colorScheme;
    const light = configured === "light" ||
      (configured === "system" && ShellCompat.systemPrefersLight());
    this.actor.toggle_style_class_name("light", light);
    this.actor.toggle_style_class_name("dark", !light);
    const surface = light ? "246, 245, 244" : "32, 32, 36";
    this.actor.set_style(
      `border-radius: ${this._config.appearance.cornerRadius}px; ` +
      `background-color: rgba(${surface}, ${this._config.appearance.opacityPercent / 100});`,
    );
  }
}

function applicationRoots() {
  const roots = ["/usr/local/share/applications", "/usr/share/applications"];
  const dataHome = GLib.getenv("XDG_DATA_HOME") || GLib.build_filenamev([GLib.get_home_dir(), ".local", "share"]);
  roots.unshift(GLib.build_filenamev([dataHome, "applications"]));
  return roots;
}
