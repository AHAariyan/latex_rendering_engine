#!/usr/bin/env python3
"""Side-by-side comparison of mathcore output against real LaTeX.

For every formula in the golden corpus this renders (a) our engine via
`mathcli` and (b) LuaLaTeX + unicode-math with the same Latin Modern Math
font, rasterized by pdftoppm at the same pixels-per-em, then stacks the pairs
into one contact sheet (ours above, TeX below) for visual review, and prints
the size ratio per formula so drift shows up as a number.

Usage: tools/texcompare/compare.py [--out sheet.png] [--size 32] [--tex 'formula' | --inline 'formula'] [name ...]
Needs: lualatex (TinyTeX is fine), pdftoppm, a built target/release/mathcli.
"""
import os, re, struct, subprocess, sys, tempfile, zlib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CORPUS_RS = os.path.join(ROOT, "crates/mathraster/tests/common/corpus.rs")
FONT = os.path.join(ROOT, "assets/fonts/latinmodern-math.otf")

def corpus():
    src = open(CORPUS_RS).read()
    body = src[src.index("const CORPUS"):src.index("];", src.index("const CORPUS"))]
    out = []
    for m in re.finditer(r'\(\s*"([a-z0-9_]+)",\s*(r?)"((?:[^"\\]|\\.)*)",\s*(true|false)\s*,?\s*\)', body, re.S):
        name, raw, tex, display = m.group(1), m.group(2) == "r", m.group(3), m.group(4) == "true"
        if not raw:
            tex = tex.replace("\\\\", "\\").replace('\\"', '"')
        out.append((name, tex, display))
    return out

def read_png(path):
    data = open(path, "rb").read(); pos = 8; w = h = 0; idat = b""; ct = 6
    while pos < len(data):
        ln = struct.unpack(">I", data[pos:pos+4])[0]; typ = data[pos+4:pos+8]; body = data[pos+8:pos+8+ln]; pos += 12 + ln
        if typ == b"IHDR": w, h, bd, ct = struct.unpack(">IIBB", body[:10])
        elif typ == b"IDAT": idat += body
    bpp = 4 if ct == 6 else (3 if ct == 2 else 1)
    raw = zlib.decompress(idat); rows = []; stride = w * bpp; prev = bytearray(stride); i = 0
    for _ in range(h):
        f = raw[i]; i += 1; line = bytearray(raw[i:i+stride]); i += stride
        for x in range(stride):
            a = line[x-bpp] if x >= bpp else 0; b = prev[x]; c = prev[x-bpp] if x >= bpp else 0
            if f == 1: line[x] = (line[x] + a) & 255
            elif f == 2: line[x] = (line[x] + b) & 255
            elif f == 3: line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b-c), abs(a-c), abs(a+b-2*c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else (b if pb <= pc else c))) & 255
        rows.append(bytes(line)); prev = line
    if bpp == 3: rows = [b"".join(r[k:k+3] + b"\xff" for k in range(0, len(r), 3)) for r in rows]
    if bpp == 1: rows = [b"".join(bytes([v, v, v, 255]) for v in r) for r in rows]
    return w, h, rows

def write_png(path, w, h, rows):
    raw = b"".join(b"\x00" + r for r in rows)
    def chunk(t, b): return struct.pack(">I", len(b)) + t + b + struct.pack(">I", zlib.crc32(t + b) & 0xffffffff)
    open(path, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))

def ink_bbox(w, h, rows):
    xs, ys = [], []
    for y, r in enumerate(rows):
        for x in range(w):
            if r[x*4] < 128 and r[x*4+3] > 0: xs.append(x); ys.append(y)
    return (min(xs), min(ys), max(xs) + 1, max(ys) + 1) if xs else (0, 0, 0, 0)

def render_ours(tex, display, size, path):
    cli = os.path.join(ROOT, "target/release/mathcli")
    args = [cli, tex, "-o", path, "--size", str(size), "--scale", "1", "--padding", "4"]
    if not display: args.append("--inline")
    subprocess.run(args, check=True)

def render_tex(tex, display, size, path, tmp):
    """Typesets at TeX's standard 10 pt so absolute dimensions (\arraycolsep,
    \fboxsep, \jot, \delimitershortfall...) mean what they mean in a normal
    document, then rasterizes at the DPI that makes 1 em = `size` px."""
    body = ("$\\displaystyle " + tex + "$") if display else ("$" + tex + "$")
    doc = r"""\documentclass[border=1pt,10pt]{standalone}
\usepackage{amsmath,amssymb,mathtools,cancel,xcolor}
\usepackage{unicode-math}
\setmathfont{latinmodern-math.otf}
\setmainfont{latinmodern-math.otf}
\begin{document}
%s
\end{document}
""" % body
    src = os.path.join(tmp, "f.tex")
    open(src, "w").write(doc)
    r = subprocess.run(["lualatex", "-interaction=nonstopmode", "-halt-on-error", "-output-directory", tmp, src],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return False
    dpi = 72.27 * size / 10.0
    subprocess.run(["pdftoppm", "-r", f"{dpi:.3f}", "-png", "-singlefile", os.path.join(tmp, "f.pdf"), path[:-4]], check=True)
    return True

def main():
    args = sys.argv[1:]
    out = "texcompare.png"; size = 32; names = []; adhoc = []
    while args:
        a = args.pop(0)
        if a == "--out": out = args.pop(0)
        elif a == "--size": size = int(args.pop(0))
        elif a == "--tex": adhoc.append((args.pop(0), True))
        elif a == "--inline": adhoc.append((args.pop(0), False))
        else: names.append(a)
    tmp = tempfile.mkdtemp()
    pairs = []
    cases = corpus()
    if adhoc:
        cases = [(f"adhoc{i}", t, d) for i, (t, d) in enumerate(adhoc)]
        names = []
    for name, tex, display in cases:
        if names and name not in names: continue
        ours = os.path.join(tmp, name + ".ours.png"); theirs = os.path.join(tmp, name + ".tex.png")
        try:
            render_ours(tex, display, size, ours)
        except subprocess.CalledProcessError:
            print(f"{name:20s} ours: FAILED"); continue
        if not render_tex(tex, display, size, theirs, tmp):
            print(f"{name:20s} tex: FAILED (see {tmp}/f.log)"); continue
        a, b = read_png(ours), read_png(theirs)
        ba, bb = ink_bbox(*a), ink_bbox(*b)
        wa, ha = ba[2]-ba[0], ba[3]-ba[1]; wb, hb = bb[2]-bb[0], bb[3]-bb[1]
        print(f"{name:20s} ours {wa:4d}x{ha:<3d} tex {wb:4d}x{hb:<3d}  width {wa/wb if wb else 0:5.2f}  height {ha/hb if hb else 0:5.2f}")
        pairs.append((name, a, b))
    if not pairs: return
    W = max(max(a[0], b[0]) for _, a, b in pairs) + 8
    H = sum(a[1] + b[1] + 10 for _, a, b in pairs)
    sheet = [bytearray(b"\xff\xff\xff\xff" * W) for _ in range(H)]
    y = 0
    for _, a, b in pairs:
        for r in a[2]: sheet[y][0:a[0]*4] = r; y += 1
        for _ in range(2): sheet[y][:] = b"\xd0\xe8\xd0\xff" * W; y += 1
        for r in b[2]: sheet[y][0:b[0]*4] = r; y += 1
        for _ in range(8): sheet[y][:] = b"\x90\x90\xc0\xff" * W; y += 1
    write_png(out, W, H, [bytes(r) for r in sheet])
    print("sheet:", out)

if __name__ == "__main__":
    main()
