#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

build_dir=$(realpath -e "${1:-build}")
stage_dir="$build_dir/install-root"

case "$stage_dir" in
  "$build_dir"/install-root) ;;
  *)
    printf 'Refusing unsafe staged-install directory: %s\n' "$stage_dir" >&2
    exit 2
    ;;
esac

mkdir -p "$stage_dir"
DESTDIR="$stage_dir" meson install -C "$build_dir"
test -x "$stage_dir/usr/libexec/prismatic-service"
if grep -Eq '"name"[[:space:]]*:[[:space:]]*"build_settings"[^}]*"value"[[:space:]]*:[[:space:]]*true' \
  "$build_dir/meson-info/intro-buildoptions.json"; then
  test -x "$stage_dir/usr/bin/prismatic-settings"
fi
test -f "$stage_dir/usr/share/gnome-shell/extensions/prismatic@cojoa13.github.io/extension.js"
test -f "$stage_dir/usr/share/gnome-shell/extensions/prismatic@cojoa13.github.io/configContract.mjs"
test -f "$stage_dir/usr/share/plasma/plasmoids/io.github.CoJoA13.Prismatic/contents/ui/main.qml"

DESTDIR="$stage_dir" ninja -C "$build_dir" uninstall
if [[ -d "$stage_dir" ]] && find "$stage_dir" -type f -print -quit | grep -q .; then
  printf 'Staged uninstall left files behind:\n' >&2
  find "$stage_dir" -type f -print >&2
  exit 1
fi
