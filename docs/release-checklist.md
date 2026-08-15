# Fedora 44 release checklist

## Automated gates

- [ ] `just check`, `just test`, and `just test-dbus` pass.
- [ ] GNOME lint and extension packaging pass.
- [ ] QML lint and Plasma package validation pass.
- [ ] `just verify-install` proves staged install/uninstall.
- [ ] SPDX and AppStream checks pass.
- [ ] Fedora 44 x86_64 and aarch64 `mock` builds pass.

## Nested GNOME Wayland

- [ ] 20 enable/disable cycles leave no actors, signals, barriers, or keybindings.
- [ ] App lifecycle, grouping, clicks, context menus, and drag-and-drop pass.
- [ ] Fixed struts, auto-hide, dodge, fullscreen, and shortcut reveal pass.
- [ ] 100%/200% scaling and monitor disconnect/reconnect pass on every edge.

## Plasma

- [ ] Panel creation is idempotent and warns about occupied edges.
- [ ] All length, alignment, edge, screen, visibility, and task mappings pass.
- [ ] Broker restart reconnects without losing task interactions.
- [ ] Removal demonstrably touches only the recorded Prismatic-owned panel.

## Manual release matrix

- [ ] Fedora 44 GNOME Shell 50.x at 100% and 200% scaling.
- [ ] Fedora 44 Plasma 6.6 and 6.7 at 100% and 200% scaling.
- [ ] All four edges, keyboard-only use, reduced motion, and fullscreen.
- [ ] Import/export and service, Shell, and plasmashell restarts.
- [ ] Changelog, version, source tag, checksums, and COPR build agree.
