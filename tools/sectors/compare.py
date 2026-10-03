#!/usr/bin/env python3
"""Renders every formula in formulas.tsv with mathcore and with real LaTeX
(LuaLaTeX + unicode-math, the same Latin Modern Math font, the same size) and
scores how alike they look, field by field.

The score is the ink overlap of the two drawings after aligning them: both
are cropped to their ink, the TeX one is scaled onto ours, and the share of
ink pixels that have ink within two pixels (0.05 em) in the other drawing is taken both
ways (an F1 score). Identical drawings score 1.0; a misplaced script or a
wrong glyph costs a few points; a wrong structure costs many.

Also records whether KaTeX accepts the formula, and writes report.html with
the formulas side by side, worst first.

Usage: tools/sectors/compare.py [--out DIR] [--only SECTOR]
Needs: lualatex with unicode-math and mhchem, pdftoppm, node with KaTeX in
tools/corpus/node_modules, a built target/release/mathcli.
"""
import base64, html, json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "tools", "texcompare"))
import compare as texcompare  # noqa: E402

SIZE = 40
TOLERANCE = 2


def load(path):
    rows = []
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if line and not line.startswith("#"):
            sector, name, tex = line.split("\t", 2)
            rows.append((sector, name, tex))
    return rows


def ink(path):
    """The drawing as a set of ink pixels and its size, cropped to the ink."""
    w, h, rows = texcompare.read_png(path)
    # Opaque and dark: ours is drawn on transparency, pdftoppm's on white.
    pts = [(x, y) for y in range(h) for x in range(w)
           if rows[y][4 * x + 3] > 128 and rows[y][4 * x] + rows[y][4 * x + 1] + rows[y][4 * x + 2] < 384]
    if not pts:
        return set(), 0, 0
    x0 = min(p[0] for p in pts); y0 = min(p[1] for p in pts)
    x1 = max(p[0] for p in pts); y1 = max(p[1] for p in pts)
    return {(x - x0, y - y0) for x, y in pts}, x1 - x0 + 1, y1 - y0 + 1


def score(ours, theirs):
    a, aw, ah = ours
    b, bw, bh = theirs
    if not a or not b:
        return 0.0
    # Scale TeX's ink onto our box, so a uniform size difference is not
    # counted as a difference in shape (the size ratio is reported apart).
    sx, sy = aw / bw, ah / bh
    b = {(int(x * sx), int(y * sy)) for x, y in b}

    # Two pixels at 40 px/em is 0.05 em: below what a reader can see, above
    # the sub-pixel drift two rasterizers accumulate along a line.
    r = range(-TOLERANCE, TOLERANCE + 1)

    def near(p, s):
        x, y = p
        return any((x + dx, y + dy) in s for dx in r for dy in r)

    precision = sum(near(p, b) for p in a) / len(a)
    recall = sum(near(p, a) for p in b) / len(b)
    return 0.0 if precision + recall == 0 else 2 * precision * recall / (precision + recall)


def katex_accepts(formulas):
    script = r"""
const katex = require("katex"); require("katex/contrib/mhchem");
const lines = require("fs").readFileSync(0, "utf8").split("\n").filter(Boolean);
console.log(JSON.stringify(lines.map(t => { try { katex.renderToString(t, {displayMode: true, throwOnError: true}); return true } catch { return false } })));
"""
    r = subprocess.run(["node", "-e", script], input="\n".join(t for _, _, t in formulas), capture_output=True, text=True,
                       cwd=os.path.join(ROOT, "tools", "corpus"))
    return json.loads(r.stdout) if r.returncode == 0 else [None] * len(formulas)


def main():
    args = sys.argv[1:]
    out = os.path.join(ROOT, "target", "sectors"); only = None
    while args:
        a = args.pop(0)
        if a == "--out": out = args.pop(0)
        elif a == "--only": only = args.pop(0)
    os.makedirs(out, exist_ok=True)
    rows = [r for r in load(os.path.join(HERE, "formulas.tsv")) if only is None or r[0] == only]
    katex = katex_accepts(rows)
    mathcli = os.path.join(ROOT, "target", "release", "mathcli")
    results = []
    with tempfile.TemporaryDirectory() as tmp:
        for (sector, name, tex), kx in zip(rows, katex):
            ours_png = os.path.join(out, f"{sector}-{name}-ours.png")
            tex_png = os.path.join(out, f"{sector}-{name}-tex.png")
            ours_ok = subprocess.run([mathcli, "--size", str(SIZE), "--scale", "1", tex, "-o", ours_png], capture_output=True).returncode == 0
            tex_ok = texcompare.render_tex(tex, True, SIZE, tex_png, tmp)
            entry = {"sector": sector, "name": name, "tex": tex, "ours": ours_ok, "latex": tex_ok, "katex": kx}
            if ours_ok and tex_ok:
                a, b = ink(ours_png), ink(tex_png)
                entry["score"] = round(score(a, b), 3)
                entry["width"] = round(a[1] / b[1], 3) if b[1] else 0
                entry["height"] = round(a[2] / b[2], 3) if b[2] else 0
            results.append(entry)
            s = entry.get("score")
            print(f"{sector:15} {name:24} {'ours' if ours_ok else 'OURS FAILED':11} "
                  f"{'tex' if tex_ok else 'TEX FAILED':10} {'' if s is None else f'match {s:.3f}  size {entry['width']:.2f}x{entry['height']:.2f}'}",
                  flush=True)
    json.dump(results, open(os.path.join(out, "results.json"), "w"), indent=1)
    summary(results)
    report(results, out)


def summary(results):
    print("\nfield            formulas  render  LaTeX  KaTeX  match>=0.95  mean match")
    sectors = sorted({r["sector"] for r in results})
    for s in sectors + ["ALL"]:
        rs = [r for r in results if s == "ALL" or r["sector"] == s]
        scored = [r["score"] for r in rs if "score" in r]
        good = sum(1 for x in scored if x >= 0.95)
        mean = sum(scored) / len(scored) if scored else 0
        print(f"{s:16} {len(rs):8} {sum(r['ours'] for r in rs):7} {sum(r['latex'] for r in rs):6} "
              f"{sum(bool(r['katex']) for r in rs):6} {good:7}/{len(scored):<4} {mean:10.3f}")


def report(results, out):
    def img(path):
        return "data:image/png;base64," + base64.b64encode(open(path, "rb").read()).decode() if os.path.exists(path) else ""
    rows = sorted(results, key=lambda r: r.get("score", -1))
    parts = ["<!doctype html><meta charset=utf-8><title>mathcore vs LaTeX</title>",
             "<style>body{font:14px system-ui;margin:16px}td{border-bottom:1px solid #ddd;padding:6px;vertical-align:top}"
             "img{max-width:520px;display:block}code{font-size:12px}</style><table>"]
    for r in rows:
        stem = os.path.join(out, f"{r['sector']}-{r['name']}")
        parts.append(f"<tr><td><b>{html.escape(r['sector'])}/{html.escape(r['name'])}</b><br>match {r.get('score', '–')} "
                     f"size {r.get('width', '–')}×{r.get('height', '–')}<br><code>{html.escape(r['tex'])}</code></td>"
                     f"<td>mathcore<img src='{img(stem + '-ours.png')}'></td><td>LaTeX<img src='{img(stem + '-tex.png')}'></td></tr>")
    parts.append("</table>")
    open(os.path.join(out, "report.html"), "w").write("".join(parts))
    print(f"\nreport: {os.path.join(out, 'report.html')}")


if __name__ == "__main__":
    main()
