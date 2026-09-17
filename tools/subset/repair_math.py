#!/usr/bin/env python3
"""Restores MATH records that fontTools' subsetter drops.

`pyftsubset` prunes MathItalicsCorrectionInfo and MathTopAccentAttachment to
the glyphs it can reach from the cmap, so glyphs that are only reachable
through GSUB — the `ssty` script-style alternates a math engine needs — keep
their outlines but lose their italic correction and accent attachment point.
This copies those records back from the original font for every retained
glyph. Glyph names are matched, so the subset must be produced with
`--glyph-names`.

Usage: repair_math.py <original.otf> <subset.otf>
"""
import sys

from fontTools.ttLib import TTFont
from fontTools.ttLib.tables import otTables as ot


def value_record(v):
    r = ot.MathValueRecord()
    r.Value = v
    r.DeviceTable = None
    return r


def coverage(glyphs):
    c = ot.Coverage()
    c.glyphs = list(glyphs)
    return c


def restore(original, subset, read, write, label):
    """Rebuilds one glyph-keyed MATH subtable over every retained glyph."""
    src = read(original)
    if src is None:
        return 0
    values = dict(zip(src[0].glyphs, src[1]))
    order = subset.getGlyphOrder()
    kept = [g for g in order if g in values]
    before = len(read(subset)[0].glyphs) if read(subset) else 0
    write(subset, coverage(kept), [value_record(values[g].Value) for g in kept])
    print(f"  {label}: {before} -> {len(kept)} records")
    return len(kept) - before


def main():
    original, subset = TTFont(sys.argv[1]), TTFont(sys.argv[2])
    if "MATH" not in original or "MATH" not in subset:
        print("no MATH table; nothing to repair")
        return

    def italics(font):
        info = font["MATH"].table.MathGlyphInfo.MathItalicsCorrectionInfo
        return (info.Coverage, info.ItalicsCorrection) if info else None

    def set_italics(font, cov, vals):
        info = font["MATH"].table.MathGlyphInfo.MathItalicsCorrectionInfo
        info.Coverage, info.ItalicsCorrection, info.ItalicsCorrectionCount = cov, vals, len(vals)

    def accents(font):
        info = font["MATH"].table.MathGlyphInfo.MathTopAccentAttachment
        return (info.TopAccentCoverage, info.TopAccentAttachment) if info else None

    def set_accents(font, cov, vals):
        info = font["MATH"].table.MathGlyphInfo.MathTopAccentAttachment
        info.TopAccentCoverage, info.TopAccentAttachment, info.TopAccentAttachmentCount = cov, vals, len(vals)

    print(f"repairing MATH records in {sys.argv[2]}")
    restore(original, subset, italics, set_italics, "italic corrections")
    restore(original, subset, accents, set_accents, "top accent attachments")
    subset.save(sys.argv[2])


if __name__ == "__main__":
    main()
