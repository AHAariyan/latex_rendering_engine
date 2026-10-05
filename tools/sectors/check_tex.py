#!/usr/bin/env python3
"""Checks that every formula of a .tsv compiles in LaTeX with the comparison
preamble (so a test formula is real LaTeX before it is used to judge us), and
that its lines are well formed and its names unique.

Usage: tools/sectors/check_tex.py FILE.tsv
"""
import os, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "texcompare"))
import compare as texcompare  # noqa: E402

PACKAGES = {"siunitx": ("siunitx",), "physicspkg": ("physics",)}


def main():
    path = sys.argv[1]
    bad, names = 0, set()
    with tempfile.TemporaryDirectory() as tmp:
        for n, line in enumerate(open(path, encoding="utf-8"), 1):
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != 3:
                print(f"line {n}: needs sector<TAB>name<TAB>tex, has {len(parts)} fields"); bad += 1; continue
            sector, name, tex = parts
            if (sector, name) in names:
                print(f"line {n}: duplicate name {name}"); bad += 1
            names.add((sector, name))
            if not texcompare.render_tex(tex, True, 40, os.path.join(tmp, "x.png"), tmp, PACKAGES.get(sector, ())):
                print(f"line {n}: LaTeX rejects {name}: {tex}"); bad += 1
    print(f"{len(names)} formulas, {bad} problem(s)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
