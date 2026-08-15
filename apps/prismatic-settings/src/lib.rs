// SPDX-License-Identifier: GPL-3.0-or-later

use prismatic_core::{Config, Snapshot};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopSession {
    Gnome,
    Plasma,
}

impl DesktopSession {
    pub fn detect(current_desktop: &str, session_type: &str) -> Result<Self, SessionError> {
        if !session_type.eq_ignore_ascii_case("wayland") {
            return Err(SessionError::UnsupportedSessionType(session_type.into()));
        }

        let desktops = current_desktop.split(':');
        for desktop in desktops {
            if desktop.eq_ignore_ascii_case("gnome")
                || desktop.eq_ignore_ascii_case("gnome-classic")
            {
                return Ok(Self::Gnome);
            }
            if desktop.eq_ignore_ascii_case("kde") || desktop.eq_ignore_ascii_case("plasma") {
                return Ok(Self::Plasma);
            }
        }
        Err(SessionError::UnsupportedDesktop(current_desktop.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionError {
    #[error("unsupported session type: {0}; Prismatic v1 requires Wayland")]
    UnsupportedSessionType(String),
    #[error("unsupported desktop: {0}; Prismatic v1 supports GNOME and Plasma")]
    UnsupportedDesktop(String),
}

pub struct ServiceClient {
    connection: zbus::blocking::Connection,
    destination: String,
    path: String,
    interface: String,
}

impl ServiceClient {
    pub fn connect_to(destination: &str, path: &str, interface: &str) -> Result<Self, ClientError> {
        Ok(Self {
            connection: zbus::blocking::Connection::session()?,
            destination: destination.into(),
            path: path.into(),
            interface: interface.into(),
        })
    }

    fn proxy(&self) -> Result<zbus::blocking::Proxy<'_>, ClientError> {
        Ok(zbus::blocking::Proxy::new(
            &self.connection,
            self.destination.as_str(),
            self.path.as_str(),
            self.interface.as_str(),
        )?)
    }

    pub fn get_snapshot(&self) -> Result<Snapshot, ClientError> {
        let (revision, json): (u64, String) = self.proxy()?.call("GetSnapshot", &())?;
        decode_snapshot(revision, &json)
    }

    pub fn replace_snapshot(
        &self,
        expected_revision: u64,
        config: &Config,
    ) -> Result<Snapshot, ClientError> {
        let json = serde_json::to_string(config)?;
        let (revision, json): (u64, String) = self
            .proxy()?
            .call("ReplaceSnapshot", &(expected_revision, json.as_str()))?;
        decode_snapshot(revision, &json)
    }

    pub fn pin(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: Option<&str>,
    ) -> Result<Snapshot, ClientError> {
        let (revision, json): (u64, String) = self.proxy()?.call(
            "Pin",
            &(expected_revision, desktop_id, before_id.unwrap_or("")),
        )?;
        decode_snapshot(revision, &json)
    }

    pub fn unpin(&self, expected_revision: u64, desktop_id: &str) -> Result<Snapshot, ClientError> {
        let (revision, json): (u64, String) = self
            .proxy()?
            .call("Unpin", &(expected_revision, desktop_id))?;
        decode_snapshot(revision, &json)
    }

    pub fn move_favorite(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: Option<&str>,
    ) -> Result<Snapshot, ClientError> {
        let (revision, json): (u64, String) = self.proxy()?.call(
            "Move",
            &(expected_revision, desktop_id, before_id.unwrap_or("")),
        )?;
        decode_snapshot(revision, &json)
    }

    pub fn import(&self, expected_revision: u64, json: &str) -> Result<Snapshot, ClientError> {
        let (revision, json): (u64, String) =
            self.proxy()?.call("Import", &(expected_revision, json))?;
        decode_snapshot(revision, &json)
    }

    pub fn export(&self) -> Result<String, ClientError> {
        Ok(self.proxy()?.call("Export", &())?)
    }

    pub fn adapter_statuses(&self) -> Result<serde_json::Value, ClientError> {
        let json: String = self.proxy()?.call("GetAdapterStatuses", &())?;
        Ok(serde_json::from_str(&json)?)
    }
}

fn decode_snapshot(revision: u64, json: &str) -> Result<Snapshot, ClientError> {
    let config = serde_json::from_str(json)?;
    Ok(Snapshot { revision, config })
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Dbus(#[from] zbus::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
