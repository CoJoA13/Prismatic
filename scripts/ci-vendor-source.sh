#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

version=${1:-0.1.0}
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
prefix="Prismatic-$version"
temporary_dir=$(mktemp -d -t prismatic-ci-source.XXXXXX)
cleanup() {
  [[ -n "$temporary_dir" && "$temporary_dir" == /tmp/prismatic-ci-source.* ]] || return 1
  rm -rf -- "$temporary_dir"
}
trap cleanup EXIT

mkdir -p "$temporary_dir/$prefix" "$project_dir/dist"
rsync -a \
  --exclude=.git --exclude=target --exclude=node_modules --exclude=build --exclude=build-local --exclude=dist \
  "$project_dir/" "$temporary_dir/$prefix/"
cargo vendor --locked --manifest-path "$temporary_dir/$prefix/Cargo.toml" "$temporary_dir/$prefix/vendor" >/dev/null
mkdir -p "$temporary_dir/$prefix/.cargo"
install -m 0644 "$project_dir/packaging/fedora/vendor-config.toml" "$temporary_dir/$prefix/.cargo/config.toml"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime='@0' \
  -C "$temporary_dir" -cJf "$project_dir/dist/$prefix-vendor.tar.xz" "$prefix"
