// SPDX-License-Identifier: GPL-3.0-or-later

export function panelProperties(config) {
  const vertical = config.geometry.edge === "left" || config.geometry.edge === "right";
  return {
    location: config.geometry.edge,
    lengthMode: config.geometry.lengthMode === "expand" ? "fill" : config.geometry.lengthMode,
    lengthPercent: config.geometry.customLengthPercent ?? 60,
    alignment: plasmaAlignment(config.geometry.alignment, vertical),
    height: config.geometry.iconSize + 16,
    hiding: plasmaHiding(config.behavior.visibility),
  };
}

export function launcherUrls(favorites) {
  return favorites
    .filter(id => id.endsWith(".desktop") && !id.includes("/") && !id.includes(".."))
    .map(id => `applications:${id}`);
}

export function desktopActions(json) {
  try {
    const actions = JSON.parse(json);
    if (!Array.isArray(actions))
      return [];
    return actions.filter(action => action !== null && typeof action === "object" &&
      /^[A-Za-z0-9._-]{1,128}$/.test(action.id ?? "") &&
      typeof action.name === "string" && action.name.trim().length > 0)
      .map(action => ({id: action.id, name: action.name.trim()}));
  } catch (_error) {
    return [];
  }
}

export function selectedConnectorMissing(config, connectors) {
  if (config.display?.mode !== "selected")
    return false;
  const selected = String(config.display.connectorByAdapter?.plasma || "");
  return selected.length > 0 && !connectors.includes(selected);
}

export function plasmaShortcut(shortcut) {
  const modifiers = [];
  const key = shortcut.replace(/<([^>]+)>/g, (_match, modifier) => {
    const normalized = {
      Super: "Meta",
      Primary: "Ctrl",
      Control: "Ctrl",
      Ctrl: "Ctrl",
      Alt: "Alt",
      Shift: "Shift",
    }[modifier];
    if (normalized)
      modifiers.push(normalized);
    return "";
  }).trim();
  const normalizedKey = key.length === 1
    ? key.toUpperCase()
    : key.charAt(0).toUpperCase() + key.slice(1).toLowerCase();
  return [...modifiers, normalizedKey].filter(Boolean).join("+");
}

function plasmaAlignment(alignment, vertical) {
  if (alignment === "center")
    return "center";
  if (!vertical)
    return alignment === "start" ? "left" : "right";
  return alignment === "start" ? "right" : "left";
}

function plasmaHiding(visibility) {
  if (visibility === "autohide")
    return "autohide";
  if (visibility === "dodge")
    return "dodgewindows";
  return "none";
}
