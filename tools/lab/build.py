#!/usr/bin/env python3
"""Builds the self-contained Test Lab page: the engine (WebAssembly, inlined),
MathJax and the browser's MathML side by side, and the field-by-field record
against LaTeX from tools/sectors.

    cargo xtask sdk web --no-verify
    python3 tools/sectors/compare.py
    python3 tools/lab/build.py [out.html]
"""
import base64, json, os, struct, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "target", "mathcore-test-lab.html")
template = open(os.path.join(ROOT, "tools/lab/template.html")).read()
glue = open(os.path.join(ROOT, "dist/web/package/wasm/mathwasm.js")).read()
wasm = base64.b64encode(open(os.path.join(ROOT, "dist/web/package/wasm/mathwasm_bg.wasm"), "rb").read()).decode()
data = []
for e in json.load(open(os.path.join(ROOT, "target/sectors/results.json"))):
    png = open(os.path.join(ROOT, f"target/sectors/{e['sector']}-{e['name']}-tex.png"), "rb").read()
    data.append({"sector": e["sector"], "name": e["name"], "tex": e["tex"], "score": e["score"],
                 "w": struct.unpack(">I", png[16:20])[0], "img": base64.b64encode(png).decode()})
html = template.replace("/*GLUE*/", glue).replace("/*WASM*/", wasm).replace("/*DATA*/", json.dumps(data))
open(out, "w").write(html)
print(f"{out}: {len(html) / 1e6:.1f} MB, {len(data)} formulas")
