#!/usr/bin/env bash
# Subsets a math font to the code points the engine can reach, keeping the
# MATH table (size variants, assemblies, kerning) for the retained glyphs and
# repairing the records fontTools drops (see tools/subset/repair_math.py).
# Usage: scripts/subset-font.sh <in.otf> <out.otf>   (needs `pyftsubset` from fontTools)
set -euo pipefail
cd "$(dirname "$0")/.."
IN="$1"; OUT="$2"
LIST=$(mktemp)
cargo run -q -p mathcore --example charset > "$LIST"
"${PYFTSUBSET:-pyftsubset}" "$IN" --output-file="$OUT" --unicodes-file="$LIST" \
  --layout-features='*' --glyph-names --notdef-outline --no-hinting --desubroutinize \
  --drop-tables-=MATH --passthrough-tables
rm -f "$LIST"
"${PYTHON:-python3}" tools/subset/repair_math.py "$IN" "$OUT"
ls -la "$IN" "$OUT"
