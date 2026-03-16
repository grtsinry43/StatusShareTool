#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/apps/macos-swiftui/Generated}"

cd "$ROOT_DIR"

cargo build -p statusshare-core --release
mkdir -p "$OUT_DIR"

LIB_PATH=""
for candidate in \
  "$ROOT_DIR/target/release/libstatusshare_core.dylib" \
  "$ROOT_DIR/target/release/libstatusshare_core.so" \
  "$ROOT_DIR/target/release/libstatusshare_core.a"
do
  if [[ -f "$candidate" ]]; then
    LIB_PATH="$candidate"
    break
  fi
done

if [[ -z "$LIB_PATH" ]]; then
  echo "Could not find built statusshare-core library artifact in target/release."
  exit 1
fi

cargo run -p statusshare-core --bin uniffi-bindgen -- \
  generate \
  --library "$LIB_PATH" \
  --language swift \
  --out-dir "$OUT_DIR"

echo "Swift bindings generated in: $OUT_DIR"
