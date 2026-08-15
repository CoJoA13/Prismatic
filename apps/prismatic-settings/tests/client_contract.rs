// SPDX-License-Identifier: GPL-3.0-or-later

use prismatic_service::{DBUS_INTERFACE, DBUS_PATH, DbusApi};
use prismatic_settings::ServiceClient;

#[test]
fn settings_client_round_trips_typed_configuration() {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        return;
    }

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let bus_name = format!(
        "io.github.CoJoA13.Prismatic.SettingsTest.p{}",
        std::process::id()
    );
    let server = runtime.block_on(async {
        zbus::connection::Builder::session()
            .unwrap()
            .name(bus_name.as_str())
            .unwrap()
            .serve_at(
                DBUS_PATH,
                DbusApi::open(directory.path().join("config.json")).unwrap(),
            )
            .unwrap()
            .build()
            .await
            .unwrap()
    });

    let client = ServiceClient::connect_to(&bus_name, DBUS_PATH, DBUS_INTERFACE).unwrap();
    let mut snapshot = client.get_snapshot().unwrap();
    snapshot.config.geometry.icon_size = 72;

    let saved = client
        .replace_snapshot(snapshot.revision, &snapshot.config)
        .unwrap();
    assert_eq!(saved.revision, snapshot.revision + 1);
    assert_eq!(saved.config.geometry.icon_size, 72);

    runtime.block_on(async {
        drop(server);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    let restarted = runtime.block_on(async {
        zbus::connection::Builder::session()
            .unwrap()
            .name(bus_name.as_str())
            .unwrap()
            .serve_at(
                DBUS_PATH,
                DbusApi::open(directory.path().join("config.json")).unwrap(),
            )
            .unwrap()
            .build()
            .await
            .unwrap()
    });
    let reconnected = client.get_snapshot().unwrap();
    assert_eq!(reconnected, saved);

    drop(client);
    runtime.block_on(async { drop(restarted) });
}
