#!/usr/bin/env python3
"""Renders one formula both ways and lists the pieces side by side.

Usage: tools/sectors/one.py 'TEX' [--inline] [--package NAME]...
"""
import importlib.util, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "tools", "texcompare"))
import compare as texcompare  # noqa: E402

spec = importlib.util.spec_from_file_location("pieces", os.path.join(HERE, "pieces.py"))
pieces = importlib.util.module_from_spec(spec); spec.loader.exec_module(pieces)

args = sys.argv[1:]
tex = args.pop(0)
display = "--inline" not in args
packages = tuple(args[i + 1] for i, a in enumerate(args) if a == "--package")
engine = next((args[i + 1] for i, a in enumerate(args) if a == "--engine"), "lualatex")
d = tempfile.mkdtemp()
ours, theirs = os.path.join(d, "ours.png"), os.path.join(d, "tex.png")
subprocess.run([os.path.join(ROOT, "target", "release", "mathcli"), "--size", "40", "--scale", "1", "--transparent"]
               + ([] if display else ["--inline"]) + [tex, "-o", ours], check=True)
assert texcompare.render_tex(tex, display, 40, theirs, d, packages, engine=engine), "LaTeX failed"
a, b = pieces.pieces(ours), pieces.pieces(theirs)
print(f"{'ours: left  top   right bottom':32} {'LaTeX: left  top   right bottom':32}")
for i in range(max(len(a), len(b))):
    fa = "%5.2f %5.2f %5.2f %5.2f" % a[i] if i < len(a) else ""
    fb = "%5.2f %5.2f %5.2f %5.2f" % b[i] if i < len(b) else ""
    print(f"{fa:32} {fb:32}")
print(ours, theirs)
