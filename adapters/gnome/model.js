// SPDX-License-Identifier: GPL-3.0-or-later

export function buildAppModel(favorites, installed, runningApps) {
  const running = new Map(runningApps.map(app => [app.id, app.windows.length]));
  const pinned = favorites
    .filter(id => installed.has(id))
    .map(id => ({
      id,
      pinned: true,
      running: running.has(id),
      windowCount: running.get(id) ?? 0,
    }));
  const unpinned = runningApps
    .filter(app => !favorites.includes(app.id))
    .map(app => ({
      id: app.id,
      pinned: false,
      running: true,
      windowCount: app.windows.length,
    }));

  if (pinned.length > 0 && unpinned.length > 0)
    return [...pinned, {divider: true}, ...unpinned];
  return [...pinned, ...unpinned];
}

export function calculateGeometry(monitor, geometry, iconCount) {
  const vertical = geometry.edge === "left" || geometry.edge === "right";
  const available = vertical ? monitor.height : monitor.width;
  const thickness = geometry.iconSize + 16;
  const contentLength = iconCount === 0
    ? thickness
    : iconCount * geometry.iconSize + Math.max(0, iconCount - 1) * 8 + 32;
  let length;
  switch (geometry.lengthMode) {
  case "expand":
    length = available;
    break;
  case "custom":
    length = Math.round(available * (geometry.customLengthPercent ?? 60) / 100);
    break;
  default:
    length = Math.min(contentLength, Math.round(available * 0.9));
    break;
  }
  const offset = alignmentOffset(available, length, geometry.alignment);

  if (vertical) {
    return {
      x: geometry.edge === "left" ? monitor.x : monitor.x + monitor.width - thickness,
      y: monitor.y + offset,
      width: thickness,
      height: length,
      overflow: contentLength > length,
    };
  }
  return {
    x: monitor.x + offset,
    y: geometry.edge === "top" ? monitor.y : monitor.y + monitor.height - thickness,
    width: length,
    height: thickness,
    overflow: contentLength > length,
  };
}

function alignmentOffset(available, length, alignment) {
  if (alignment === "start")
    return 0;
  if (alignment === "end")
    return available - length;
  return Math.round((available - length) / 2);
}

export function shouldDodge(windows, dockRect, workspace, monitor) {
  return windows.some(window =>
    window.normal &&
    window.workspace === workspace &&
    window.monitor === monitor &&
    intersects(window.rect, dockRect),
  );
}

export function chooseMonitorForConnector(logicalMonitors, connector, fallback) {
  if (typeof connector !== "string" || connector.length === 0)
    return fallback;
  const match = logicalMonitors.find(logical =>
    Array.isArray(logical.connectors) && logical.connectors.includes(connector),
  );
  return Number.isInteger(match?.number) ? match.number : fallback;
}

function intersects(a, b) {
  return a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y;
}

export function isInstalledDesktopUri(uri, applicationRoots) {
  if (!uri.startsWith("file://"))
    return false;
  let path;
  try {
    path = decodeURIComponent(uri.slice("file://".length));
  } catch (_error) {
    return false;
  }
  if (!path.endsWith(".desktop") || path.includes("\0"))
    return false;
  const segments = path.split("/");
  if (segments.some(segment => segment === "." || segment === ".."))
    return false;

  return applicationRoots.some(root => {
    const normalized = root.endsWith("/") ? root.slice(0, -1) : root;
    return path.startsWith(`${normalized}/`) && path.length > normalized.length + 1;
  });
}
