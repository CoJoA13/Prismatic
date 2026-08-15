// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::env;
use std::process::Command;
use std::rc::Rc;

use adw::gtk;
use adw::prelude::*;
use gtk::gio;
use libadwaita as adw;
use prismatic_core::{
    Accent, Alignment, ColorScheme, Config, DisplayMode, Edge, LengthMode, Snapshot, Visibility,
};
use prismatic_settings::{DesktopSession, ServiceClient};

const APP_ID: &str = "io.github.CoJoA13.Prismatic";
const SERVICE_NAME: &str = "io.github.CoJoA13.Prismatic.Service";
const SERVICE_PATH: &str = "/io/github/CoJoA13/Prismatic";
const SERVICE_INTERFACE: &str = "io.github.CoJoA13.Prismatic.Service1";
const GNOME_UUID: &str = "prismatic@cojoa13.github.io";

const PLASMA_REMOVAL_SCRIPT: &str = r#"
let removed = false;
for (const panel of panels()) {
    for (const widget of panel.widgets('io.github.CoJoA13.Prismatic')) {
        widget.currentConfigGroup = ['General'];
        const ownedPanelId = String(widget.readConfig('ownedPanelId', ''));
        if (ownedPanelId === String(panel.id)) {
            panel.remove();
            removed = true;
            break;
        }
    }
    if (removed) {
        break;
    }
}
print(removed ? 'removed=true' : 'removed=false');
"#;

#[derive(Clone)]
struct Controls {
    display_mode: adw::ComboRow,
    gnome_connector: adw::EntryRow,
    plasma_connector: adw::EntryRow,
    edge: adw::ComboRow,
    length_mode: adw::ComboRow,
    custom_length: adw::SpinRow,
    alignment: adw::ComboRow,
    icon_size: adw::SpinRow,
    visibility: adw::ComboRow,
    reveal_delay: adw::SpinRow,
    hide_delay: adw::SpinRow,
    shortcut: adw::EntryRow,
    color_scheme: adw::ComboRow,
    accent: adw::EntryRow,
    opacity: adw::SpinRow,
    corner_radius: adw::SpinRow,
}

fn main() -> adw::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let host = adw::ApplicationWindow::builder()
        .application(app)
        .title("Prismatic")
        .default_width(760)
        .default_height(720)
        .build();
    let dialog = adw::PreferencesDialog::builder()
        .title("Prismatic")
        .search_enabled(true)
        .build();

    let client = match ServiceClient::connect_to(SERVICE_NAME, SERVICE_PATH, SERVICE_INTERFACE) {
        Ok(client) => Rc::new(client),
        Err(error) => {
            let page = adw::PreferencesPage::builder()
                .title("Diagnostics")
                .icon_name("dialog-error-symbolic")
                .build();
            let group = adw::PreferencesGroup::builder()
                .title("Prismatic service unavailable")
                .description(error.to_string())
                .build();
            page.add(&group);
            dialog.add(&page);
            host.present();
            dialog.present(Some(&host));
            return;
        }
    };
    let snapshot = match client.get_snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            dialog.add_toast(adw::Toast::new(&format!(
                "Could not load settings: {error}"
            )));
            Snapshot::default()
        }
    };
    let state = Rc::new(RefCell::new(snapshot.clone()));
    let controls = Controls::new(&snapshot);

    dialog.add(&general_page(&dialog, &host, &client, &state, &controls));
    dialog.add(&position_page(&controls));
    dialog.add(&behavior_page(&controls));
    dialog.add(&appearance_page(&controls));
    dialog.add(&applications_page(&snapshot));
    dialog.add(&transfer_page(&dialog, &host, &client, &state));
    dialog.add(&diagnostics_page(&client));
    let host_for_close = host.clone();
    dialog.connect_closed(move |_| host_for_close.close());
    host.present();
    dialog.present(Some(&host));
}

impl Controls {
    fn new(snapshot: &Snapshot) -> Self {
        let config = &snapshot.config;
        Self {
            display_mode: combo_row(
                "Display",
                "Choose the primary display or an adapter-specific connector",
                &["Primary", "Selected connector"],
                match config.display.mode {
                    DisplayMode::Primary => 0,
                    DisplayMode::Selected => 1,
                },
            ),
            gnome_connector: entry_row(
                "GNOME connector",
                config
                    .display
                    .connector_by_adapter
                    .get("gnome")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            plasma_connector: entry_row(
                "Plasma connector",
                config
                    .display
                    .connector_by_adapter
                    .get("plasma")
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            edge: combo_row(
                "Screen edge",
                "Where the dock is anchored",
                &["Top", "Bottom", "Left", "Right"],
                match config.geometry.edge {
                    Edge::Top => 0,
                    Edge::Bottom => 1,
                    Edge::Left => 2,
                    Edge::Right => 3,
                },
            ),
            length_mode: combo_row(
                "Length mode",
                "Fit icons, fill the edge, or use a percentage",
                &["Fit Content", "Expand to Edge", "Custom"],
                match config.geometry.length_mode {
                    LengthMode::Fit => 0,
                    LengthMode::Expand => 1,
                    LengthMode::Custom => 2,
                },
            ),
            custom_length: spin_row(
                "Custom length",
                "Percentage of available edge length",
                25.0,
                100.0,
                config.geometry.custom_length_percent as f64,
            ),
            alignment: combo_row(
                "Alignment",
                "Anchor within the selected edge",
                &["Start", "Center", "End"],
                match config.geometry.alignment {
                    Alignment::Start => 0,
                    Alignment::Center => 1,
                    Alignment::End => 2,
                },
            ),
            icon_size: spin_row(
                "Icon size",
                "Logical pixels",
                24.0,
                96.0,
                config.geometry.icon_size as f64,
            ),
            visibility: combo_row(
                "Visibility",
                "Reserve space, always hide, or dodge windows",
                &["Fixed", "Auto-hide", "Dodge Windows"],
                match config.behavior.visibility {
                    Visibility::Fixed => 0,
                    Visibility::Autohide => 1,
                    Visibility::Dodge => 2,
                },
            ),
            reveal_delay: spin_row(
                "GNOME reveal delay",
                "Milliseconds at the pressure barrier",
                0.0,
                1000.0,
                config.behavior.reveal_delay_ms as f64,
            ),
            hide_delay: spin_row(
                "GNOME hide delay",
                "Milliseconds after the pointer leaves the GNOME dock",
                0.0,
                1000.0,
                config.behavior.hide_delay_ms as f64,
            ),
            shortcut: entry_row("Reveal shortcut", &config.behavior.shortcut),
            color_scheme: combo_row(
                "Color scheme",
                "Follow the desktop or force light/dark",
                &["System", "Light", "Dark"],
                match config.appearance.color_scheme {
                    ColorScheme::System => 0,
                    ColorScheme::Light => 1,
                    ColorScheme::Dark => 2,
                },
            ),
            accent: entry_row(
                "Accent",
                match &config.appearance.accent {
                    Accent::System => "system",
                    Accent::Custom(color) => color,
                },
            ),
            opacity: spin_row(
                "Surface opacity",
                "Percent",
                60.0,
                100.0,
                config.appearance.opacity_percent as f64,
            ),
            corner_radius: spin_row(
                "Corner radius",
                "Logical pixels",
                0.0,
                24.0,
                config.appearance.corner_radius as f64,
            ),
        }
    }

    fn apply_to(&self, snapshot: &Snapshot) -> Result<prismatic_core::Config, String> {
        let mut config = snapshot.config.clone();
        config.display.mode = match self.display_mode.selected() {
            0 => DisplayMode::Primary,
            _ => DisplayMode::Selected,
        };
        set_connector(&mut config, "gnome", self.gnome_connector.text().as_str());
        set_connector(&mut config, "plasma", self.plasma_connector.text().as_str());
        config.geometry.edge = match self.edge.selected() {
            0 => Edge::Top,
            1 => Edge::Bottom,
            2 => Edge::Left,
            _ => Edge::Right,
        };
        config.geometry.length_mode = match self.length_mode.selected() {
            0 => LengthMode::Fit,
            1 => LengthMode::Expand,
            _ => LengthMode::Custom,
        };
        config.geometry.custom_length_percent = self.custom_length.value() as u8;
        config.geometry.alignment = match self.alignment.selected() {
            0 => Alignment::Start,
            1 => Alignment::Center,
            _ => Alignment::End,
        };
        config.geometry.icon_size = self.icon_size.value() as u8;
        config.behavior.visibility = match self.visibility.selected() {
            0 => Visibility::Fixed,
            1 => Visibility::Autohide,
            _ => Visibility::Dodge,
        };
        config.behavior.reveal_delay_ms = self.reveal_delay.value() as u16;
        config.behavior.hide_delay_ms = self.hide_delay.value() as u16;
        config.behavior.shortcut = self.shortcut.text().to_string();
        config.appearance.color_scheme = match self.color_scheme.selected() {
            0 => ColorScheme::System,
            1 => ColorScheme::Light,
            _ => ColorScheme::Dark,
        };
        let accent = self.accent.text();
        config.appearance.accent = if accent.eq_ignore_ascii_case("system") {
            Accent::System
        } else {
            Accent::Custom(accent.to_string())
        };
        config.appearance.opacity_percent = self.opacity.value() as u8;
        config.appearance.corner_radius = self.corner_radius.value() as u8;
        config.validate().map_err(|error| error.to_string())?;
        Ok(config)
    }
}

fn general_page(
    window: &adw::PreferencesDialog,
    host: &adw::ApplicationWindow,
    client: &Rc<ServiceClient>,
    state: &Rc<RefCell<Snapshot>>,
    controls: &Controls,
) -> adw::PreferencesPage {
    let page = page("General", "preferences-system-symbolic");
    let setup = adw::PreferencesGroup::builder()
        .title("Desktop integration")
        .description("Activation is explicit and never edits existing panels or launchers")
        .build();
    let detected = detect_session();
    let row = adw::ActionRow::builder()
        .title("Current session")
        .subtitle(detected.as_deref().unwrap_or_else(|error| error))
        .build();
    let activate = adw::ButtonRow::builder()
        .title("Activate Prismatic for this desktop")
        .start_icon_name("emblem-system-symbolic")
        .build();
    let window_for_activate = window.clone();
    let host_for_activate = host.clone();
    let state_for_activate = Rc::clone(state);
    activate.connect_activated(move |_| {
        if current_session() == Ok(DesktopSession::Plasma) {
            show_plasma_confirmation(
                &window_for_activate,
                &host_for_activate,
                false,
                state_for_activate.borrow().config.clone(),
            );
            return;
        }
        let config = state_for_activate.borrow().config.clone();
        let result = activate_current_session(&config);
        let message = match result {
            Ok(message) => message,
            Err(error) => format!("Activation failed: {error}"),
        };
        window_for_activate.add_toast(adw::Toast::new(&message));
    });
    let remove = adw::ButtonRow::builder()
        .title("Remove Prismatic Plasma Dock")
        .start_icon_name("user-trash-symbolic")
        .visible(current_session() == Ok(DesktopSession::Plasma))
        .build();
    let window_for_remove = window.clone();
    let host_for_remove = host.clone();
    let state_for_remove = Rc::clone(state);
    remove.connect_activated(move |_| {
        show_plasma_confirmation(
            &window_for_remove,
            &host_for_remove,
            true,
            state_for_remove.borrow().config.clone(),
        );
    });
    setup.add(&row);
    setup.add(&activate);
    setup.add(&remove);

    let save = adw::PreferencesGroup::builder().title("Save").build();
    let apply = adw::ButtonRow::builder()
        .title("Apply changes")
        .start_icon_name("document-save-symbolic")
        .build();
    let window_for_apply = window.clone();
    let client = Rc::clone(client);
    let state = Rc::clone(state);
    let controls = controls.clone();
    apply.connect_activated(move |_| {
        let current = state.borrow().clone();
        let result = controls.apply_to(&current).and_then(|config| {
            client
                .replace_snapshot(current.revision, &config)
                .map_err(|error| error.to_string())
        });
        match result {
            Ok(snapshot) => {
                *state.borrow_mut() = snapshot;
                window_for_apply.add_toast(adw::Toast::new("Settings saved"));
            }
            Err(error) => window_for_apply.add_toast(adw::Toast::new(&error)),
        }
    });
    save.add(&apply);
    page.add(&setup);
    page.add(&save);
    page
}

fn position_page(controls: &Controls) -> adw::PreferencesPage {
    let page = page("Position", "view-grid-symbolic");
    let display = adw::PreferencesGroup::builder().title("Display").build();
    display.add(&controls.display_mode);
    display.add(&controls.gnome_connector);
    display.add(&controls.plasma_connector);
    let geometry = adw::PreferencesGroup::builder().title("Geometry").build();
    geometry.add(&controls.edge);
    geometry.add(&controls.length_mode);
    geometry.add(&controls.custom_length);
    geometry.add(&controls.alignment);
    geometry.add(&controls.icon_size);
    page.add(&display);
    page.add(&geometry);
    page
}

fn behavior_page(controls: &Controls) -> adw::PreferencesPage {
    let page = page("Behavior", "preferences-desktop-symbolic");
    let group = adw::PreferencesGroup::builder()
        .title("Visibility")
        .description("Plasma uses its compositor-native panel timing; Plasma's public panel API does not expose reveal or hide delays")
        .build();
    group.add(&controls.visibility);
    group.add(&controls.reveal_delay);
    group.add(&controls.hide_delay);
    group.add(&controls.shortcut);
    page.add(&group);
    page
}

fn appearance_page(controls: &Controls) -> adw::PreferencesPage {
    let page = page("Appearance", "applications-graphics-symbolic");
    let group = adw::PreferencesGroup::builder()
        .title("Native adaptive surface")
        .description("High contrast and reduced motion continue to follow the desktop")
        .build();
    group.add(&controls.color_scheme);
    group.add(&controls.accent);
    group.add(&controls.opacity);
    group.add(&controls.corner_radius);
    page.add(&group);
    page
}

fn applications_page(snapshot: &Snapshot) -> adw::PreferencesPage {
    let page = page("Applications", "application-x-executable-symbolic");
    let group = adw::PreferencesGroup::builder()
        .title("Pinned applications")
        .description("Reorder or unpin directly from the dock")
        .build();
    if snapshot.config.favorites.is_empty() {
        group.add(
            &adw::ActionRow::builder()
                .title("No pinned applications")
                .build(),
        );
    } else {
        for (index, desktop_id) in snapshot.config.favorites.iter().enumerate() {
            group.add(
                &adw::ActionRow::builder()
                    .title(desktop_id)
                    .subtitle(format!("Position {}", index + 1))
                    .build(),
            );
        }
    }
    page.add(&group);
    page
}

fn transfer_page(
    window: &adw::PreferencesDialog,
    host: &adw::ApplicationWindow,
    client: &Rc<ServiceClient>,
    state: &Rc<RefCell<Snapshot>>,
) -> adw::PreferencesPage {
    let page = page("Import / Export", "document-save-symbolic");
    let group = adw::PreferencesGroup::builder()
        .title("Portable schema-v1 configuration")
        .description("Imports are validated and keep a rollback copy")
        .build();
    let import = adw::ButtonRow::builder()
        .title("Import configuration…")
        .start_icon_name("document-open-symbolic")
        .build();
    let export = adw::ButtonRow::builder()
        .title("Export configuration…")
        .start_icon_name("document-save-symbolic")
        .build();

    let parent = window.clone();
    let host_for_import = host.clone();
    let client_for_import = Rc::clone(client);
    let state_for_import = Rc::clone(state);
    import.connect_activated(move |_| {
        let chooser = gtk::FileDialog::builder()
            .title("Import Prismatic Configuration")
            .build();
        let parent = parent.clone();
        let client = Rc::clone(&client_for_import);
        let state = Rc::clone(&state_for_import);
        chooser.open(
            Some(&host_for_import),
            None::<&gio::Cancellable>,
            move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    let result = std::fs::read_to_string(path)
                        .map_err(|error| error.to_string())
                        .and_then(|json| {
                            let revision = state.borrow().revision;
                            client
                                .import(revision, &json)
                                .map_err(|error| error.to_string())
                        });
                    match result {
                        Ok(snapshot) => {
                            *state.borrow_mut() = snapshot;
                            parent.add_toast(adw::Toast::new(
                                "Configuration imported; reopen Settings to refresh controls",
                            ));
                        }
                        Err(error) => parent.add_toast(adw::Toast::new(&error)),
                    }
                }
            },
        );
    });

    let parent = window.clone();
    let host_for_export = host.clone();
    let client_for_export = Rc::clone(client);
    export.connect_activated(move |_| {
        let chooser = gtk::FileDialog::builder()
            .title("Export Prismatic Configuration")
            .initial_name("prismatic-config.json")
            .build();
        let parent = parent.clone();
        let client = Rc::clone(&client_for_export);
        chooser.save(
            Some(&host_for_export),
            None::<&gio::Cancellable>,
            move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    let result =
                        client
                            .export()
                            .map_err(|error| error.to_string())
                            .and_then(|json| {
                                std::fs::write(path, json).map_err(|error| error.to_string())
                            });
                    match result {
                        Ok(()) => parent.add_toast(adw::Toast::new("Configuration exported")),
                        Err(error) => parent.add_toast(adw::Toast::new(&error)),
                    }
                }
            },
        );
    });

    group.add(&import);
    group.add(&export);
    page.add(&group);
    page
}

fn diagnostics_page(client: &ServiceClient) -> adw::PreferencesPage {
    let page = page("Diagnostics", "dialog-information-symbolic");
    let status = client
        .adapter_statuses()
        .and_then(|value| serde_json::to_string_pretty(&value).map_err(Into::into))
        .unwrap_or_else(|error| error.to_string());
    let group = adw::PreferencesGroup::builder()
        .title("Adapter status")
        .description("No telemetry or application launch history is collected")
        .build();
    group.add(
        &adw::ActionRow::builder()
            .title("Runtime state")
            .subtitle(status)
            .subtitle_selectable(true)
            .build(),
    );
    page.add(&group);
    page
}

fn page(title: &str, icon: &str) -> adw::PreferencesPage {
    adw::PreferencesPage::builder()
        .title(title)
        .icon_name(icon)
        .build()
}

fn combo_row(title: &str, subtitle: &str, values: &[&str], selected: u32) -> adw::ComboRow {
    let model = gtk::StringList::new(values);
    adw::ComboRow::builder()
        .title(title)
        .subtitle(subtitle)
        .model(&model)
        .selected(selected)
        .build()
}

fn spin_row(title: &str, subtitle: &str, minimum: f64, maximum: f64, value: f64) -> adw::SpinRow {
    let adjustment = gtk::Adjustment::new(value, minimum, maximum, 1.0, 5.0, 0.0);
    adw::SpinRow::builder()
        .title(title)
        .subtitle(subtitle)
        .adjustment(&adjustment)
        .digits(0)
        .build()
}

fn entry_row(title: &str, text: &str) -> adw::EntryRow {
    adw::EntryRow::builder().title(title).text(text).build()
}

fn set_connector(config: &mut prismatic_core::Config, adapter: &str, connector: &str) {
    let connector = connector.trim();
    if connector.is_empty() {
        config.display.connector_by_adapter.remove(adapter);
    } else {
        config
            .display
            .connector_by_adapter
            .insert(adapter.into(), connector.into());
    }
}

fn detect_session() -> Result<String, String> {
    current_session()
        .map(|session| format!("{session:?} on Wayland"))
        .map_err(|error| error.to_string())
}

fn current_session() -> Result<DesktopSession, prismatic_settings::SessionError> {
    let desktop = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_default();
    DesktopSession::detect(&desktop, &session_type)
}

fn plasma_activation_script(config: &Config) -> String {
    let edge = match config.geometry.edge {
        Edge::Top => "top",
        Edge::Bottom => "bottom",
        Edge::Left => "left",
        Edge::Right => "right",
    };
    let vertical = matches!(config.geometry.edge, Edge::Left | Edge::Right);
    let alignment = match (config.geometry.alignment, vertical) {
        (Alignment::Center, _) => "center",
        (Alignment::Start, false) => "left",
        (Alignment::End, false) => "right",
        (Alignment::Start, true) => "right",
        (Alignment::End, true) => "left",
    };
    let length_mode = match config.geometry.length_mode {
        LengthMode::Fit => "fit",
        LengthMode::Expand => "fill",
        LengthMode::Custom => "custom",
    };
    let hiding = match config.behavior.visibility {
        Visibility::Fixed => "none",
        Visibility::Autohide => "autohide",
        Visibility::Dodge => "dodgewindows",
    };
    format!(
        r#"
let found = false;
let occupied = false;
for (const existingPanel of panels()) {{
    if (existingPanel.location === {edge:?}) {{
        occupied = true;
    }}
    if (existingPanel.widgets('io.github.CoJoA13.Prismatic').length > 0) {{
        found = true;
    }}
}}
if (!found) {{
    const panel = new Panel;
    panel.location = {edge:?};
    panel.lengthMode = {length_mode:?};
    panel.alignment = {alignment:?};
    panel.height = {height};
    panel.hiding = {hiding:?};
    const widget = panel.addWidget('io.github.CoJoA13.Prismatic');
    widget.currentConfigGroup = ['General'];
    widget.writeConfig('ownedPanelId', String(panel.id));
    widget.reloadConfig();
    print('created=' + panel.id + ';occupied=' + occupied);
}} else {{
    print('existing=true');
}}
"#,
        height = u16::from(config.geometry.icon_size) + 16,
    )
}

fn plasma_edge_occupied(edge: Edge) -> Result<bool, String> {
    let edge = match edge {
        Edge::Top => "top",
        Edge::Bottom => "bottom",
        Edge::Left => "left",
        Edge::Right => "right",
    };
    let script = format!(
        "let occupied = false; for (const panel of panels()) {{ if (panel.location === {edge:?} && panel.widgets('io.github.CoJoA13.Prismatic').length === 0) occupied = true; }} print('occupied=' + occupied);"
    );
    run_plasma_script(&script, "Plasma edge probe").map(|result| result.contains("occupied=true"))
}

fn activate_current_session(config: &Config) -> Result<String, String> {
    match current_session().map_err(|error| error.to_string())? {
        DesktopSession::Gnome => run_command(
            "gnome-extensions",
            &["enable", GNOME_UUID],
            "GNOME extension enabled",
        ),
        DesktopSession::Plasma => {
            let script = plasma_activation_script(config);
            run_plasma_script(&script, "Prismatic Plasma panel created or already present")
        }
    }
}

fn show_plasma_confirmation(
    window: &adw::PreferencesDialog,
    host: &adw::ApplicationWindow,
    removing: bool,
    config: Config,
) {
    let occupied = (!removing)
        .then(|| plasma_edge_occupied(config.geometry.edge))
        .transpose()
        .ok()
        .flatten();
    let (heading, body, action, appearance) = if removing {
        (
            "Remove the Prismatic dock?",
            "Only a panel containing Prismatic whose recorded ownership ID matches the panel will be removed. Existing panels are never touched.",
            "Remove Dock",
            adw::ResponseAppearance::Destructive,
        )
    } else {
        (
            "Create a separate Plasma dock?",
            if occupied == Some(true) {
                "The selected edge already contains another panel. Prismatic will create a separate owned panel and will not modify the existing panel."
            } else if occupied == Some(false) {
                "Prismatic will create a separate owned panel and will not modify any existing desktop UI."
            } else {
                "Prismatic could not inspect the selected edge. It may already contain a panel; Prismatic will not modify existing desktop UI."
            },
            "Create Dock",
            adw::ResponseAppearance::Suggested,
        )
    };
    let dialog = adw::AlertDialog::new(Some(heading), Some(body));
    dialog.add_responses(&[("cancel", "Cancel"), ("confirm", action)]);
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("cancel"));
    dialog.set_response_appearance("confirm", appearance);
    let window = window.clone();
    dialog.connect_response(Some("confirm"), move |_, _| {
        let result = if removing {
            run_plasma_script(
                PLASMA_REMOVAL_SCRIPT,
                "Prismatic Plasma panel removal completed",
            )
        } else {
            activate_current_session(&config)
        };
        let message = result.unwrap_or_else(|error| format!("Plasma action failed: {error}"));
        window.add_toast(adw::Toast::new(&message));
    });
    dialog.present(Some(host));
}

fn run_plasma_script(script: &str, success: &str) -> Result<String, String> {
    let output = run_command(
        "qdbus-qt6",
        &[
            "org.kde.plasmashell",
            "/PlasmaShell",
            "org.kde.PlasmaShell.evaluateScript",
            script,
        ],
        success,
    )?;
    if output.contains("Error:") {
        Err(output)
    } else {
        Ok(output)
    }
}

fn run_command(program: &str, arguments: &[&str], success: &str) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run {program}: {error}"))?;
    if output.status.success() {
        let details = String::from_utf8_lossy(&output.stdout);
        let details = details.trim();
        Ok(if details.is_empty() {
            success.into()
        } else {
            format!("{success}: {details}")
        })
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
