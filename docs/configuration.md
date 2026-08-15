# Configuration contract

The only durable document is `$XDG_CONFIG_HOME/prismatic/config.json`. It wraps the
schema-v1 config with a monotonic revision:

```json
{
  "revision": 7,
  "config": {
    "schemaVersion": 1,
    "seeded": true,
    "favorites": ["firefox.desktop"],
    "display": {"mode": "primary", "connectorByAdapter": {}},
    "geometry": {
      "edge": "bottom",
      "lengthMode": "fit",
      "customLengthPercent": 60,
      "alignment": "center",
      "iconSize": 48
    },
    "behavior": {
      "visibility": "dodge",
      "revealDelayMs": 160,
      "hideDelayMs": 300,
      "shortcut": "<Super><Alt>d"
    },
    "appearance": {
      "colorScheme": "system",
      "accent": {"mode": "system"},
      "opacityPercent": 88,
      "cornerRadius": 16
    }
  }
}
```

## Ranges and enums

- `edge`: `top`, `bottom`, `left`, `right`
- `lengthMode`: `fit`, `expand`, `custom`; custom length is 25–100
- `alignment`: `start`, `center`, `end`; icon size is 24–96 logical px
- `visibility`: `fixed`, `autohide`, `dodge`; each delay is 0–1000 ms. GNOME
  applies both delays. Plasma 6.6/6.7 uses native panel timing because its public
  panel API does not expose per-panel reveal or hide delays.
- `colorScheme`: `system`, `light`, `dark`; opacity 60–100; radius 0–24 px
- `accent`: `{"mode":"system"}` or `{"mode":"custom","color":"#RRGGBB"}`
- `favorites`: unique, concrete, installed `.desktop` identifiers

## D-Bus methods

`GetSnapshot`, `ReplaceSnapshot`, `Pin`, `Unpin`, `Move`, `Import`, `Export`,
`GetDesktopActions`, `LaunchDesktopAction`, `RegisterAdapter`, and
`UpdateAdapterStatus` form the supported interface. The service
emits `SnapshotChanged` and `AdapterStatusChanged`. Named error domains distinguish
invalid configuration, revision conflicts, unknown desktop entries, unsupported schema
versions, unsupported adapters, unknown desktop actions, and invalid requests.

Client code must never edit the JSON document directly.
