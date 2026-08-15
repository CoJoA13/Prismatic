# SPDX-License-Identifier: GPL-3.0-or-later

project(
  'prismatic',
  'rust',
  version: '0.1.0',
  license: 'GPL-3.0-or-later',
  meson_version: '>= 1.3.0',
  default_options: ['warning_level=2'],
)

if get_option('fedora_target') != '44'
  error('Prismatic v1 supports Fedora 44 only')
endif

prefix = get_option('prefix')
bindir = get_option('bindir')
libexecdir = get_option('libexecdir')
datadir = get_option('datadir')

cargo = find_program('cargo')
glib_compile_schemas = find_program('glib-compile-schemas')

rust_outputs = ['prismatic-service']
rust_install_dirs = [libexecdir]
rust_sources = files(
  'Cargo.toml',
  'Cargo.lock',
  'crates/prismatic-core/Cargo.toml',
  'crates/prismatic-core/src/lib.rs',
  'crates/prismatic-service/Cargo.toml',
  'crates/prismatic-service/src/lib.rs',
  'crates/prismatic-service/src/main.rs',
  'apps/prismatic-settings/Cargo.toml',
  'apps/prismatic-settings/src/lib.rs',
  'apps/prismatic-settings/src/main.rs',
)
if get_option('build_settings')
  rust_outputs += ['prismatic-settings']
  rust_install_dirs += [bindir]
endif

custom_target(
  'rust-binaries',
  output: rust_outputs,
  command: [
    files('build-aux/cargo-build.sh'),
    meson.project_source_root(),
    '@OUTDIR@',
    cargo,
    get_option('build_settings') ? 'true' : 'false',
  ],
  build_by_default: true,
  install: true,
  install_dir: rust_install_dirs,
  depend_files: rust_sources,
)

gnome_extension_dir = join_paths(datadir, 'gnome-shell', 'extensions', 'prismatic@cojoa13.github.io')
install_data(
  'adapters/gnome/extension.js',
  'adapters/gnome/dock.js',
  'adapters/gnome/dockContent.js',
  'adapters/gnome/model.js',
  'adapters/gnome/serviceClient.js',
  'adapters/gnome/shellCompat.js',
  'adapters/gnome/metadata.json',
  'adapters/gnome/stylesheet.css',
  install_dir: gnome_extension_dir,
)
install_data(
  'adapters/shared/configContract.mjs',
  rename: 'configContract.mjs',
  install_dir: gnome_extension_dir,
)

custom_target(
  'gnome-extension-schemas',
  input: 'adapters/gnome/schemas/org.gnome.shell.extensions.prismatic.gschema.xml',
  output: 'gschemas.compiled',
  command: [glib_compile_schemas, '--strict', '--targetdir=@OUTDIR@', join_paths(meson.project_source_root(), 'adapters/gnome/schemas')],
  build_by_default: true,
  install: true,
  install_dir: join_paths(gnome_extension_dir, 'schemas'),
)
install_data(
  'adapters/gnome/schemas/org.gnome.shell.extensions.prismatic.gschema.xml',
  install_dir: join_paths(gnome_extension_dir, 'schemas'),
)

plasma_applet_dir = join_paths(datadir, 'plasma', 'plasmoids', 'io.github.CoJoA13.Prismatic')
install_data('adapters/plasma/metadata.json', install_dir: plasma_applet_dir)
install_subdir('adapters/plasma/contents', install_dir: plasma_applet_dir)

plasma_layout_dir = join_paths(datadir, 'plasma', 'layout-templates', 'io.github.CoJoA13.Prismatic.Dock')
install_data('adapters/plasma/layout-template/metadata.json', install_dir: plasma_layout_dir)
install_subdir('adapters/plasma/layout-template/contents', install_dir: plasma_layout_dir)

service_conf = configuration_data()
service_conf.set('libexecdir', join_paths(prefix, libexecdir))
configure_file(
  input: 'data/io.github.CoJoA13.Prismatic.Service.service.in',
  output: 'io.github.CoJoA13.Prismatic.Service.service',
  configuration: service_conf,
  install: true,
  install_dir: join_paths(datadir, 'dbus-1', 'services'),
)
configure_file(
  input: 'systemd/prismatic-service.service.in',
  output: 'prismatic-service.service',
  configuration: service_conf,
  install: true,
  install_dir: join_paths(datadir, 'systemd', 'user'),
)

install_data('data/io.github.CoJoA13.Prismatic.desktop', install_dir: join_paths(datadir, 'applications'))
install_data('data/io.github.CoJoA13.Prismatic.metainfo.xml', install_dir: join_paths(datadir, 'metainfo'))
install_data(
  'data/icons/hicolor/512x512/apps/io.github.CoJoA13.Prismatic.png',
  install_dir: join_paths(datadir, 'icons', 'hicolor', '512x512', 'apps'),
)
install_data('LICENSE', install_dir: join_paths(datadir, 'licenses', 'prismatic'))

summary({
  'Version': meson.project_version(),
  'Fedora target': get_option('fedora_target'),
  'Settings application': get_option('build_settings'),
  'GNOME extension directory': gnome_extension_dir,
  'Plasma applet directory': plasma_applet_dir,
}, section: 'Prismatic')
