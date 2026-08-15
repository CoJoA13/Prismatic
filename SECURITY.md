# Security policy

## Supported versions

Until the first stable release, only the current `main` branch receives security
fixes. Prismatic v1 targets Fedora 44 Wayland sessions only.

## Private reporting

Use GitHub's **Security → Report a vulnerability** flow for this repository. Include
the Prismatic commit, Fedora version, desktop/session version, reproduction steps,
and the impact. Please do not include secrets or personally identifying journal data.

Maintainers will acknowledge a complete report within seven days, coordinate a fix
and disclosure window, and credit reporters who want attribution.

## Security boundaries

- The service accepts concrete installed `.desktop` IDs, never arbitrary executables.
- External drops containing files or uninstalled launchers are rejected.
- Imports are schema-validated and revision-checked before atomic replacement.
- No telemetry, launch history, or network service exists.
- The Plasma adapter may manage only its recorded Prismatic-owned panel ID.
