# User guide

## First run

Launch **Prismatic** after installation. The app detects a supported Wayland desktop
and shows an explicit activation button. It does not disable, replace, or reconfigure
an existing dock or panel.

### GNOME Shell 50

Choose **Enable GNOME Dock**. The extension adds its own dock actor and can be disabled
with `gnome-extensions disable prismatic@cojoa13.github.io`. Log out and back in after
a system-wide extension update if Shell has not reloaded it.

### Plasma 6.6+

Choose **Create Plasma Dock**. Prismatic checks for an existing Prismatic applet before
creating a dedicated panel. If the selected screen edge is occupied, inspect the edge
before confirming. Existing panels are not modified.

## Applications and drag-and-drop

Favorites appear first; grouped running applications from all workspaces follow.
Drag pinned icons to reorder them or pin a running application. Drops are accepted only
for installed XDG `.desktop` launchers. Arbitrary files and external executables are
rejected.

Left click launches or focuses; repeat to cycle windows. Middle click requests a new
window. Right click shows windows, supported desktop actions, pinning, and quit actions.

## Visibility

Fixed mode reserves work area. Auto-hide and dodge modes overlay windows. Pointer reveal
is suppressed over fullscreen applications; `Super+Alt+D` remains available. Fit Content
grows to 90% of the work area and then scrolls.

Reveal and hide delay controls apply to GNOME. Plasma uses its native panel timing because
Plasma 6.6/6.7 does not expose per-panel timing through the public panel API.

## Recovery and diagnostics

Use the Diagnostics page and `journalctl --user -u prismatic-service` when reporting a
problem. A corrupt config is quarantined beside the replacement. Import keeps a rollback
copy at `config.json.bak`. Diagnostic logs contain no telemetry or launch history.
