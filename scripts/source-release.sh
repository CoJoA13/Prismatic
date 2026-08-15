#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

version=${1:?usage: source-release.sh VERSION}
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
reference="v$version"
prefix="Prismatic-$version"
temporary_dir=$(mktemp -d -t prismatic-release.XXXXXX)
cleanup() {
  [[ -n "$temporary_dir" && "$temporary_dir" == /tmp/prismatic-release.* ]] || return 1
  rm -rf -- "$temporary_dir"
}
trap cleanup EXIT

git -C "$project_dir" rev-parse --verify "$reference^{commit}" >/dev/null
mkdir -p "$project_dir/dist" "$temporary_dir/$prefix"
git -C "$project_dir" archive "$reference" | tar -x -C "$temporary_dir/$prefix"
cargo vendor --locked --manifest-path "$temporary_dir/$prefix/Cargo.toml" "$temporary_dir/$prefix/vendor" >/dev/null
mkdir -p "$temporary_dir/$prefix/.cargo"
install -m 0644 "$project_dir/packaging/fedora/vendor-config.toml" "$temporary_dir/$prefix/.cargo/config.toml"

source_date_epoch=$(git -C "$project_dir" log -1 --format=%ct "$reference")
archive="$project_dir/dist/$prefix-vendor.tar.xz"
tar --sort=name --owner=0 --group=0 --numeric-owner \
  --mtime="@$source_date_epoch" -C "$temporary_dir" -cJf "$archive" "$prefix"
(cd "$project_dir/dist" && sha256sum "$(basename -- "$archive")" >"$(basename -- "$archive").sha256")
(cd "$project_dir/dist" && sha256sum -c "$(basename -- "$archive").sha256")
printf '%s\n%s\n' "$archive" "$archive.sha256"
