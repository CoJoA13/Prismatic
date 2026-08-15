// SPDX-License-Identifier: GPL-3.0-or-later

const values = (allowed, value) => allowed.includes(value);
const inRange = (minimum, maximum, value) => Number.isInteger(value) && value >= minimum && value <= maximum;
const record = value => value !== null && typeof value === "object" && !Array.isArray(value);
const onlyKeys = (value, allowed) => record(value) &&
  Object.keys(value).every(key => allowed.includes(key));

function validAccent(accent) {
  if (!onlyKeys(accent, ["mode", "color"]))
    return false;
  if (accent.mode === "system")
    return accent.color === undefined;
  return accent.mode === "custom" && /^#[0-9a-fA-F]{6}$/.test(accent.color ?? "");
}

export function acceptsSchemaV1(config) {
  if (!onlyKeys(config, [
    "schemaVersion", "seeded", "favorites", "display", "geometry", "behavior", "appearance",
  ]) || config.schemaVersion !== 1 || typeof config.seeded !== "boolean" ||
      !Array.isArray(config.favorites))
    return false;
  if (!config.favorites.every((id, index) =>
    typeof id === "string" && id.endsWith(".desktop") && !id.includes("/") &&
      !id.includes("..") && !/[\u0000-\u001f\u007f]/.test(id) &&
      config.favorites.indexOf(id) === index))
    return false;
  if (!onlyKeys(config.display, ["mode", "connectorByAdapter"]) ||
      !values(["primary", "selected"], config.display.mode) ||
      !record(config.display.connectorByAdapter) ||
      Object.entries(config.display.connectorByAdapter).some(([adapter, connector]) =>
        adapter.trim().length === 0 || typeof connector !== "string" || connector.trim().length === 0))
    return false;
  if (!onlyKeys(config.geometry, [
    "edge", "lengthMode", "customLengthPercent", "alignment", "iconSize",
  ]) || !values(["top", "bottom", "left", "right"], config.geometry.edge))
    return false;
  if (!values(["fit", "expand", "custom"], config.geometry.lengthMode))
    return false;
  if (!values(["start", "center", "end"], config.geometry.alignment))
    return false;
  if (!inRange(25, 100, config.geometry.customLengthPercent) ||
      !inRange(24, 96, config.geometry.iconSize))
    return false;
  if (!onlyKeys(config.behavior, [
    "visibility", "revealDelayMs", "hideDelayMs", "shortcut",
  ]) || !values(["fixed", "autohide", "dodge"], config.behavior.visibility))
    return false;
  if (!inRange(0, 1000, config.behavior.revealDelayMs) ||
      !inRange(0, 1000, config.behavior.hideDelayMs) ||
      typeof config.behavior.shortcut !== "string" || config.behavior.shortcut.trim().length === 0)
    return false;
  if (!onlyKeys(config.appearance, [
    "colorScheme", "accent", "opacityPercent", "cornerRadius",
  ]) || !values(["system", "light", "dark"], config.appearance.colorScheme) ||
      !validAccent(config.appearance.accent))
    return false;
  return inRange(60, 100, config.appearance.opacityPercent) &&
    inRange(0, 24, config.appearance.cornerRadius);
}
