#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

output=${1:-dist/prismatic@cojoa13.github.io.shell-extension.zip}
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ "$output" = /* ]]; then
  output_path=$output
else
  output_path=$project_dir/$output
fi
temporary_dir=$(mktemp -d -t prismatic-gnome.XXXXXX)
cleanup() {
  [[ -n "$temporary_dir" && "$temporary_dir" == /tmp/prismatic-gnome.* ]] || return 1
  rm -rf -- "$temporary_dir"
}
trap cleanup EXIT

mkdir -p "$(dirname -- "$output_path")" "$temporary_dir/schemas"
install -m 0644 \
  "$project_dir/adapters/gnome/extension.js" \
  "$project_dir/adapters/gnome/dockContent.js" \
  "$project_dir/adapters/gnome/dock.js" \
  "$project_dir/adapters/gnome/model.js" \
  "$project_dir/adapters/gnome/serviceClient.js" \
  "$project_dir/adapters/gnome/shellCompat.js" \
  "$project_dir/adapters/gnome/metadata.json" \
  "$project_dir/adapters/gnome/stylesheet.css" \
  "$temporary_dir/"
install -m 0644 "$project_dir/adapters/shared/configContract.mjs" "$temporary_dir/configContract.mjs"
install -m 0644 "$project_dir/adapters/gnome/schemas/"*.xml "$temporary_dir/schemas/"
glib-compile-schemas --strict "$temporary_dir/schemas"
find "$temporary_dir" -exec touch --date="@${SOURCE_DATE_EPOCH:-0}" -- {} +

(cd "$temporary_dir" && zip -q -X -FS -r "$output_path" .)
printf '%s\n' "$output_path"
