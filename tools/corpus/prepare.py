#!/usr/bin/env python3
"""Turns the im2latex-100k formula list (arXiv) into one formula per line.

    curl -L -o im2latex_formulas.lst 'https://zenodo.org/record/56198/files/im2latex_formulas.lst?download=1'
    python3 tools/corpus/prepare.py im2latex_formulas.lst > formulas.txt

`\\label{...}` is dropped: it is document markup, not typesetting, and every
engine would otherwise reject a third of the corpus for it. Stray carriage
returns are spaces. Nothing else is touched.
"""
import re, sys

raw = open(sys.argv[1], encoding="latin-1", newline="").read()
label = re.compile(r"\\label\s*\{[^{}]*\}")
for line in raw.split("\n"):
    tex = label.sub("", line.replace("\r", " ")).strip()
    if tex:
        print(tex)
