#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

cargo build -p windows-pinvoke --release

echo "Header: $ROOT_DIR/crates/windows-pinvoke/include/statusshare_c_api.h"
echo "Artifacts:"
find "$ROOT_DIR/target/release" -maxdepth 1 \
  \( -name "libwindows_pinvoke.so" -o -name "libwindows_pinvoke.dylib" -o -name "windows_pinvoke.dll" \) \
  -print

