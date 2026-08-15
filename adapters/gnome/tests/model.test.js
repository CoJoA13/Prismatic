// SPDX-License-Identifier: GPL-3.0-or-later

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {fileURLToPath} from "node:url";

import {acceptsSchemaV1} from "../../shared/configContract.mjs";
import {renderDockContent} from "../dockContent.js";

import {
  buildAppModel,
  calculateGeometry,
  chooseMonitorForConnector,
  isInstalledDesktopUri,
  shouldDodge,
} from "../model.js";

test("app model hides unavailable favorites and appends grouped running applications", () => {
  const model = buildAppModel(
    ["firefox.desktop", "missing.desktop"],
    new Set(["firefox.desktop", "org.gnome.Ptyxis.desktop"]),
    [
      {id: "firefox.desktop", windows: ["one", "two"]},
      {id: "org.gnome.Ptyxis.desktop", windows: ["terminal"]},
    ],
  );

  assert.deepEqual(model, [
    {id: "firefox.desktop", pinned: true, running: true, windowCount: 2},
    {divider: true},
    {
      id: "org.gnome.Ptyxis.desktop",
      pinned: false,
      running: true,
      windowCount: 1,
    },
  ]);
});

test("fit geometry caps at ninety percent and preserves edge alignment", () => {
  const centered = calculateGeometry(
    {x: 0, y: 0, width: 1920, height: 1080},
    {edge: "bottom", lengthMode: "fit", alignment: "center", iconSize: 48},
    100,
  );
  assert.deepEqual(centered, {x: 96, y: 1016, width: 1728, height: 64, overflow: true});

  const vertical = calculateGeometry(
    {x: 1920, y: 0, width: 1440, height: 2560},
    {
      edge: "left",
      lengthMode: "custom",
      customLengthPercent: 50,
      alignment: "end",
      iconSize: 48,
    },
    4,
  );
  assert.deepEqual(vertical, {
    x: 1920,
    y: 1280,
    width: 64,
    height: 1280,
    overflow: false,
  });
});

test("dodge considers only intersecting normal windows on the active workspace", () => {
  const dock = {x: 600, y: 1016, width: 720, height: 64};
  const windows = [
    {rect: {x: 0, y: 0, width: 1920, height: 1080}, workspace: 1, monitor: 0, normal: true},
    {rect: {x: 600, y: 1000, width: 400, height: 80}, workspace: 0, monitor: 0, normal: false},
    {rect: {x: 600, y: 1000, width: 400, height: 80}, workspace: 0, monitor: 1, normal: true},
  ];
  assert.equal(shouldDodge(windows, dock, 0, 0), false);
  windows.push({
    rect: {x: 600, y: 1000, width: 400, height: 80},
    workspace: 0,
    monitor: 0,
    normal: true,
  });
  assert.equal(shouldDodge(windows, dock, 0, 0), true);
});

test("desktop drops are restricted to installed XDG launcher roots", () => {
  const roots = ["/usr/share/applications", "/home/test/.local/share/applications"];
  assert.equal(
    isInstalledDesktopUri("file:///usr/share/applications/firefox.desktop", roots),
    true,
  );
  assert.equal(isInstalledDesktopUri("file:///tmp/untrusted.desktop", roots), false);
  assert.equal(
    isInstalledDesktopUri("file:///usr/share/applications/../evil.desktop", roots),
    false,
  );
  assert.equal(isInstalledDesktopUri("file:///usr/share/applications/readme.txt", roots), false);
});

test("saved connectors select Mutter logical monitor numbers and fall back safely", () => {
  const logicalMonitors = [
    {number: 2, connectors: ["DP-2"]},
    {number: 0, connectors: ["eDP-1"]},
    {number: 1, connectors: ["DP-1", "HDMI-A-1"]},
  ];
  assert.equal(chooseMonitorForConnector(logicalMonitors, "HDMI-A-1", 0), 1);
  assert.equal(chooseMonitorForConnector(logicalMonitors, "disconnected", 0), 0);
  assert.equal(chooseMonitorForConnector(logicalMonitors, "", 2), 2);
});

test("dock refreshes replace the scroll content tree without accumulating actors", () => {
  const content = {
    children: [],
    destroy_all_children() {
      this.children = [];
    },
    add_child(child) {
      this.children.push(child);
    },
  };
  const model = [{id: "one.desktop"}, {divider: true}, {id: "two.desktop"}];
  const createButton = item => ({button: item.id});
  const createDivider = () => ({divider: true});

  renderDockContent(content, model, createButton, createDivider);
  renderDockContent(content, model, createButton, createDivider);

  assert.deepEqual(content.children, [
    {button: "one.desktop"},
    {divider: true},
    {button: "two.desktop"},
  ]);
});

test("service snapshots and offline caches use the complete shared schema validator", () => {
  const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  const clientSource = fs.readFileSync(path.join(project, "adapters/gnome/serviceClient.js"), "utf8");
  assert.match(clientSource, /import \{acceptsSchemaV1\} from "\.\/configContract\.mjs"/);
  assert.match(clientSource, /if \(!acceptsSchemaV1\(config\)\)/);
  assert.match(clientSource, /if \(acceptsSchemaV1\(config\)\)/);
});

test("shared schema fixtures agree with the GNOME contract", () => {
  const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  const fixtureRoot = path.join(project, "tests/fixtures");
  const manifest = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "manifest.json")));
  for (const fixture of manifest.valid)
    assert.equal(acceptsSchemaV1(JSON.parse(fs.readFileSync(path.join(fixtureRoot, fixture)))), true);
  for (const fixture of manifest.invalid)
    assert.equal(acceptsSchemaV1(JSON.parse(fs.readFileSync(path.join(fixtureRoot, fixture)))), false);
});
