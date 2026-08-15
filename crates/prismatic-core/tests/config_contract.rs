// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::PathBuf;

use prismatic_core::{
    Alignment, Appearance, ColorScheme, Config, ConfigStore, DisplayMode, Edge, Geometry,
    LengthMode, Snapshot, StoreError, Visibility,
};

#[test]
fn defaults_match_the_public_dock_contract() {
    let config = Config::default();

    assert_eq!(config.schema_version, 1);
    assert!(!config.seeded);
    assert!(config.favorites.is_empty());
    assert_eq!(config.display.mode, DisplayMode::Primary);
    assert_eq!(config.geometry.edge, Edge::Bottom);
    assert_eq!(config.geometry.length_mode, LengthMode::Fit);
    assert_eq!(config.geometry.custom_length_percent, 60);
    assert_eq!(config.geometry.alignment, Alignment::Center);
    assert_eq!(config.geometry.icon_size, 48);
    assert_eq!(config.behavior.visibility, Visibility::Dodge);
    assert_eq!(config.behavior.reveal_delay_ms, 160);
    assert_eq!(config.behavior.hide_delay_ms, 300);
    assert_eq!(config.behavior.shortcut, "<Super><Alt>d");
    assert_eq!(config.appearance.color_scheme, ColorScheme::System);
    assert_eq!(config.appearance, Appearance::default());
    assert!(config.validate().is_ok());
}

#[test]
fn validation_rejects_every_out_of_contract_range() {
    let mut icon = Config::default();
    icon.geometry.icon_size = 23;
    assert_eq!(icon.validate().unwrap_err().field(), "geometry.iconSize");

    let mut length = Config::default();
    length.geometry.custom_length_percent = 101;
    assert_eq!(
        length.validate().unwrap_err().field(),
        "geometry.customLengthPercent"
    );

    let mut reveal = Config::default();
    reveal.behavior.reveal_delay_ms = 1001;
    assert_eq!(
        reveal.validate().unwrap_err().field(),
        "behavior.revealDelayMs"
    );

    let mut opacity = Config::default();
    opacity.appearance.opacity_percent = 59;
    assert_eq!(
        opacity.validate().unwrap_err().field(),
        "appearance.opacityPercent"
    );

    let mut radius = Config::default();
    radius.appearance.corner_radius = 25;
    assert_eq!(
        radius.validate().unwrap_err().field(),
        "appearance.cornerRadius"
    );
}

#[test]
fn validation_rejects_unsafe_or_duplicate_launcher_ids() {
    for invalid in ["firefox", "../evil.desktop", "/tmp/evil.desktop", ""] {
        let mut config = Config::default();
        config.favorites.push(invalid.to_string());
        assert_eq!(config.validate().unwrap_err().field(), "favorites");
    }

    let duplicate = Config {
        favorites: vec!["firefox.desktop".into(), "firefox.desktop".into()],
        ..Config::default()
    };
    assert_eq!(duplicate.validate().unwrap_err().field(), "favorites");
}

#[test]
fn store_persists_snapshots_and_rejects_stale_revisions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let store = ConfigStore::open(&path).unwrap();
    let mut next = store.snapshot().config;
    next.geometry.icon_size = 64;

    let saved = store.replace(0, next.clone()).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.config, next);

    let stale = store.replace(0, Config::default()).unwrap_err();
    assert!(matches!(
        stale,
        StoreError::RevisionConflict {
            expected: 0,
            actual: 1
        }
    ));

    let reopened = ConfigStore::open(&path).unwrap();
    assert_eq!(reopened.snapshot(), saved);
}

#[test]
fn ordered_favorite_operations_are_atomic_and_revision_checked() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();

    let first = store.pin(0, "firefox.desktop", None).unwrap();
    let second = store
        .pin(
            first.revision,
            "org.gnome.Nautilus.desktop",
            Some("firefox.desktop"),
        )
        .unwrap();
    assert_eq!(
        second.config.favorites,
        ["org.gnome.Nautilus.desktop", "firefox.desktop"]
    );

    let moved = store
        .move_favorite(
            second.revision,
            "firefox.desktop",
            Some("org.gnome.Nautilus.desktop"),
        )
        .unwrap();
    assert_eq!(
        moved.config.favorites,
        ["firefox.desktop", "org.gnome.Nautilus.desktop"]
    );

    let removed = store.unpin(moved.revision, "firefox.desktop").unwrap();
    assert_eq!(removed.config.favorites, ["org.gnome.Nautilus.desktop"]);
}

#[test]
fn opening_corrupt_state_quarantines_it_and_recovers_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    fs::write(&path, "{not-json").unwrap();

    let store = ConfigStore::open(&path).unwrap();
    let recovery = store.recovery().expect("corrupt file must be reported");

    assert!(recovery.quarantined_path.exists());
    assert_eq!(
        fs::read_to_string(recovery.quarantined_path).unwrap(),
        "{not-json"
    );
    assert_eq!(store.snapshot().config, Config::default());
    assert!(path.exists());
}

#[test]
fn opening_a_future_schema_preserves_it_instead_of_quarantining_it_as_corrupt() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let future = serde_json::json!({
        "revision": 9,
        "config": {
            "schemaVersion": 2,
            "seeded": true,
            "favorites": [],
            "display": {"mode": "primary", "connectorByAdapter": {}},
            "geometry": {
                "edge": "bottom", "lengthMode": "fit", "customLengthPercent": 60,
                "alignment": "center", "iconSize": 48
            },
            "behavior": {
                "visibility": "dodge", "revealDelayMs": 160, "hideDelayMs": 300,
                "shortcut": "<Super><Alt>d"
            },
            "appearance": {
                "colorScheme": "system", "accent": {"mode": "system"},
                "opacityPercent": 88, "cornerRadius": 16
            }
        }
    });
    let bytes = serde_json::to_vec_pretty(&future).unwrap();
    fs::write(&path, &bytes).unwrap();

    let error = ConfigStore::open(&path).unwrap_err();

    assert!(matches!(error, StoreError::UnsupportedSchema(2)));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(!path.with_extension("json.corrupt").exists());
}

#[test]
fn opening_an_unwrapped_schema_v1_document_migrates_it_to_a_revisioned_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let legacy = Config {
        seeded: true,
        geometry: Geometry {
            icon_size: 56,
            ..Geometry::default()
        },
        ..Config::default()
    };
    fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let store = ConfigStore::open(&path).unwrap();
    assert_eq!(store.snapshot().revision, 0);
    assert_eq!(store.snapshot().config, legacy);

    let migrated: Snapshot = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(migrated, store.snapshot());
    assert!(!path.with_extension("json.corrupt").exists());
}

#[test]
fn import_validates_before_replacing_and_keeps_a_rollback_copy() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let store = ConfigStore::open(&path).unwrap();
    let initial = store.pin(0, "firefox.desktop", None).unwrap();

    let imported = Config {
        seeded: true,
        favorites: vec!["org.gnome.Ptyxis.desktop".into()],
        ..Config::default()
    };
    let json = serde_json::to_string_pretty(&imported).unwrap();
    let snapshot = store.import(initial.revision, &json).unwrap();

    assert_eq!(snapshot.config, imported);
    let backup_path = path.with_extension("json.bak");
    let backup: Snapshot = serde_json::from_slice(&fs::read(&backup_path).unwrap()).unwrap();
    assert_eq!(backup, initial);

    let backup_before_stale = fs::read(&backup_path).unwrap();
    assert!(store.import(initial.revision, &json).is_err());
    assert_eq!(fs::read(&backup_path).unwrap(), backup_before_stale);

    let before_invalid = store.snapshot();
    assert!(store.import(before_invalid.revision, "{}").is_err());
    assert_eq!(store.snapshot(), before_invalid);
}

#[test]
fn shared_adapter_fixtures_match_the_rust_contract() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_dir.join("manifest.json")).unwrap()).unwrap();

    for fixture in manifest["valid"].as_array().unwrap() {
        let bytes = fs::read(fixture_dir.join(fixture.as_str().unwrap())).unwrap();
        let config: Config = serde_json::from_slice(&bytes).unwrap();
        assert!(
            config.validate().is_ok(),
            "fixture {fixture} should be valid"
        );
    }
    for fixture in manifest["invalid"].as_array().unwrap() {
        let bytes = fs::read(fixture_dir.join(fixture.as_str().unwrap())).unwrap();
        let config: Config = serde_json::from_slice(&bytes).unwrap();
        assert!(
            config.validate().is_err(),
            "fixture {fixture} should be invalid"
        );
    }
}
