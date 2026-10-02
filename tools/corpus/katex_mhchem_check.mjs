// katex_check.mjs with the mhchem extension loaded, for chemistry inputs.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const katex = require("katex");
require("katex/contrib/mhchem");
const lines = readFileSync(process.argv[2], "utf8").split("\n").filter(Boolean);
for (const [i, tex] of lines.entries()) {
  try { katex.renderToString(tex, { displayMode: true, throwOnError: true }); console.log(`${i}\tok`); }
  catch (e) { console.log(`${i}\treject\t${String(e.message).slice(0, 120)}`); }
}
