# SPDX-License-Identifier: GPL-3.0-or-later

set shell := ["bash", "-euo", "pipefail", "-c"]

default: test

bootstrap:
    npm ci

fmt:
    cargo fmt --all
    npm run lint -- --fix

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    npm run lint

test:
    cargo test --workspace --all-targets
    npm test

test-dbus:
    dbus-run-session -- cargo test -p prismatic-service --test dbus_contract
    dbus-run-session -- cargo test -p prismatic-settings --test client_contract

configure build_dir="build":
    meson setup --wipe {{build_dir}} --prefix=/usr -Dfedora_target=44

build build_dir="build":
    meson compile -C {{build_dir}}

staged-install build_dir="build" stage_dir="build/stage":
    DESTDIR="$PWD/{{stage_dir}}" meson install -C {{build_dir}}

verify-install build_dir="build":
    build-aux/verify-install.sh {{build_dir}}

package-gnome output="dist/prismatic@cojoa13.github.io.shell-extension.zip":
    scripts/package-gnome.sh {{output}}

package-plasma output="dist/io.github.CoJoA13.Prismatic.plasmoid":
    scripts/package-plasma.sh {{output}}

source-tarball version="0.1.0":
    scripts/source-release.sh {{version}}

rpm spec="packaging/fedora/prismatic.spec":
    rpmbuild -ba {{spec}}

mock result_dir="result":
    mock -r fedora-44-x86_64 --resultdir {{result_dir}} packaging/fedora/prismatic.spec
