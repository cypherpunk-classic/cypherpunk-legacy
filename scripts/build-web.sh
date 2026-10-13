#!/usr/bin/env bash
# Build the browser version into dist/ (a static site: index.html + wasm).
#
# Needs, once:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
# Optional, for a smaller download:  wasm-opt (from binaryen)
set -euo pipefail
cd "$(dirname "$0")/.."

want=$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/version = |"/, ""); print}' Cargo.lock)
have=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [ "$have" != "$want" ]; then
  echo "wasm-bindgen-cli $want is needed (found: ${have:-none}):"
  echo "  cargo install wasm-bindgen-cli --version $want"
  exit 1
fi

cargo build --profile web-release --target wasm32-unknown-unknown
rm -rf dist
mkdir -p dist
wasm-bindgen --no-typescript --target web --out-dir dist --out-name cypherpunk-legacy \
  target/wasm32-unknown-unknown/web-release/cypherpunk-legacy.wasm
if command -v wasm-opt >/dev/null 2>&1; then
  echo "optimising with wasm-opt…"
  wasm-opt -Os --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types \
    -o dist/cypherpunk-legacy_bg.wasm dist/cypherpunk-legacy_bg.wasm
fi
cp web/index.html dist/index.html
touch dist/.nojekyll
du -sh dist/*.wasm
echo "done: serve dist/ with any static web server, e.g.  python3 -m http.server -d dist 8080"
