// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;

use futures_util::StreamExt;
use prismatic_service::{
    DBUS_INTERFACE, DBUS_PATH, DesktopAction, ServiceHost, StartupError, SystemAppCatalog,
};

#[tokio::test]
async fn dbus_contract_exposes_snapshots_registration_and_named_errors() {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let applications = directory.path().join("applications");
    std::fs::create_dir(&applications).unwrap();
    std::fs::write(
        applications.join("one.desktop"),
        "[Desktop Entry]\nType=Application\nName=One\nExec=/usr/bin/true\nActions=Private;\n\n[Desktop Action Private]\nName=New Private Window\nExec=/usr/bin/true\n",
    )
    .unwrap();
    std::fs::write(
        applications.join("two.desktop"),
        "[Desktop Entry]\nType=Application\n",
    )
    .unwrap();
    let catalog = SystemAppCatalog::from_application_directories(vec![applications]);
    let bus_name = format!("io.github.CoJoA13.Prismatic.Test.p{}", std::process::id());
    let config_path = directory.path().join("config.json");
    let server = ServiceHost::start_with_catalog(&config_path, catalog.clone(), bus_name.as_str())
        .await
        .unwrap();
    let duplicate_name = format!("{bus_name}.Duplicate");
    let duplicate =
        ServiceHost::start_with_catalog(&config_path, catalog.clone(), duplicate_name.as_str())
            .await;
    assert!(matches!(duplicate, Err(StartupError::AlreadyRunning)));
    let client = zbus::Connection::session().await.unwrap();
    let proxy = zbus::Proxy::new(&client, bus_name.as_str(), DBUS_PATH, DBUS_INTERFACE)
        .await
        .unwrap();

    let (revision, json): (u64, String) = proxy.call("GetSnapshot", &()).await.unwrap();
    let config: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(config["schemaVersion"], 1);

    let introspection = zbus::Proxy::new(
        &client,
        bus_name.as_str(),
        DBUS_PATH,
        "org.freedesktop.DBus.Introspectable",
    )
    .await
    .unwrap();
    let xml: String = introspection.call("Introspect", &()).await.unwrap();
    for member in [
        "GetSnapshot",
        "ReplaceSnapshot",
        "Pin",
        "Unpin",
        "Move",
        "Import",
        "Export",
        "RegisterAdapter",
        "UpdateAdapterStatus",
        "GetDesktopActions",
        "LaunchDesktopAction",
        "SnapshotChanged",
        "AdapterStatusChanged",
    ] {
        assert!(
            xml.contains(&format!("name=\"{member}\"")),
            "missing {member}"
        );
    }

    let actions_json: String = proxy
        .call("GetDesktopActions", &("one.desktop",))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<DesktopAction>>(&actions_json).unwrap(),
        vec![DesktopAction {
            id: "Private".into(),
            name: "New Private Window".into(),
        }]
    );
    proxy
        .call::<_, _, ()>("LaunchDesktopAction", &("one.desktop", "Private"))
        .await
        .unwrap();
    let unknown_action = proxy
        .call::<_, _, ()>("LaunchDesktopAction", &("one.desktop", "Missing"))
        .await
        .unwrap_err();
    assert!(unknown_action.to_string().contains("UnknownDesktopAction"));

    let mut replacement = config.clone();
    replacement["geometry"]["iconSize"] = 72.into();
    let replacement = replacement.to_string();
    let (mut revision, json): (u64, String) = proxy
        .call("ReplaceSnapshot", &(revision, replacement.as_str()))
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json).unwrap()["geometry"]["iconSize"],
        72
    );

    let stale = proxy
        .call::<_, _, (u64, String)>("ReplaceSnapshot", &(revision - 1, replacement.as_str()))
        .await
        .unwrap_err();
    assert!(stale.to_string().contains("RevisionConflict"));

    let pinned: (u64, String) = proxy
        .call("Pin", &(revision, "one.desktop", ""))
        .await
        .unwrap();
    revision = pinned.0;
    let pinned: (u64, String) = proxy
        .call("Pin", &(revision, "two.desktop", ""))
        .await
        .unwrap();
    revision = pinned.0;
    let moved: (u64, String) = proxy
        .call("Move", &(revision, "two.desktop", "one.desktop"))
        .await
        .unwrap();
    revision = moved.0;
    let moved_config: serde_json::Value = serde_json::from_str(&moved.1).unwrap();
    assert_eq!(
        moved_config["favorites"],
        serde_json::json!(["two.desktop", "one.desktop"])
    );
    let unpinned: (u64, String) = proxy
        .call("Unpin", &(revision, "one.desktop"))
        .await
        .unwrap();
    revision = unpinned.0;

    let exported: String = proxy.call("Export", &()).await.unwrap();
    let mut imported: serde_json::Value = serde_json::from_str(&exported).unwrap();
    imported["appearance"]["opacityPercent"] = 91.into();
    let imported = imported.to_string();
    let imported_reply: (u64, String) = proxy
        .call("Import", &(revision, imported.as_str()))
        .await
        .unwrap();
    revision = imported_reply.0;

    let invalid = proxy
        .call::<_, _, (u64, String)>("Pin", &(revision, "not-installed.desktop", ""))
        .await
        .unwrap_err();
    assert!(invalid.to_string().contains("UnknownDesktopEntry"));

    let mut invalid_config: serde_json::Value = serde_json::from_str(&imported).unwrap();
    invalid_config["geometry"]["iconSize"] = 200.into();
    let error = proxy
        .call::<_, _, (u64, String)>(
            "ReplaceSnapshot",
            &(revision, invalid_config.to_string().as_str()),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("InvalidConfig"));

    let mut future_config: serde_json::Value = serde_json::from_str(&imported).unwrap();
    future_config["schemaVersion"] = 2.into();
    let error = proxy
        .call::<_, _, (u64, String)>(
            "ReplaceSnapshot",
            &(revision, future_config.to_string().as_str()),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("UnsupportedSchema"));

    let mut snapshot_signals = proxy.receive_signal("SnapshotChanged").await.unwrap();
    let mut status_signals = proxy.receive_signal("AdapterStatusChanged").await.unwrap();

    let status = serde_json::json!({
        "active": true,
        "version": "50.3",
        "capabilities": ["dodge"],
        "outputs": ["DP-1"],
        "message": null
    })
    .to_string();
    let (registered_revision, _): (u64, String) = proxy
        .call("RegisterAdapter", &("gnome", status.as_str()))
        .await
        .unwrap();
    assert_eq!(registered_revision, revision);
    let signal = tokio::time::timeout(Duration::from_secs(1), status_signals.next())
        .await
        .unwrap()
        .unwrap();
    let (adapter_id, signal_status): (String, String) = signal.body().deserialize().unwrap();
    assert_eq!(adapter_id, "gnome");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&signal_status).unwrap(),
        serde_json::from_str::<serde_json::Value>(&status).unwrap()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), snapshot_signals.next())
            .await
            .is_err(),
        "registration must not emit an unchanged snapshot"
    );

    let inactive = status.replace("\"active\":true", "\"active\":false");
    proxy
        .call::<_, _, ()>("UpdateAdapterStatus", &("gnome", inactive.as_str()))
        .await
        .unwrap();
    let statuses: String = proxy.call("GetAdapterStatuses", &()).await.unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&statuses).unwrap()["gnome"]["active"],
        false
    );

    drop(server);
    let restarted_name = format!("{bus_name}.Restarted");
    let restarted = ServiceHost::start_with_catalog(&config_path, catalog, restarted_name.as_str())
        .await
        .unwrap();
    drop(restarted);
}
