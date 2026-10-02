#!/usr/bin/env python3
"""Writes the regression corpus to platforms/web/corpus.json for bench.html."""
import json, pathlib, re

root = pathlib.Path(__file__).resolve().parent.parent
src = (root / "crates/mathraster/tests/common/corpus.rs").read_text()
entries = re.findall(r'\(\s*"([^"]+)",\s*r(#*)"(.*?)"\2,\s*(true|false),?\s*\)', src, re.S)
corpus = [{"name": n, "tex": t, "display": d == "true"} for n, _, t, d in entries]
out = root / "platforms/web/corpus.json"
out.write_text(json.dumps(corpus, indent=1, ensure_ascii=False) + "\n")
print(f"{len(corpus)} formulas -> {out.relative_to(root)}")
