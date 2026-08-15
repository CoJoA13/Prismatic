// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeMap;
use std::env;
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use prismatic_core::{Config, ConfigStore, Snapshot, StoreError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zbus::object_server::SignalEmitter;

pub const DBUS_NAME: &str = "io.github.CoJoA13.Prismatic.Service";
pub const DBUS_PATH: &str = "/io/github/CoJoA13/Prismatic";
pub const DBUS_INTERFACE: &str = "io.github.CoJoA13.Prismatic.Service1";

const DEFAULT_FAVORITE_ROLES: &[&[&str]] = &[
    &["firefox.desktop", "org.mozilla.firefox.desktop"],
    &["org.gnome.Nautilus.desktop", "org.kde.dolphin.desktop"],
    &[
        "org.gnome.Ptyxis.desktop",
        "org.gnome.Console.desktop",
        "org.kde.konsole.desktop",
    ],
    &["org.gnome.Software.desktop", "org.kde.discover.desktop"],
];

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("another Prismatic service instance is already running")]
    AlreadyRunning,
    #[error("could not acquire the service lock at {path}: {source}")]
    Lock {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Dbus(#[from] zbus::Error),
}

#[derive(Debug)]
pub struct ServiceHost {
    _connection: zbus::Connection,
    _instance_lock: File,
}

impl ServiceHost {
    pub async fn start(config_path: impl AsRef<Path>) -> Result<Self, StartupError> {
        Self::start_with_catalog(config_path, SystemAppCatalog::default(), DBUS_NAME).await
    }

    pub async fn start_with_catalog(
        config_path: impl AsRef<Path>,
        catalog: SystemAppCatalog,
        bus_name: &str,
    ) -> Result<Self, StartupError> {
        let config_path = config_path.as_ref();
        let lock_path = service_lock_path(config_path);
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| StartupError::Lock {
                path: lock_path.clone(),
                source,
            })?;
        }
        let instance_lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| StartupError::Lock {
                path: lock_path.clone(),
                source,
            })?;
        match instance_lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(StartupError::AlreadyRunning),
            Err(TryLockError::Error(source)) => {
                return Err(StartupError::Lock {
                    path: lock_path,
                    source,
                });
            }
        }

        let api = DbusApi::open_with_catalog(config_path, catalog)?;
        let connection = zbus::connection::Builder::session()?
            .serve_at(DBUS_PATH, api)?
            .name(bus_name)?
            .build()
            .await?;
        Ok(Self {
            _connection: connection,
            _instance_lock: instance_lock,
        })
    }
}

fn service_lock_path(config_path: &Path) -> PathBuf {
    let mut path = config_path.as_os_str().to_os_string();
    path.push(".lock");
    PathBuf::from(path)
}

pub trait AppCatalog: Send + Sync + 'static {
    fn is_installed(&self, desktop_id: &str) -> bool;
}

#[derive(Debug, Clone)]
pub struct SystemAppCatalog {
    application_directories: Vec<PathBuf>,
}

impl Default for SystemAppCatalog {
    fn default() -> Self {
        let mut directories = Vec::new();
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
        if let Some(data_home) = data_home {
            directories.push(data_home.join("applications"));
        }
        let data_dirs =
            env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        directories.extend(
            data_dirs
                .split(':')
                .filter(|entry| !entry.is_empty())
                .map(|entry| PathBuf::from(entry).join("applications")),
        );
        Self {
            application_directories: directories,
        }
    }
}

impl SystemAppCatalog {
    pub fn from_application_directories(application_directories: Vec<PathBuf>) -> Self {
        Self {
            application_directories,
        }
    }

    fn desktop_file(&self, desktop_id: &str) -> Option<PathBuf> {
        if !desktop_id.ends_with(".desktop")
            || desktop_id.contains('/')
            || desktop_id.contains("..")
            || desktop_id.chars().any(char::is_control)
        {
            return None;
        }
        self.application_directories
            .iter()
            .map(|directory| directory.join(desktop_id))
            .find(|path| path.is_file())
    }
}

impl AppCatalog for SystemAppCatalog {
    fn is_installed(&self, desktop_id: &str) -> bool {
        self.desktop_file(desktop_id).is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopAction {
    pub id: String,
    pub name: String,
}

struct DesktopAppInfo(*mut c_void);

impl DesktopAppInfo {
    fn open(path: &Path) -> Option<Self> {
        let filename = CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
        // SAFETY: `filename` is a live NUL-terminated byte string for this call. A non-null
        // result owns one GObject reference, released by `Drop` below.
        let info = unsafe { g_desktop_app_info_new_from_filename(filename.as_ptr()) };
        (!info.is_null()).then_some(Self(info))
    }

    fn actions(&self) -> Vec<DesktopAction> {
        // SAFETY: `self.0` is a live GDesktopAppInfo. GLib owns the returned terminated array
        // for the lifetime of the object.
        let mut action = unsafe { g_desktop_app_info_list_actions(self.0) };
        let mut actions = Vec::new();
        if action.is_null() {
            return actions;
        }
        loop {
            // SAFETY: GLib documents the list as NUL-terminated and object-owned.
            let action_id = unsafe { *action };
            if action_id.is_null() {
                break;
            }
            // SAFETY: the action ID comes from this same GDesktopAppInfo and is therefore valid
            // input. The returned label is newly allocated and freed below.
            let label = unsafe { g_desktop_app_info_get_action_name(self.0, action_id) };
            if !label.is_null() {
                // SAFETY: action IDs and labels are documented NUL-terminated UTF-8 strings.
                let id = unsafe { CStr::from_ptr(action_id) }
                    .to_string_lossy()
                    .into_owned();
                // SAFETY: see above; `label` remains live until `g_free`.
                let name = unsafe { CStr::from_ptr(label) }
                    .to_string_lossy()
                    .into_owned();
                // SAFETY: `label` is a GLib allocation returned with full ownership.
                unsafe { g_free(label.cast()) };
                if !id.is_empty() && !name.trim().is_empty() {
                    actions.push(DesktopAction { id, name });
                }
            }
            // SAFETY: the current item was non-null, so advancing reaches either another item
            // or the documented terminating null pointer.
            action = unsafe { action.add(1) };
        }
        actions
    }

    fn launch(&self, action_id: &str) -> bool {
        let Ok(action_id) = CString::new(action_id) else {
            return false;
        };
        let known = self
            .actions()
            .iter()
            .any(|action| action.id.as_bytes() == action_id.as_bytes());
        if !known {
            return false;
        }
        // SAFETY: the ID was compared with the list returned by this object, as required by
        // GLib. A null launch context is explicitly supported.
        unsafe {
            g_desktop_app_info_launch_action(self.0, action_id.as_ptr(), std::ptr::null_mut())
        };
        true
    }
}

impl Drop for DesktopAppInfo {
    fn drop(&mut self) {
        // SAFETY: `self.0` owns exactly one live GObject reference.
        unsafe { g_object_unref(self.0) };
    }
}

#[link(name = "gio-2.0")]
unsafe extern "C" {
    fn g_desktop_app_info_new_from_filename(filename: *const c_char) -> *mut c_void;
    fn g_desktop_app_info_list_actions(info: *mut c_void) -> *const *const c_char;
    fn g_desktop_app_info_get_action_name(
        info: *mut c_void,
        action_name: *const c_char,
    ) -> *mut c_char;
    fn g_desktop_app_info_launch_action(
        info: *mut c_void,
        action_name: *const c_char,
        launch_context: *mut c_void,
    );
}

#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_object_unref(object: *mut c_void);
}

#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_free(memory: *mut c_void);
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterStatus {
    pub active: bool,
    pub version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct Service<C: AppCatalog> {
    store: ConfigStore,
    catalog: C,
    adapters: Mutex<BTreeMap<String, AdapterStatus>>,
}

impl<C: AppCatalog> Service<C> {
    pub fn new(store: ConfigStore, catalog: C) -> Result<Self, ServiceError> {
        let service = Self {
            store,
            catalog,
            adapters: Mutex::new(BTreeMap::new()),
        };
        service.seed_favorites_once()?;
        Ok(service)
    }

    fn seed_favorites_once(&self) -> Result<(), ServiceError> {
        let snapshot = self.store.snapshot();
        if snapshot.config.seeded {
            return Ok(());
        }

        let mut config = snapshot.config;
        config.seeded = true;
        config.favorites = DEFAULT_FAVORITE_ROLES
            .iter()
            .filter_map(|candidates| {
                candidates
                    .iter()
                    .find(|candidate| self.catalog.is_installed(candidate))
                    .map(|candidate| (*candidate).to_string())
            })
            .collect();
        self.store.replace(snapshot.revision, config)?;
        Ok(())
    }

    pub fn snapshot(&self) -> Snapshot {
        self.store.snapshot()
    }

    pub fn replace_snapshot(
        &self,
        expected_revision: u64,
        json: &str,
    ) -> Result<Snapshot, ServiceError> {
        let config: Config = serde_json::from_str(json)?;
        self.validate_favorites(&config)?;
        Ok(self.store.replace(expected_revision, config)?)
    }

    pub fn pin(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: Option<&str>,
    ) -> Result<Snapshot, ServiceError> {
        if !self.catalog.is_installed(desktop_id) {
            return Err(ServiceError::UnknownDesktopEntry(desktop_id.into()));
        }
        Ok(self.store.pin(expected_revision, desktop_id, before_id)?)
    }

    pub fn unpin(
        &self,
        expected_revision: u64,
        desktop_id: &str,
    ) -> Result<Snapshot, ServiceError> {
        Ok(self.store.unpin(expected_revision, desktop_id)?)
    }

    pub fn move_favorite(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: Option<&str>,
    ) -> Result<Snapshot, ServiceError> {
        Ok(self
            .store
            .move_favorite(expected_revision, desktop_id, before_id)?)
    }

    pub fn import(&self, expected_revision: u64, json: &str) -> Result<Snapshot, ServiceError> {
        let config: Config = serde_json::from_str(json)?;
        self.validate_favorites(&config)?;
        Ok(self.store.import(expected_revision, json)?)
    }

    pub fn export(&self) -> Result<String, ServiceError> {
        Ok(self.store.export()?)
    }

    pub fn register_adapter(
        &self,
        adapter_id: &str,
        status: AdapterStatus,
    ) -> Result<Snapshot, ServiceError> {
        validate_adapter_id(adapter_id)?;
        self.adapters
            .lock()
            .expect("adapter registry poisoned")
            .insert(adapter_id.into(), status);
        Ok(self.snapshot())
    }

    pub fn update_adapter_status(
        &self,
        adapter_id: &str,
        status: AdapterStatus,
    ) -> Result<(), ServiceError> {
        validate_adapter_id(adapter_id)?;
        self.adapters
            .lock()
            .expect("adapter registry poisoned")
            .insert(adapter_id.into(), status);
        Ok(())
    }

    pub fn adapter_statuses(&self) -> BTreeMap<String, AdapterStatus> {
        self.adapters
            .lock()
            .expect("adapter registry poisoned")
            .clone()
    }

    fn validate_favorites(&self, config: &Config) -> Result<(), ServiceError> {
        config.validate().map_err(StoreError::from)?;
        if let Some(desktop_id) = config
            .favorites
            .iter()
            .find(|desktop_id| !self.catalog.is_installed(desktop_id))
        {
            return Err(ServiceError::UnknownDesktopEntry(desktop_id.clone()));
        }
        Ok(())
    }
}

impl Service<SystemAppCatalog> {
    pub fn desktop_actions(&self, desktop_id: &str) -> Result<Vec<DesktopAction>, ServiceError> {
        let path = self
            .catalog
            .desktop_file(desktop_id)
            .ok_or_else(|| ServiceError::UnknownDesktopEntry(desktop_id.into()))?;
        let info = DesktopAppInfo::open(&path)
            .ok_or_else(|| ServiceError::UnknownDesktopEntry(desktop_id.into()))?;
        Ok(info.actions())
    }

    pub fn launch_desktop_action(
        &self,
        desktop_id: &str,
        action_id: &str,
    ) -> Result<(), ServiceError> {
        let path = self
            .catalog
            .desktop_file(desktop_id)
            .ok_or_else(|| ServiceError::UnknownDesktopEntry(desktop_id.into()))?;
        let info = DesktopAppInfo::open(&path)
            .ok_or_else(|| ServiceError::UnknownDesktopEntry(desktop_id.into()))?;
        if info.launch(action_id) {
            Ok(())
        } else {
            Err(ServiceError::UnknownDesktopAction {
                desktop_id: desktop_id.into(),
                action_id: action_id.into(),
            })
        }
    }
}

fn validate_adapter_id(adapter_id: &str) -> Result<(), ServiceError> {
    if matches!(adapter_id, "gnome" | "plasma") {
        Ok(())
    } else {
        Err(ServiceError::UnsupportedAdapter(adapter_id.into()))
    }
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("desktop entry is not installed: {0}")]
    UnknownDesktopEntry(String),
    #[error("desktop entry {desktop_id} has no action named {action_id}")]
    UnknownDesktopAction {
        desktop_id: String,
        action_id: String,
    },
    #[error("unsupported adapter: {0}")]
    UnsupportedAdapter(String),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub struct DbusApi {
    service: Service<SystemAppCatalog>,
}

impl DbusApi {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, ServiceError> {
        Self::open_with_catalog(path, SystemAppCatalog::default())
    }

    pub fn open_with_catalog(
        path: impl AsRef<std::path::Path>,
        catalog: SystemAppCatalog,
    ) -> Result<Self, ServiceError> {
        Ok(Self {
            service: Service::new(ConfigStore::open(path)?, catalog)?,
        })
    }

    fn snapshot_payload(&self) -> Result<(u64, String), DbusError> {
        let snapshot = self.service.snapshot();
        Ok((
            snapshot.revision,
            serde_json::to_string(&snapshot.config).map_err(ServiceError::from)?,
        ))
    }

    async fn emit_snapshot(
        &self,
        emitter: &SignalEmitter<'_>,
        snapshot: Snapshot,
    ) -> Result<(u64, String), DbusError> {
        let json = serde_json::to_string(&snapshot.config).map_err(ServiceError::from)?;
        Self::snapshot_changed(emitter, snapshot.revision, &json).await?;
        Ok((snapshot.revision, json))
    }
}

#[zbus::interface(name = "io.github.CoJoA13.Prismatic.Service1")]
impl DbusApi {
    #[zbus(name = "GetSnapshot", out_args("revision", "config"))]
    async fn get_snapshot(&self) -> Result<(u64, String), DbusError> {
        self.snapshot_payload()
    }

    #[zbus(name = "ReplaceSnapshot", out_args("revision", "config"))]
    async fn replace_snapshot(
        &self,
        expected_revision: u64,
        config: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u64, String), DbusError> {
        let snapshot = self
            .service
            .replace_snapshot(expected_revision, config)
            .map_err(DbusError::from)?;
        self.emit_snapshot(&emitter, snapshot).await
    }

    #[zbus(name = "Pin", out_args("revision", "config"))]
    async fn pin(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u64, String), DbusError> {
        let before_id = (!before_id.is_empty()).then_some(before_id);
        let snapshot = self
            .service
            .pin(expected_revision, desktop_id, before_id)
            .map_err(DbusError::from)?;
        self.emit_snapshot(&emitter, snapshot).await
    }

    #[zbus(name = "Unpin", out_args("revision", "config"))]
    async fn unpin(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u64, String), DbusError> {
        let snapshot = self
            .service
            .unpin(expected_revision, desktop_id)
            .map_err(DbusError::from)?;
        self.emit_snapshot(&emitter, snapshot).await
    }

    #[zbus(name = "Move", out_args("revision", "config"))]
    async fn move_favorite(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u64, String), DbusError> {
        let before_id = (!before_id.is_empty()).then_some(before_id);
        let snapshot = self
            .service
            .move_favorite(expected_revision, desktop_id, before_id)
            .map_err(DbusError::from)?;
        self.emit_snapshot(&emitter, snapshot).await
    }

    #[zbus(name = "Import", out_args("revision", "config"))]
    async fn import(
        &self,
        expected_revision: u64,
        config: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u64, String), DbusError> {
        let snapshot = self
            .service
            .import(expected_revision, config)
            .map_err(DbusError::from)?;
        self.emit_snapshot(&emitter, snapshot).await
    }

    #[zbus(name = "Export")]
    async fn export(&self) -> Result<String, DbusError> {
        self.service.export().map_err(DbusError::from)
    }

    #[zbus(name = "GetDesktopActions")]
    async fn get_desktop_actions(&self, desktop_id: &str) -> Result<String, DbusError> {
        let actions = self.service.desktop_actions(desktop_id)?;
        serde_json::to_string(&actions)
            .map_err(ServiceError::from)
            .map_err(DbusError::from)
    }

    #[zbus(name = "LaunchDesktopAction")]
    async fn launch_desktop_action(
        &self,
        desktop_id: &str,
        action_id: &str,
    ) -> Result<(), DbusError> {
        self.service
            .launch_desktop_action(desktop_id, action_id)
            .map_err(DbusError::from)
    }

    #[zbus(name = "RegisterAdapter", out_args("revision", "config"))]
    async fn register_adapter(
        &self,
        adapter_id: &str,
        status: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(u64, String), DbusError> {
        let status: AdapterStatus = serde_json::from_str(status).map_err(ServiceError::from)?;
        let snapshot = self
            .service
            .register_adapter(adapter_id, status.clone())
            .map_err(DbusError::from)?;
        let status_json = serde_json::to_string(&status).map_err(ServiceError::from)?;
        Self::adapter_status_changed(&emitter, adapter_id, &status_json).await?;
        let config = serde_json::to_string(&snapshot.config).map_err(ServiceError::from)?;
        Ok((snapshot.revision, config))
    }

    #[zbus(name = "UpdateAdapterStatus")]
    async fn update_adapter_status(
        &self,
        adapter_id: &str,
        status: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), DbusError> {
        let status: AdapterStatus = serde_json::from_str(status).map_err(ServiceError::from)?;
        self.service
            .update_adapter_status(adapter_id, status.clone())
            .map_err(DbusError::from)?;
        let status_json = serde_json::to_string(&status).map_err(ServiceError::from)?;
        Self::adapter_status_changed(&emitter, adapter_id, &status_json).await?;
        Ok(())
    }

    #[zbus(name = "GetAdapterStatuses")]
    async fn get_adapter_statuses(&self) -> Result<String, DbusError> {
        serde_json::to_string(&self.service.adapter_statuses())
            .map_err(ServiceError::from)
            .map_err(DbusError::from)
    }

    #[zbus(signal, name = "SnapshotChanged")]
    async fn snapshot_changed(
        emitter: &SignalEmitter<'_>,
        revision: u64,
        config: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal, name = "AdapterStatusChanged")]
    async fn adapter_status_changed(
        emitter: &SignalEmitter<'_>,
        adapter_id: &str,
        status: &str,
    ) -> zbus::Result<()>;
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "io.github.CoJoA13.Prismatic.Error")]
pub enum DbusError {
    InvalidConfig(String),
    RevisionConflict(String),
    UnknownDesktopEntry(String),
    UnknownDesktopAction(String),
    UnsupportedSchema(String),
    UnsupportedAdapter(String),
    InvalidRequest(String),
    Failed(String),
    #[zbus(error)]
    ZBus(zbus::Error),
}

impl From<ServiceError> for DbusError {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::UnknownDesktopEntry(id) => Self::UnknownDesktopEntry(id),
            ServiceError::UnknownDesktopAction {
                desktop_id,
                action_id,
            } => Self::UnknownDesktopAction(format!("{desktop_id}: {action_id}")),
            ServiceError::UnsupportedAdapter(id) => Self::UnsupportedAdapter(id),
            ServiceError::Json(error) => Self::InvalidRequest(error.to_string()),
            ServiceError::Store(StoreError::RevisionConflict { expected, actual }) => {
                Self::RevisionConflict(format!("expected {expected}, actual {actual}"))
            }
            ServiceError::Store(StoreError::UnsupportedSchema(version)) => {
                Self::UnsupportedSchema(format!("unsupported schema version {version}"))
            }
            ServiceError::Store(StoreError::InvalidConfig(error))
                if error.field() == "schemaVersion" =>
            {
                Self::UnsupportedSchema(error.to_string())
            }
            ServiceError::Store(StoreError::InvalidConfig(error)) => {
                Self::InvalidConfig(error.to_string())
            }
            ServiceError::Store(StoreError::Json(error)) => Self::InvalidConfig(error.to_string()),
            other => Self::Failed(other.to_string()),
        }
    }
}
