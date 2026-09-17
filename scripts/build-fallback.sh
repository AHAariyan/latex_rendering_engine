#!/usr/bin/env bash
# Builds assets/fonts/fallback-subset.otf: the glyphs the primary bundled font
# lacks, taken from STIX Two Math, so every symbol the parser accepts renders
# out of the box. Needs pyftsubset from fontTools.
set -euo pipefail
cd "$(dirname "$0")/.."
LIST=$(mktemp)
cargo run -q -p mathcore --example charset --features missing-only > "$LIST" 2>/dev/null || {
  cargo run -q -p mathcore --example gaps > "$LIST"
}
echo "filling $(wc -l < "$LIST") code points"
"${PYFTSUBSET:-pyftsubset}" assets/fonts/STIXTwoMath-Regular.otf \
  --output-file=assets/fonts/fallback-subset.otf --unicodes-file="$LIST" \
  --layout-features='*' --glyph-names --notdef-outline --no-hinting --desubroutinize \
  --drop-tables-=MATH --passthrough-tables
"${PYTHON:-python3}" tools/subset/repair_math.py assets/fonts/STIXTwoMath-Regular.otf assets/fonts/fallback-subset.otf
rm -f "$LIST"
ls -la assets/fonts/fallback-subset.otf
