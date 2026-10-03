# Field-by-field comparison against LaTeX

`formulas.tsv` holds 293 formulas from 15 fields (algebra, calculus, linear
algebra, statistics, physics, chemistry, engineering, CS and logic, finance,
geometry, number theory, discrete maths, school maths, and the physics and
siunitx packages, typeset in LaTeX with those packages loaded). `compare.py` renders
each with mathcore and with LuaLaTeX (same Latin Modern Math font, same size,
real display math) and scores the ink overlap with a 0.05 em tolerance.

    cargo build --release -p mathcli
    python3 tools/sectors/compare.py      # needs lualatex, pdftoppm, node

Current: 276/293 at ≥ 0.95, mean 0.989; physics package 27/27, siunitx 22/22.

Known, intended differences:
- `\binom`: ours follows classic TeX (pdfLaTeX and LuaLaTeX with Computer
  Modern measure the same parenthesis); LuaLaTeX with OpenType Latin Modern
  picks a larger one.
- `\mathbf` in the reference comes out regular because the harness's text
  font has no bold; ours is bold.
