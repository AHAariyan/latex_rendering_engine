// Checks that the WebAssembly binding lays out every corpus formula exactly as
// the Rust core does. Run through scripts/parity.sh.
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import assert from "node:assert/strict";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const { MathEngine } = require("./pkg-node/mathwasm.js");
const expected = JSON.parse(readFileSync(join(here, "../../tests/parity/expected.json"), "utf8"));

const engine = new MathEngine();
const round = (v) => Math.round(v * 1000) / 1000;
const bitsToArgb = (f) => new Uint32Array(new Float32Array([f]).buffer)[0];
const TOL = 0.002; // the reference is rounded to three decimals

let items = 0;
for (const c of expected.cases) {
  const flat = engine.render(c.tex, expected.fontSize, c.display, 0xff000000, null, c.maxWidth);
  const near = (a, b, what) =>
    assert.ok(Math.abs(a - b) <= TOL, `${c.name}: ${what} is ${a}, reference says ${b}`);
  near(round(flat[0]), c.width, "width");
  near(round(flat[1]), c.ascent, "ascent");
  near(round(flat[2]), c.descent, "descent");
  assert.equal(flat[3], c.items.length, `${c.name}: item count`);
  for (let n = 0, i = 4; n < c.items.length; n++, i += 8) {
    const [k, g, a, b, w, h, t, col] = c.items[n];
    assert.equal(flat[i], k, `${c.name}: item ${n} kind`);
    assert.equal(flat[i + 1], g, `${c.name}: item ${n} glyph`);
    near(round(flat[i + 2]), a, `item ${n} x`);
    near(round(flat[i + 3]), b, `item ${n} y`);
    near(round(flat[i + 4]), w, `item ${n} w`);
    near(round(flat[i + 5]), h, `item ${n} h`);
    near(round(flat[i + 6]), t, `item ${n} thickness`);
    assert.equal(bitsToArgb(flat[i + 7]), col, `${c.name}: item ${n} color`);
    items++;
  }
}
console.log(`wasm parity: ${expected.cases.length} formulas, ${items} items identical to the core`);
