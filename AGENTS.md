# Repository guidance

- Prismatic v1 supports Fedora 44 Wayland only: GNOME Shell 50.x and Plasma 6.6+.
- The Rust D-Bus service is the only persistent configuration writer.
- Keep GNOME private Shell API calls isolated in `adapters/gnome/shellCompat.js`.
- Never alter a Plasma panel not proven to be the recorded Prismatic-owned panel.
- Use `just check`, `just test`, `just test-dbus`, and `just verify-install` before release.
- Add SPDX identifiers and tests with behavior changes; do not add telemetry or launch history.
