#!/usr/bin/env bash
# The real-world gate: ~100k arXiv formulas through mathcore and KaTeX, every
# accepted formula checked for invariants, then mutation fuzzing on a 256 KB
# stack. Prints a summary; exits non-zero on any violation, panic or fuzz failure.
#   tools/corpus/run.sh [WORKDIR]       (default: target/corpus)
set -euo pipefail
cd "$(dirname "$0")/../.."
W="${1:-target/corpus}"; mkdir -p "$W"
[ -f "$W/im2latex_formulas.lst" ] || curl -sSL -o "$W/im2latex_formulas.lst" 'https://zenodo.org/record/56198/files/im2latex_formulas.lst?download=1'
python3 tools/corpus/prepare.py "$W/im2latex_formulas.lst" > "$W/formulas.txt"
[ -d tools/corpus/node_modules/katex ] || npm install --silent --prefix tools/corpus katex@0.16.11
node tools/corpus/katex_check.mjs "$W/formulas.txt" > "$W/katex.tsv" 2>/dev/null
cargo build --profile fuzz -q -p mathcore --example corpus_check --example fuzz_mutate
target/fuzz/examples/corpus_check "$W/formulas.txt" > "$W/mathcore.tsv"
paste <(cut -f2,4 "$W/mathcore.tsv") <(cut -f2,4 "$W/katex.tsv") > "$W/joined.tsv"
echo "== formulas: $(wc -l < "$W/formulas.txt")"
awk -F'\t' '{print "mathcore=" $1 ", katex=" $3}' "$W/joined.tsv" | sort | uniq -c
echo "== KaTeX accepts, mathcore rejects, by reason"
awk -F'\t' '$1=="reject" && $3=="ok" {m=$2; gsub(/at byte [0-9]+:? ?/, "", m); print m}' "$W/joined.tsv" | sort | uniq -c | sort -rn | head -15
bad=$(awk -F'\t' '$2=="violation" || $2=="panic"' "$W/mathcore.tsv" | wc -l | tr -d ' ')
echo "== invariant violations or panics: $bad"
awk -F'\t' '$2=="violation" || $2=="panic" {print $2, $4}' "$W/mathcore.tsv" | sort | uniq -c | sort -rn | head
fuzz=0
for seed in 1 2 3 4; do target/fuzz/examples/fuzz_mutate "$W/formulas.txt" "$seed" 25000 | tail -3 || fuzz=1; done
[ "$bad" = 0 ] && [ "$fuzz" = 0 ]
