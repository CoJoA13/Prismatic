// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::path::PathBuf;

use prismatic_service::{DBUS_NAME, ServiceHost};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or("neither XDG_CONFIG_HOME nor HOME is available")?;
    let _host = ServiceHost::start(config_home.join("prismatic/config.json")).await?;

    eprintln!("prismatic-service: ready on {DBUS_NAME}");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
