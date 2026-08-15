// SPDX-License-Identifier: GPL-3.0-or-later

use prismatic_settings::{DesktopSession, SessionError};

#[test]
fn detects_supported_wayland_desktops_from_composite_names() {
    assert_eq!(
        DesktopSession::detect("GNOME:GNOME-Classic", "wayland").unwrap(),
        DesktopSession::Gnome
    );
    assert_eq!(
        DesktopSession::detect("KDE", "wayland").unwrap(),
        DesktopSession::Plasma
    );
}

#[test]
fn rejects_x11_even_when_the_desktop_name_is_supported() {
    let error = DesktopSession::detect("GNOME", "x11").unwrap_err();
    assert_eq!(error, SessionError::UnsupportedSessionType("x11".into()));
}

#[test]
fn rejects_desktops_outside_the_version_one_scope() {
    let error = DesktopSession::detect("sway", "wayland").unwrap_err();
    assert_eq!(error, SessionError::UnsupportedDesktop("sway".into()));
}
