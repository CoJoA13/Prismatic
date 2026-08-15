// SPDX-License-Identifier: GPL-3.0-or-later

import GLib from "gi://GLib";

import {Extension} from "resource:///org/gnome/shell/extensions/extension.js";

import {DockController} from "./dock.js";
import {ServiceClient} from "./serviceClient.js";
import * as ShellCompat from "./shellCompat.js";

export default class PrismaticExtension extends Extension {
  enable() {
    this._enabled = true;
    this._client = new ServiceClient();
    this._reconnectTimer = 0;
    const config = this._client.cachedConfig();
    this._dock = new DockController(this._client, config);
    this._syncShortcut(config.behavior.shortcut);
    this._connectService().catch(error => {
      console.warn(`Prismatic service unavailable; using cached settings: ${error.message}`);
      if (this._enabled)
        this._scheduleReconnect();
    });
  }

  disable() {
    this._enabled = false;
    if (this._reconnectTimer)
      GLib.source_remove(this._reconnectTimer);
    this._reconnectTimer = 0;
    const client = this._client;
    this._client = null;
    if (client) {
      client.updateStatus({
        active: false,
        version: "50",
        capabilities: [],
        outputs: [],
        message: "extension disabled",
      }).catch(() => {}).finally(() => client.disconnect());
    }
    ShellCompat.removeKeybinding("reveal-shortcut");
    this._dock?.destroy();
    this._dock = null;
  }

  _syncShortcut(shortcut) {
    const settings = this.getSettings();
    settings.set_strv("reveal-shortcut", [shortcut]);
    ShellCompat.removeKeybinding("reveal-shortcut");
    ShellCompat.addKeybinding(
      "reveal-shortcut",
      settings,
      () => {
        this._dock?.show(true);
        this._dock?.actor.grab_key_focus();
      },
    );
  }

  async _connectService() {
    const client = this._client;
    if (!client || !this._enabled)
      throw new Error("Prismatic extension is disabled");
    const snapshot = await client.connect(
      (_nextRevision, nextConfig) => {
        if (!this._enabled)
          return;
        this._syncShortcut(nextConfig.behavior.shortcut);
        this._dock?.applyConfig(nextConfig);
      },
      () => {
        if (this._enabled)
          this._scheduleReconnect();
      },
    );
    if (!this._enabled)
      throw new Error("Prismatic extension was disabled during service connection");
    await client.register({
      active: true,
      version: "50",
      capabilities: [
        "fit",
        "expand",
        "custom-length",
        "autohide",
        "dodge",
        "pressure-barrier",
        "drag-drop",
      ],
      outputs: ShellCompat.monitorConnectorIds(),
      message: null,
    });
    return snapshot;
  }

  _scheduleReconnect() {
    if (!this._enabled || this._reconnectTimer)
      return;
    this._reconnectTimer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
      this._reconnectTimer = 0;
      this._connectService().then(([_revision, config]) => {
        if (!this._enabled)
          return;
        this._dock?.applyConfig(config);
        this._syncShortcut(config.behavior.shortcut);
      }).catch(() => {
        if (this._enabled)
          this._scheduleReconnect();
      });
      return GLib.SOURCE_REMOVE;
    });
  }
}
