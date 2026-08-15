// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;

use prismatic_core::ConfigStore;
use prismatic_service::{
    AdapterStatus, AppCatalog, DesktopAction, Service, ServiceError, SystemAppCatalog,
};

#[derive(Clone)]
struct FakeCatalog {
    installed: HashSet<String>,
}

impl FakeCatalog {
    fn new(ids: &[&str]) -> Self {
        Self {
            installed: ids.iter().map(|id| (*id).to_string()).collect(),
        }
    }
}

impl AppCatalog for FakeCatalog {
    fn is_installed(&self, desktop_id: &str) -> bool {
        self.installed.contains(desktop_id)
    }
}

#[test]
fn first_start_seeds_one_installed_fedora_launcher_per_role() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();
    let catalog = FakeCatalog::new(&[
        "firefox.desktop",
        "org.gnome.Nautilus.desktop",
        "org.kde.dolphin.desktop",
        "org.gnome.Ptyxis.desktop",
        "org.gnome.Software.desktop",
    ]);

    let service = Service::new(store, catalog).unwrap();
    let snapshot = service.snapshot();

    assert!(snapshot.config.seeded);
    assert_eq!(
        snapshot.config.favorites,
        [
            "firefox.desktop",
            "org.gnome.Nautilus.desktop",
            "org.gnome.Ptyxis.desktop",
            "org.gnome.Software.desktop",
        ]
    );
}

#[test]
fn existing_configuration_is_never_seeded_again() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let store = ConfigStore::open(&path).unwrap();
    let catalog = FakeCatalog::new(&["firefox.desktop"]);
    let service = Service::new(store, catalog.clone()).unwrap();
    let revision = service.snapshot().revision;
    service.unpin(revision, "firefox.desktop").unwrap();
    drop(service);

    let reopened = Service::new(ConfigStore::open(path).unwrap(), catalog).unwrap();
    assert!(reopened.snapshot().config.seeded);
    assert!(reopened.snapshot().config.favorites.is_empty());
}

#[test]
fn pin_rejects_launchers_that_are_not_installed() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();
    let service = Service::new(store, FakeCatalog::new(&[])).unwrap();
    let revision = service.snapshot().revision;

    let error = service
        .pin(revision, "untrusted.desktop", None)
        .unwrap_err();

    assert!(matches!(
        error,
        ServiceError::UnknownDesktopEntry(id) if id == "untrusted.desktop"
    ));
}

#[test]
fn snapshot_replacement_rejects_uninstalled_favorites() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();
    let service = Service::new(store, FakeCatalog::new(&[])).unwrap();
    let snapshot = service.snapshot();
    let mut config = snapshot.config;
    config.favorites = vec!["untrusted.desktop".into()];

    let error = service
        .replace_snapshot(snapshot.revision, &serde_json::to_string(&config).unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        ServiceError::UnknownDesktopEntry(id) if id == "untrusted.desktop"
    ));
}

#[test]
fn import_rejects_uninstalled_favorites_without_creating_a_rollback() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let store = ConfigStore::open(&path).unwrap();
    let service = Service::new(store, FakeCatalog::new(&[])).unwrap();
    let before = service.snapshot();
    let mut config = before.config.clone();
    config.favorites = vec!["untrusted.desktop".into()];

    let error = service
        .import(before.revision, &serde_json::to_string(&config).unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        ServiceError::UnknownDesktopEntry(id) if id == "untrusted.desktop"
    ));
    assert_eq!(service.snapshot(), before);
    assert!(!path.with_extension("json.bak").exists());
}

#[test]
fn adapters_register_and_update_structured_runtime_status() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();
    let service = Service::new(store, FakeCatalog::new(&[])).unwrap();
    let initial = AdapterStatus {
        active: true,
        version: "50.3".into(),
        capabilities: vec!["dodge".into(), "pressure-barrier".into()],
        outputs: vec!["DP-1".into()],
        message: None,
    };

    let returned = service.register_adapter("gnome", initial.clone()).unwrap();
    assert_eq!(returned, service.snapshot());
    assert_eq!(service.adapter_statuses().get("gnome"), Some(&initial));

    let updated = AdapterStatus {
        active: false,
        message: Some("extension disabled".into()),
        ..initial
    };
    service
        .update_adapter_status("gnome", updated.clone())
        .unwrap();
    assert_eq!(service.adapter_statuses().get("gnome"), Some(&updated));
}

#[test]
fn adapter_ids_are_closed_to_the_supported_desktops() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();
    let service = Service::new(store, FakeCatalog::new(&[])).unwrap();

    let error = service
        .register_adapter("sway", AdapterStatus::default())
        .unwrap_err();

    assert!(matches!(
        error,
        ServiceError::UnsupportedAdapter(id) if id == "sway"
    ));
}

#[test]
fn desktop_actions_are_listed_and_invoked_only_for_installed_launchers() {
    let directory = tempfile::tempdir().unwrap();
    let applications = directory.path().join("applications");
    std::fs::create_dir(&applications).unwrap();
    std::fs::write(
        applications.join("actions.desktop"),
        "[Desktop Entry]\nType=Application\nName=Actions\nExec=/usr/bin/true\nActions=Private;\n\n[Desktop Action Private]\nName=New Private Window\nExec=/usr/bin/true\n",
    )
    .unwrap();
    let catalog = SystemAppCatalog::from_application_directories(vec![applications]);
    let store = ConfigStore::open(directory.path().join("config.json")).unwrap();
    let service = Service::new(store, catalog).unwrap();

    assert_eq!(
        service.desktop_actions("actions.desktop").unwrap(),
        vec![DesktopAction {
            id: "Private".into(),
            name: "New Private Window".into(),
        }]
    );
    service
        .launch_desktop_action("actions.desktop", "Private")
        .unwrap();
    assert!(matches!(
        service.launch_desktop_action("actions.desktop", "Missing"),
        Err(ServiceError::UnknownDesktopAction { .. })
    ));
    assert!(matches!(
        service.desktop_actions("missing.desktop"),
        Err(ServiceError::UnknownDesktopEntry(_))
    ));
    assert!(matches!(
        service.desktop_actions("../actions.desktop"),
        Err(ServiceError::UnknownDesktopEntry(_))
    ));
}
