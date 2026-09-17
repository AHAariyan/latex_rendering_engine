#!/usr/bin/env bash
# Checks that every binding lays out the corpus exactly as the Rust core does.
# Pass --update to regenerate the reference first.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "${1:-}" = "--update" ]; then
  cargo run -q -p mathcore --example parity_dump > tests/parity/expected.json
  echo "reference regenerated"
fi

cargo build --release -p mathffi
(cd crates/mathwasm && wasm-pack build --release --target nodejs --out-dir ../../platforms/web/pkg-node)
node platforms/web/parity.mjs
(cd platforms/dart/mathcore_dart && dart pub get >/dev/null && dart test test/parity_test.dart)
