# Prismatic

Prismatic is a native application dock for Fedora 44 Wayland sessions. It uses a
shared Rust configuration broker, a GTK 4/libadwaita settings app, and separate
desktop-native renderers for GNOME Shell 50 and Plasma 6.6 or newer.

> [!IMPORTANT]
> This repository currently contains the v0.1 foundation. It is suitable for
> development and nested-session testing, but has not completed the Fedora 44
> manual release checklist yet.

## Features

- Ordered pinned applications plus grouped running applications from every workspace.
- Top, bottom, left, and right placement on the primary or a selected connector.
- Fit Content, expanded, and 25–100% custom lengths with start/center/end alignment.
- 24–96 px icons, native light/dark appearance, accent, opacity, and corner radius.
- Fixed, auto-hide, and dodge-windows visibility; GNOME has configurable delays,
  while Plasma follows its compositor-native panel timing.
- Secure launcher drag-and-drop, keyboard navigation, and `Super+Alt+D` reveal.
- Revision-checked D-Bus configuration with atomic storage, recovery, and import rollback.
- No telemetry and no application-launch history.

Prismatic does not replace or modify existing docks or panels. GNOME gets an
independent Shell actor. Plasma gets a dedicated panel only after the user presses
the creation action in Settings, and Prismatic records and touches only that panel.

## Supported platform

| Component | v1 contract |
| --- | --- |
| Distribution | Fedora 44 only |
| Session | Wayland only |
| GNOME | Shell 50.x |
| KDE | Plasma 6.6+ |
| GTK/libadwaita | GTK 4.22 / libadwaita 1.9 |

X11, wlroots compositors, badges, icon magnification, arbitrary CSS, workspace
tools, and user scripting are intentionally outside the v1 scope.

## Build from source

Install the Fedora development tools:

```bash
sudo dnf install cargo rust clippy rustfmt meson ninja-build just npm \
  gtk4-devel libadwaita-devel glib2-devel gjs eslint \
  libplasma-devel plasma-workspace-devel appstream desktop-file-utils \
  qt6-qtdeclarative-devel qt6-qttools gettext zip rpm-build mock
```

Then build and stage the install:

```bash
npm ci
just check
just test
just configure
just build
just verify-install
```

Install system-wide only when you are ready to test the native sessions:

```bash
sudo meson install -C build
systemctl --user daemon-reload
```

Open **Prismatic** from the application grid. The settings app detects GNOME or
Plasma and offers an explicit activation action; it never changes existing desktop
UI on first run. See [the user guide](docs/user-guide.md) for session-specific steps.

## Development commands

| Command | Purpose |
| --- | --- |
| `just check` | Rust format/clippy and JavaScript lint |
| `just test` | Rust and shared adapter contract tests |
| `just test-dbus` | Live, isolated session-bus tests |
| `just verify-install` | Staged Meson install and uninstall check |
| `just package-gnome` | Build the Shell extension archive |
| `just package-plasma` | Build the Plasma applet archive |
| `just source-tarball 0.1.0` | Reproducible source archive and checksums |

The configuration API is documented in [docs/configuration.md](docs/configuration.md),
and system boundaries are described in [docs/architecture.md](docs/architecture.md).

## License

Prismatic is licensed under the [GNU General Public License v3.0 or later](LICENSE).
The generated application icon is distributed under the same project license.
