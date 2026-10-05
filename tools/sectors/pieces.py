#!/usr/bin/env python3
"""Lists the bounding box of every connected piece of ink in our drawing and
in the LaTeX one, side by side, to see exactly where two renderings differ.

Usage: tools/sectors/pieces.py OURS.png TEX.png   (boxes in em at 40 px/em,
relative to the left and top of the ink)
"""
import os, sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "texcompare"))
import compare as texcompare  # noqa: E402

EM = 40.0


def pieces(path):
    w, h, rows = texcompare.read_png(path)
    ink = {(x, y) for y in range(h) for x in range(w)
           if rows[y][4 * x + 3] > 128 and rows[y][4 * x] + rows[y][4 * x + 1] + rows[y][4 * x + 2] < 384}
    if not ink:
        return []
    x0 = min(p[0] for p in ink); y0 = min(p[1] for p in ink)
    out, seen = [], set()
    for p in sorted(ink):
        if p in seen:
            continue
        stack, box = [p], [p[0], p[1], p[0], p[1]]
        seen.add(p)
        while stack:
            x, y = stack.pop()
            box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    q = (x + dx, y + dy)
                    if q in ink and q not in seen:
                        seen.add(q); stack.append(q)
        out.append(((box[0] - x0) / EM, (box[1] - y0) / EM, (box[2] - x0 + 1) / EM, (box[3] - y0 + 1) / EM))
    return sorted(out)


if __name__ == "__main__":
    a, b = pieces(sys.argv[1]), pieces(sys.argv[2])
    print(f"{'ours: left  top   right bottom':34}   {'LaTeX: left  top   right bottom':34}   d-left d-right d-top")
    for i in range(max(len(a), len(b))):
        fa = "%5.2f %5.2f %5.2f %5.2f" % a[i] if i < len(a) else ""
        fb = "%5.2f %5.2f %5.2f %5.2f" % b[i] if i < len(b) else ""
        d = "%+.2f  %+.2f  %+.2f" % (a[i][0] - b[i][0], a[i][2] - b[i][2], a[i][1] - b[i][1]) if i < len(a) and i < len(b) else ""
        print(f"{fa:34}   {fb:34}   {d}")
