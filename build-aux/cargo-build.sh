#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

source_dir=${1:?source directory is required}
output_dir=${2:?output directory is required}
cargo=${3:?cargo executable is required}
build_settings=${4:?settings build toggle is required}

export CARGO_TARGET_DIR="$output_dir/cargo-target"
"$cargo" build --manifest-path "$source_dir/Cargo.toml" --locked --release --package prismatic-service
install -m 0755 "$CARGO_TARGET_DIR/release/prismatic-service" "$output_dir/prismatic-service"

if [[ "$build_settings" == true ]]; then
  "$cargo" build --manifest-path "$source_dir/Cargo.toml" --locked --release --package prismatic-settings --features ui --bin prismatic-settings
  install -m 0755 "$CARGO_TARGET_DIR/release/prismatic-settings" "$output_dir/prismatic-settings"
fi
