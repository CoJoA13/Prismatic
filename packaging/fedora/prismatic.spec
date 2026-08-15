# SPDX-License-Identifier: GPL-3.0-or-later

Name:           prismatic
Version:        0.1.0
Release:        1%{?dist}
Summary:        Native Fedora dock for GNOME and Plasma
License:        GPL-3.0-or-later
URL:            https://github.com/CoJoA13/Prismatic
Source0:        %{url}/releases/download/v%{version}/Prismatic-%{version}-vendor.tar.xz

ExclusiveArch:  x86_64 aarch64
BuildRequires:  cargo
BuildRequires:  desktop-file-utils
BuildRequires:  gcc
BuildRequires:  glib2-devel
BuildRequires:  libadwaita-devel >= 1.9
BuildRequires:  meson >= 1.3
BuildRequires:  ninja-build
BuildRequires:  pkgconfig(gtk4) >= 4.22
BuildRequires:  rust >= 1.92
BuildRequires:  systemd-rpm-macros

Requires:       prismatic-core%{?_isa} = %{version}-%{release}
Requires:       gnome-shell-extension-prismatic = %{version}-%{release}
Requires:       plasma6-applet-prismatic = %{version}-%{release}

%description
Prismatic is a Fedora 44 Wayland dock with native GNOME Shell 50 and Plasma
6.6+ adapters driven by a shared configuration service.

%package -n prismatic-core
Summary:        Prismatic configuration service and settings application
Requires:       dbus
Requires:       hicolor-icon-theme
Requires:       qt6-qttools
Requires:       systemd

%description -n prismatic-core
The revisioned D-Bus configuration broker and GTK 4/libadwaita settings app
shared by Prismatic desktop adapters.

%package -n gnome-shell-extension-prismatic
Summary:        Prismatic dock adapter for GNOME Shell 50
BuildArch:      noarch
Requires:       gnome-shell >= 50
Requires:       gnome-shell < 51
Requires:       prismatic-core%{?_isa} = %{version}-%{release}

%description -n gnome-shell-extension-prismatic
Independent GNOME Shell dock actor for Prismatic on Fedora 44 Wayland.

%package -n plasma6-applet-prismatic
Summary:        Prismatic dock adapter for Plasma 6.6+
BuildArch:      noarch
Requires:       plasma-workspace >= 6.6
Requires:       prismatic-core%{?_isa} = %{version}-%{release}

%description -n plasma6-applet-prismatic
Native Plasma task applet and dedicated panel layout template for Prismatic.

%prep
%autosetup -n Prismatic-%{version} -p1
test -d vendor

%build
export CARGO_HOME=%{_builddir}/Prismatic-%{version}/.cargo-home
%meson -Dfedora_target=44 -Dbuild_settings=true
%meson_build

%install
%meson_install
desktop-file-validate %{buildroot}%{_datadir}/applications/io.github.CoJoA13.Prismatic.desktop

%check
export CARGO_HOME=%{_builddir}/Prismatic-%{version}/.cargo-home
cargo test --workspace --all-targets --offline

%post -n prismatic-core
%systemd_user_post prismatic-service.service

%preun -n prismatic-core
%systemd_user_preun prismatic-service.service

%postun -n prismatic-core
%systemd_user_postun_with_restart prismatic-service.service
update-desktop-database -q %{_datadir}/applications &>/dev/null || :

%posttrans -n prismatic-core
update-desktop-database -q %{_datadir}/applications &>/dev/null || :

%files
%license LICENSE
%doc README.md

%files -n prismatic-core
%license LICENSE
%doc README.md CHANGELOG.md docs/user-guide.md
%{_bindir}/prismatic-settings
%{_libexecdir}/prismatic-service
%{_datadir}/applications/io.github.CoJoA13.Prismatic.desktop
%{_datadir}/dbus-1/services/io.github.CoJoA13.Prismatic.Service.service
%{_datadir}/systemd/user/prismatic-service.service
%{_datadir}/metainfo/io.github.CoJoA13.Prismatic.metainfo.xml
%{_datadir}/icons/hicolor/512x512/apps/io.github.CoJoA13.Prismatic.png
%{_datadir}/licenses/prismatic/LICENSE

%files -n gnome-shell-extension-prismatic
%license LICENSE
%{_datadir}/gnome-shell/extensions/prismatic@cojoa13.github.io/

%files -n plasma6-applet-prismatic
%license LICENSE
%{_datadir}/plasma/plasmoids/io.github.CoJoA13.Prismatic/
%{_datadir}/plasma/layout-templates/io.github.CoJoA13.Prismatic.Dock/

%changelog
* Fri Aug 14 2026 Prismatic contributors <noreply@github.com> - 0.1.0-1
- Initial Fedora 44 foundation
