#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

output=${1:-dist/io.github.CoJoA13.Prismatic.plasmoid}
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ "$output" = /* ]]; then
  output_path=$output
else
  output_path=$project_dir/$output
fi
temporary_dir=$(mktemp -d -t prismatic-plasma.XXXXXX)
cleanup() {
  [[ -n "$temporary_dir" && "$temporary_dir" == /tmp/prismatic-plasma.* ]] || return 1
  rm -rf -- "$temporary_dir"
}
trap cleanup EXIT

mkdir -p "$(dirname -- "$output_path")"
install -m 0644 "$project_dir/adapters/plasma/metadata.json" "$temporary_dir/metadata.json"
cp -a "$project_dir/adapters/plasma/contents" "$temporary_dir/contents"
find "$temporary_dir" -exec touch --date="@${SOURCE_DATE_EPOCH:-0}" -- {} +
(cd "$temporary_dir" && zip -q -X -FS -r "$output_path" .)
printf '%s\n' "$output_path"
