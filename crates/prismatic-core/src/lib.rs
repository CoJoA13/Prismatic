// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    Primary,
    Selected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisplayConfig {
    pub mode: DisplayMode,
    #[serde(default)]
    pub connector_by_adapter: BTreeMap<String, String>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            mode: DisplayMode::Primary,
            connector_by_adapter: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LengthMode {
    Fit,
    Expand,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Geometry {
    pub edge: Edge,
    pub length_mode: LengthMode,
    pub custom_length_percent: u8,
    pub alignment: Alignment,
    pub icon_size: u8,
}

impl Default for Geometry {
    fn default() -> Self {
        Self {
            edge: Edge::Bottom,
            length_mode: LengthMode::Fit,
            custom_length_percent: 60,
            alignment: Alignment::Center,
            icon_size: 48,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Fixed,
    Autohide,
    Dodge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Behavior {
    pub visibility: Visibility,
    pub reveal_delay_ms: u16,
    pub hide_delay_ms: u16,
    pub shortcut: String,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            visibility: Visibility::Dodge,
            reveal_delay_ms: 160,
            hide_delay_ms: 300,
            shortcut: "<Super><Alt>d".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "color", rename_all = "lowercase")]
pub enum Accent {
    System,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    pub color_scheme: ColorScheme,
    pub accent: Accent,
    pub opacity_percent: u8,
    pub corner_radius: u8,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::System,
            accent: Accent::System,
            opacity_percent: 88,
            corner_radius: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub seeded: bool,
    pub favorites: Vec<String>,
    pub display: DisplayConfig,
    pub geometry: Geometry,
    pub behavior: Behavior,
    pub appearance: Appearance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            seeded: false,
            favorites: Vec::new(),
            display: DisplayConfig::default(),
            geometry: Geometry::default(),
            behavior: Behavior::default(),
            appearance: Appearance::default(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ConfigError::new(
                "schemaVersion",
                format!("unsupported schema version {}", self.schema_version),
            ));
        }
        if !(25..=100).contains(&self.geometry.custom_length_percent) {
            return Err(ConfigError::new(
                "geometry.customLengthPercent",
                "must be between 25 and 100",
            ));
        }
        if !(24..=96).contains(&self.geometry.icon_size) {
            return Err(ConfigError::new(
                "geometry.iconSize",
                "must be between 24 and 96",
            ));
        }
        if self.behavior.reveal_delay_ms > 1000 {
            return Err(ConfigError::new(
                "behavior.revealDelayMs",
                "must be between 0 and 1000",
            ));
        }
        if self.behavior.hide_delay_ms > 1000 {
            return Err(ConfigError::new(
                "behavior.hideDelayMs",
                "must be between 0 and 1000",
            ));
        }
        if !(60..=100).contains(&self.appearance.opacity_percent) {
            return Err(ConfigError::new(
                "appearance.opacityPercent",
                "must be between 60 and 100",
            ));
        }
        if self.appearance.corner_radius > 24 {
            return Err(ConfigError::new(
                "appearance.cornerRadius",
                "must be between 0 and 24",
            ));
        }
        if let Accent::Custom(color) = &self.appearance.accent
            && !is_hex_color(color)
        {
            return Err(ConfigError::new(
                "appearance.accent",
                "custom accent must use #RRGGBB",
            ));
        }
        if self.behavior.shortcut.trim().is_empty() {
            return Err(ConfigError::new("behavior.shortcut", "must not be empty"));
        }
        if self
            .display
            .connector_by_adapter
            .iter()
            .any(|(adapter, connector)| adapter.trim().is_empty() || connector.trim().is_empty())
        {
            return Err(ConfigError::new(
                "display.connectorByAdapter",
                "adapter and connector names must not be empty",
            ));
        }

        let mut unique = HashSet::new();
        if self
            .favorites
            .iter()
            .any(|id| !is_safe_desktop_id(id) || !unique.insert(id))
        {
            return Err(ConfigError::new(
                "favorites",
                "favorites must be unique installed desktop entry identifiers",
            ));
        }
        Ok(())
    }
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_safe_desktop_id(value: &str) -> bool {
    !value.is_empty()
        && value.ends_with(".desktop")
        && !value.contains('/')
        && !value.contains("..")
        && !value.chars().any(char::is_control)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid field {field}: {message}")]
pub struct ConfigError {
    field: String,
    message: String,
}

impl ConfigError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub revision: u64,
    pub config: Config,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    pub quarantined_path: PathBuf,
}

#[derive(Debug)]
struct StoreState {
    snapshot: Snapshot,
    recovery: Option<Recovery>,
}

#[derive(Debug)]
pub struct ConfigStore {
    path: PathBuf,
    state: Mutex<StoreState>,
}

impl ConfigStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let (snapshot, recovery, needs_persist) = if path.exists() {
            match load_snapshot(&path) {
                Ok((snapshot, migrated)) => (snapshot, None, migrated),
                Err(StoreError::Io(error)) => return Err(StoreError::Io(error)),
                Err(StoreError::UnsupportedSchema(version)) => {
                    return Err(StoreError::UnsupportedSchema(version));
                }
                Err(_) => {
                    let quarantined_path = next_quarantine_path(&path);
                    fs::rename(&path, &quarantined_path)?;
                    (
                        Snapshot::default(),
                        Some(Recovery { quarantined_path }),
                        true,
                    )
                }
            }
        } else {
            (Snapshot::default(), None, true)
        };

        if needs_persist {
            persist_snapshot(&path, &snapshot)?;
        }
        Ok(Self {
            path,
            state: Mutex::new(StoreState { snapshot, recovery }),
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        self.state
            .lock()
            .expect("config store poisoned")
            .snapshot
            .clone()
    }

    pub fn recovery(&self) -> Option<Recovery> {
        self.state
            .lock()
            .expect("config store poisoned")
            .recovery
            .clone()
    }

    pub fn replace(&self, expected_revision: u64, config: Config) -> Result<Snapshot, StoreError> {
        config.validate()?;
        let mut state = self.state.lock().expect("config store poisoned");
        verify_revision(expected_revision, state.snapshot.revision)?;
        let snapshot = Snapshot {
            revision: state.snapshot.revision + 1,
            config,
        };
        persist_snapshot(&self.path, &snapshot)?;
        state.snapshot = snapshot.clone();
        Ok(snapshot)
    }

    pub fn pin(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: Option<&str>,
    ) -> Result<Snapshot, StoreError> {
        if !is_safe_desktop_id(desktop_id) {
            return Err(StoreError::InvalidDesktopId(desktop_id.into()));
        }
        let mut config = self.snapshot().config;
        config.favorites.retain(|favorite| favorite != desktop_id);
        insert_before(&mut config.favorites, desktop_id, before_id)?;
        self.replace(expected_revision, config)
    }

    pub fn unpin(&self, expected_revision: u64, desktop_id: &str) -> Result<Snapshot, StoreError> {
        let mut config = self.snapshot().config;
        let old_len = config.favorites.len();
        config.favorites.retain(|favorite| favorite != desktop_id);
        if config.favorites.len() == old_len {
            return Err(StoreError::NotPinned(desktop_id.into()));
        }
        self.replace(expected_revision, config)
    }

    pub fn move_favorite(
        &self,
        expected_revision: u64,
        desktop_id: &str,
        before_id: Option<&str>,
    ) -> Result<Snapshot, StoreError> {
        let mut config = self.snapshot().config;
        let position = config
            .favorites
            .iter()
            .position(|favorite| favorite == desktop_id)
            .ok_or_else(|| StoreError::NotPinned(desktop_id.into()))?;
        config.favorites.remove(position);
        insert_before(&mut config.favorites, desktop_id, before_id)?;
        self.replace(expected_revision, config)
    }

    pub fn export(&self) -> Result<String, StoreError> {
        Ok(serde_json::to_string_pretty(&self.snapshot().config)?)
    }

    pub fn import(&self, expected_revision: u64, json: &str) -> Result<Snapshot, StoreError> {
        let config: Config = serde_json::from_str(json)?;
        config.validate()?;
        let mut state = self.state.lock().expect("config store poisoned");
        verify_revision(expected_revision, state.snapshot.revision)?;

        let backup_path = self.path.with_extension("json.bak");
        persist_bytes(&backup_path, &fs::read(&self.path)?)?;
        let snapshot = Snapshot {
            revision: state.snapshot.revision + 1,
            config,
        };
        persist_snapshot(&self.path, &snapshot)?;
        state.snapshot = snapshot.clone();
        Ok(snapshot)
    }
}

fn insert_before(
    favorites: &mut Vec<String>,
    desktop_id: &str,
    before_id: Option<&str>,
) -> Result<(), StoreError> {
    let index = match before_id {
        Some(anchor) => favorites
            .iter()
            .position(|favorite| favorite == anchor)
            .ok_or_else(|| StoreError::AnchorNotPinned(anchor.into()))?,
        None => favorites.len(),
    };
    favorites.insert(index, desktop_id.into());
    Ok(())
}

fn verify_revision(expected: u64, actual: u64) -> Result<(), StoreError> {
    if expected == actual {
        Ok(())
    } else {
        Err(StoreError::RevisionConflict { expected, actual })
    }
}

fn load_snapshot(path: &Path) -> Result<(Snapshot, bool), StoreError> {
    let bytes = fs::read(path)?;
    let snapshot = match serde_json::from_slice::<Snapshot>(&bytes) {
        Ok(snapshot) => snapshot,
        Err(snapshot_error) => match serde_json::from_slice::<Config>(&bytes) {
            Ok(config) => {
                if config.schema_version != SCHEMA_VERSION {
                    return Err(StoreError::UnsupportedSchema(config.schema_version));
                }
                config.validate()?;
                return Ok((
                    Snapshot {
                        revision: 0,
                        config,
                    },
                    true,
                ));
            }
            Err(_) => return Err(StoreError::Json(snapshot_error)),
        },
    };
    if snapshot.config.schema_version != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema(
            snapshot.config.schema_version,
        ));
    }
    snapshot.config.validate()?;
    Ok((snapshot, false))
}

fn persist_snapshot(path: &Path, snapshot: &Snapshot) -> Result<(), StoreError> {
    let mut bytes = serde_json::to_vec_pretty(snapshot)?;
    bytes.push(b'\n');
    persist_bytes(path, &bytes)
}

fn persist_bytes(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("data")
    ));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

fn next_quarantine_path(path: &Path) -> PathBuf {
    let base = path.with_extension("json.corrupt");
    if !base.exists() {
        return base;
    }
    for suffix in 1_u32.. {
        let candidate = path.with_extension(format!("json.corrupt.{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("unsupported configuration schema version: {0}")]
    UnsupportedSchema(u32),
    #[error("configuration revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("invalid desktop entry identifier: {0}")]
    InvalidDesktopId(String),
    #[error("favorite is not pinned: {0}")]
    NotPinned(String),
    #[error("favorite anchor is not pinned: {0}")]
    AnchorNotPinned(String),
    #[error(transparent)]
    InvalidConfig(#[from] ConfigError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
