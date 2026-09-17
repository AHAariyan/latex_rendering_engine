// Node smoke test for the wasm package: node platforms/web/test.mjs
import { createRequire } from "node:module";
import assert from "node:assert/strict";
const require = createRequire(import.meta.url);
const { MathEngine, version } = require("./pkg-node/mathwasm.js");

const engine = new MathEngine();
assert.equal(engine.unitsPerEm(), 1000);
console.log("mathwasm", version());

const svg = engine.renderSvg("x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}", 32, true, 0xff000000, null);
assert.ok(svg.startsWith("<svg"), "svg output");
assert.ok(svg.includes("<use "), "glyph uses");
assert.ok(svg.includes("<rect "), "fraction rule");

const flat = engine.render("\\sum_{i=1}^n i^2", 32, true, 0xffff0000, null, 0);
assert.ok(flat instanceof Float32Array);
const count = flat[3];
assert.ok(count === 7 && flat.length === 4 + count * 8, "flat layout shape");
const bits = new Uint32Array(new Float32Array([flat[4 + 7]]).buffer)[0];
assert.equal(bits, 0xffff0000, "color round-trips");

const withMacro = engine.renderSvg("\\R", 32, true, 0xff000000, "\\R=\\mathbb{R}", 0);
assert.ok(withMacro.includes("<use "), "macro expanded");

let threw = false;
try { engine.renderSvg("\\frac{a", 32, true, 0xff000000, null, 0); } catch (e) { threw = /parse error/.test(String(e.message || e)); }
assert.ok(threw, "parse error surfaced as exception");

const wide = engine.render("a + b + c + d + e + f + g + h + i + j", 32, true, 0xff000000, null, 0);
const narrow = engine.render("a + b + c + d + e + f + g + h + i + j", 32, true, 0xff000000, null, 150);
assert.ok(narrow[0] <= 150 && narrow[0] < wide[0], "line breaking respects the width");
assert.ok(narrow[1] + narrow[2] > wide[1] + wide[2], "broken layout is taller");

const outline = engine.glyphOutline(flat[5]);
assert.ok(outline && outline.length > 3 && outline[0] === 0, "outline starts with move");

const N = 200, t0 = performance.now();
for (let i = 0; i < N; i++) engine.render("x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}", 32, true, 0xff000000, null, 0);
console.log(`render x${N}: ${((performance.now() - t0) / N).toFixed(3)} ms each`);
console.log("all wasm checks passed");
