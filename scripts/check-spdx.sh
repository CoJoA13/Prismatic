#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
expected_license_hash=8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903
license_hash=$(sha256sum "$project_dir/LICENSE" | cut -d' ' -f1)
if [[ "$license_hash" != "$expected_license_hash" ]]; then
  printf 'LICENSE is not the canonical GNU GPL version 3 text\n' >&2
  exit 1
fi
missing=0
while IFS= read -r -d '' path; do
  if ! grep -q 'SPDX-License-Identifier: GPL-3.0-or-later' "$path"; then
    printf 'Missing GPL-3.0-or-later SPDX identifier: %s\n' "${path#"$project_dir/"}" >&2
    missing=1
  fi
done < <(
  find "$project_dir" \
    -path "$project_dir/.git" -prune -o \
    -path "$project_dir/target" -prune -o \
    -path "$project_dir/node_modules" -prune -o \
    -path "$project_dir/build" -prune -o \
    -path "$project_dir/build-local" -prune -o \
    -type f \( \
      -name '*.rs' -o -name '*.js' -o -name '*.mjs' -o -name '*.qml' -o \
      -name '*.sh' -o -name '*.toml' -o -name '*.xml' -o -name '*.desktop' -o \
      -name '*.service.in' -o -name '*.spec' -o -name 'meson.build' -o \
      -name 'meson_options.txt' -o -name 'Justfile' \
    \) -print0
)
exit "$missing"
