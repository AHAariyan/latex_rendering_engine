#!/usr/bin/env python3
"""Checks speech translation files without compiling: every phrase in
keys.rs translated exactly once, slots kept, nothing empty, valid strings.

Usage: tools/speech/validate.py [lang ...]   (module names: es, zh_hans, ...)
"""
import os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
DIR = os.path.join(HERE, "..", "..", "crates", "mathcore", "src", "speech_lang")
STR = r'"((?:[^"\\]|\\.)*)"'
PAIR = re.compile(r'^\s*\(' + STR + r'\s*,\s*' + STR + r'\s*\),\s*(//.*)?$')


def unescape(s):
    return s.replace('\\"', '"').replace("\\\\", "\\")


def pairs(path):
    out, errors = [], []
    body = open(path, encoding="utf-8").read()
    inside = False
    for n, line in enumerate(body.splitlines(), 1):
        if "&[" in line and "static" in line:
            inside = True
            continue
        if inside and line.strip() == "];":
            break
        if not inside or not line.strip() or line.strip().startswith("//"):
            continue
        m = PAIR.match(line)
        if not m:
            errors.append(f"line {n}: not a (\"key\", \"text\") pair: {line.strip()[:80]}")
            continue
        out.append((unescape(m.group(1)), unescape(m.group(2))))
    return out, errors


def slots(s):
    return sorted(int(x) for x in re.findall(r"\{(\d)\}", s))


def check(module):
    keys = [k for k, _ in pairs(os.path.join(DIR, "keys.rs"))[0]]
    got, errors = pairs(os.path.join(DIR, module + ".rs"))
    seen = set()
    for k, t in got:
        if k not in keys:
            errors.append(f"unknown key: {k!r}")
        if k in seen:
            errors.append(f"duplicate key: {k!r}")
        seen.add(k)
        if not t.strip():
            errors.append(f"empty translation: {k!r}")
        if slots(k) != slots(t):
            errors.append(f"slots differ: {k!r} -> {t!r}")
        if re.search(r"\{[^0-9]", t) or "{}" in t:
            errors.append(f"stray brace: {t!r}")
    missing = [k for k in keys if k not in seen]
    if missing:
        errors.append(f"missing {len(missing)}: {missing}")
    return errors


if __name__ == "__main__":
    mods = sys.argv[1:] or sorted(f[:-3] for f in os.listdir(DIR) if f.endswith(".rs") and f not in ("keys.rs", "tests.rs"))
    bad = 0
    for m in mods:
        errs = check(m)
        print(f"{m:8} {'ok' if not errs else f'{len(errs)} problem(s)'}")
        for e in errs[:40]:
            print("   ", e)
        bad += bool(errs)
    sys.exit(1 if bad else 0)
