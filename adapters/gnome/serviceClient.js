// SPDX-License-Identifier: GPL-3.0-or-later

import Gio from "gi://Gio";
import GLib from "gi://GLib";

import {acceptsSchemaV1} from "./configContract.mjs";

const NAME = "io.github.CoJoA13.Prismatic.Service";
const PATH = "/io/github/CoJoA13/Prismatic";
const INTERFACE = "io.github.CoJoA13.Prismatic.Service1";
const CALL_TIMEOUT_MS = 5000;

Gio._promisify(Gio.DBusProxy, "new_for_bus", "new_for_bus_finish");
Gio._promisify(Gio.DBusProxy.prototype, "call", "call_finish");

const DEFAULT_CONFIG = {
  schemaVersion: 1,
  seeded: false,
  favorites: [],
  display: {mode: "primary", connectorByAdapter: {}},
  geometry: {
    edge: "bottom",
    lengthMode: "fit",
    customLengthPercent: 60,
    alignment: "center",
    iconSize: 48,
  },
  behavior: {
    visibility: "dodge",
    revealDelayMs: 160,
    hideDelayMs: 300,
    shortcut: "<Super><Alt>d",
  },
  appearance: {
    colorScheme: "system",
    accent: {mode: "system"},
    opacityPercent: 88,
    cornerRadius: 16,
  },
};

export class ServiceClient {
  constructor() {
    this._proxy = null;
    this._cancellable = null;
    this._signalId = 0;
    this._ownerSignalId = 0;
    this._revision = 0;
    this._config = this._readCache();
  }

  async connect(onSnapshot, onDisconnect = () => {}) {
    this.disconnect();
    this._cancellable = new Gio.Cancellable();
    this._proxy = await Gio.DBusProxy.new_for_bus(
      Gio.BusType.SESSION,
      Gio.DBusProxyFlags.NONE,
      null,
      NAME,
      PATH,
      INTERFACE,
      this._cancellable,
    );
    const [revision, json] = await this._call("GetSnapshot", null);
    this._applySnapshot(revision, json);
    this._signalId = this._proxy.connect("g-signal", (_proxy, _sender, signal, parameters) => {
      if (signal !== "SnapshotChanged")
        return;
      const [nextRevision, nextJson] = parameters.deepUnpack();
      this._applySnapshot(nextRevision, nextJson);
      onSnapshot(this._revision, this._config);
    });
    this._ownerSignalId = this._proxy.connect("notify::g-name-owner", proxy => {
      if (!proxy.g_name_owner)
        onDisconnect();
    });
    return [this._revision, this._config];
  }

  disconnect() {
    if (this._proxy && this._signalId)
      this._proxy.disconnect(this._signalId);
    if (this._proxy && this._ownerSignalId)
      this._proxy.disconnect(this._ownerSignalId);
    this._cancellable?.cancel();
    this._signalId = 0;
    this._ownerSignalId = 0;
    this._proxy = null;
    this._cancellable = null;
  }

  async register(status) {
    const [revision, json] = await this._call(
      "RegisterAdapter",
      new GLib.Variant("(ss)", ["gnome", JSON.stringify(status)]),
    );
    this._applySnapshot(revision, json);
  }

  async updateStatus(status) {
    await this._call(
      "UpdateAdapterStatus",
      new GLib.Variant("(ss)", ["gnome", JSON.stringify(status)]),
    );
  }

  async pin(desktopId, beforeId = "") {
    const [revision, json] = await this._call(
      "Pin",
      new GLib.Variant("(tss)", [this._revision, desktopId, beforeId]),
    );
    this._applySnapshot(revision, json);
    return this._config;
  }

  async unpin(desktopId) {
    const [revision, json] = await this._call(
      "Unpin",
      new GLib.Variant("(ts)", [this._revision, desktopId]),
    );
    this._applySnapshot(revision, json);
    return this._config;
  }

  async move(desktopId, beforeId = "") {
    const [revision, json] = await this._call(
      "Move",
      new GLib.Variant("(tss)", [this._revision, desktopId, beforeId]),
    );
    this._applySnapshot(revision, json);
    return this._config;
  }

  cachedConfig() {
    return this._config;
  }

  async _call(method, parameters) {
    const proxy = this._proxy;
    const cancellable = this._cancellable;
    if (!proxy)
      throw new Error("Prismatic service is not connected");
    const reply = await proxy.call(
      method,
      parameters,
      Gio.DBusCallFlags.NONE,
      CALL_TIMEOUT_MS,
      cancellable,
    );
    return reply ? reply.deepUnpack() : [];
  }

  _applySnapshot(revision, json) {
    const config = JSON.parse(json);
    if (!acceptsSchemaV1(config))
      throw new Error("Invalid Prismatic schema-v1 snapshot");
    this._revision = revision;
    this._config = config;
    this._writeCache(config);
  }

  _cacheFile() {
    return Gio.File.new_for_path(
      GLib.build_filenamev([GLib.get_user_cache_dir(), "prismatic", "gnome.json"]),
    );
  }

  _readCache() {
    try {
      const [ok, contents] = this._cacheFile().load_contents(null);
      if (ok) {
        const config = JSON.parse(new TextDecoder().decode(contents));
        if (acceptsSchemaV1(config))
          return config;
      }
    } catch (_error) {
      // Cache recovery falls through to the public defaults.
    }
    return structuredClone(DEFAULT_CONFIG);
  }

  _writeCache(config) {
    try {
      const directory = this._cacheFile().get_parent();
      directory.make_directory_with_parents(null);
    } catch (error) {
      if (!error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.EXISTS))
        console.warn(`Prismatic cache directory: ${error.message}`);
    }
    try {
      this._cacheFile().replace_contents(
        JSON.stringify(config),
        null,
        false,
        Gio.FileCreateFlags.REPLACE_DESTINATION,
        null,
      );
    } catch (error) {
      console.warn(`Prismatic cache write: ${error.message}`);
    }
  }
}
