// SPDX-License-Identifier: GPL-3.0-or-later

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {fileURLToPath} from "node:url";

import {acceptsSchemaV1} from "../../shared/configContract.mjs";
import {
  desktopActions,
  launcherUrls,
  panelProperties,
  plasmaShortcut,
  selectedConnectorMissing,
} from "../contents/code/panelLogic.mjs";

test("panel properties map the shared contract to native Plasma values", () => {
  assert.deepEqual(
    panelProperties({
      geometry: {
        edge: "left",
        lengthMode: "custom",
        customLengthPercent: 55,
        alignment: "start",
        iconSize: 48,
      },
      behavior: {visibility: "dodge"},
    }),
    {
      location: "left",
      lengthMode: "custom",
      lengthPercent: 55,
      alignment: "right",
      height: 64,
      hiding: "dodgewindows",
    },
  );

  assert.equal(
    panelProperties({
      geometry: {edge: "bottom", lengthMode: "expand", alignment: "end", iconSize: 40},
      behavior: {visibility: "autohide"},
    }).lengthMode,
    "fill",
  );
});

test("favorites become canonical application launcher URLs without accepting paths", () => {
  assert.deepEqual(
    launcherUrls(["firefox.desktop", "org.kde.dolphin.desktop", "../evil.desktop"]),
    ["applications:firefox.desktop", "applications:org.kde.dolphin.desktop"],
  );
});

test("shared shortcut syntax becomes a native Plasma key sequence", () => {
  assert.equal(plasmaShortcut("<Super><Alt>d"), "Meta+Alt+D");
  assert.equal(plasmaShortcut("<Ctrl><Shift>space"), "Ctrl+Shift+Space");
});

test("desktop action payloads are closed to safe identifiers and visible labels", () => {
  assert.deepEqual(
    desktopActions(JSON.stringify([
      {id: "Private", name: "New Private Window"},
      {id: "bad/action", name: "Unsafe"},
      {id: "Empty", name: ""},
    ])),
    [{id: "Private", name: "New Private Window"}],
  );
  assert.deepEqual(desktopActions("not json"), []);
});

test("selected connector status follows the current output topology", () => {
  const config = {display: {mode: "selected", connectorByAdapter: {plasma: "DP-2"}}};
  assert.equal(selectedConnectorMissing(config, ["eDP-1", "DP-2"]), false);
  assert.equal(selectedConnectorMissing(config, ["eDP-1"]), true);
  assert.equal(selectedConnectorMissing({display: {mode: "primary"}}, ["eDP-1"]), false);
});

test("runtime events keep Plasma topology, diagnostics, and keyboard menus current", () => {
  const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  const mainSource = fs.readFileSync(
    path.join(project, "adapters/plasma/contents/ui/main.qml"),
    "utf8",
  );
  const buttonSource = fs.readFileSync(
    path.join(project, "adapters/plasma/contents/ui/TaskButton.qml"),
    "utf8",
  );

  assert.match(mainSource, /function onScreensChanged\(\)/);
  assert.doesNotMatch(mainSource, /onScreen(?:Added|Removed)/);

  const applySnapshot = mainSource.slice(
    mainSource.indexOf("function applySnapshot"),
    mainSource.indexOf("function registerAdapter"),
  );
  assert.match(applySnapshot, /config = parsed;[\s\S]*registerAdapter\(true,/);
  assert.match(
    buttonSource,
    /Keys\.onMenuPressed:\s*button\.menuRequested\(button\.desktopId\)/,
  );
});

test("shared schema fixtures agree with the Plasma contract", () => {
  const project = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  const fixtureRoot = path.join(project, "tests/fixtures");
  const manifest = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "manifest.json")));
  for (const fixture of manifest.valid)
    assert.equal(acceptsSchemaV1(JSON.parse(fs.readFileSync(path.join(fixtureRoot, fixture)))), true);
  for (const fixture of manifest.invalid)
    assert.equal(acceptsSchemaV1(JSON.parse(fs.readFileSync(path.join(fixtureRoot, fixture)))), false);
});
