#!/usr/bin/env bash
# Builds the wasm binding and serves platforms/web/bench.html, which renders
# the regression corpus with mathcore, KaTeX and MathJax side by side.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v wasm-pack >/dev/null || cargo install wasm-pack
rustup target add wasm32-unknown-unknown >/dev/null
(cd crates/mathwasm && wasm-pack build --release --target web --out-dir ../../platforms/web/pkg)
python3 scripts/web-corpus.py
echo "Open http://localhost:${PORT:-8000}/bench.html"
cd platforms/web && exec python3 -m http.server "${PORT:-8000}"
