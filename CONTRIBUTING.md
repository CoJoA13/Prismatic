# Contributing to Prismatic

Thank you for helping build a reliable Fedora dock. By participating, you agree to
follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before opening a change

1. Search existing issues and keep each change focused on one problem.
2. Preserve the v1 support boundary: Fedora 44, Wayland, GNOME 50, and Plasma 6.6+.
3. Keep persistent writes in `prismatic-service`; adapters are configuration clients.
4. Keep GNOME private APIs inside `adapters/gnome/shellCompat.js`.
5. Never modify a Plasma panel unless its ID is recorded as Prismatic-owned.
6. Add an SPDX license identifier to source and configuration files.

## Tests and commits

Write a failing contract test before behavior changes, then run:

```bash
just check
just test
just test-dbus
just verify-install
```

Native behavior changes also need the relevant nested GNOME or Plasma test from
[the release checklist](docs/release-checklist.md). Use imperative commit subjects,
explain user-visible behavior in the body, and update `CHANGELOG.md`.

## Translations

User-visible strings belong in gettext-compatible `i18n()` calls or Rust source
listed in `po/POTFILES.in`. Do not concatenate translated sentence fragments.

## Reporting security issues

Do not open a public issue for a vulnerability. Follow [SECURITY.md](SECURITY.md).
