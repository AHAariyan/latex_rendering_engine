// Every command, symbol, macro and environment KaTeX defines, each with the
// smallest input KaTeX accepts, as `name<TAB>tex` lines. Feeding the inputs to
// corpus_check shows which ones mathcore lacks.
//   node tools/corpus/katex_commands.mjs > katex_commands.tsv
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const katex = require("katex");
const src = readFileSync(require.resolve("katex/dist/katex.js"), "utf8");

const names = new Set();
for (const m of src.matchAll(/defineSymbol\(\s*math\s*,[^,]+,[^,]+,[^,]+,\s*"((?:\\\\|[^"\\])+)"/g)) names.add(m[1].replace(/\\\\/g, "\\"));
for (const m of src.matchAll(/defineMacro\(\s*"((?:\\\\|[^"\\])+)"/g)) names.add(m[1].replace(/\\\\/g, "\\"));
for (const block of src.matchAll(/names:\s*\[([^\]]*)\]/g))
  for (const m of block[1].matchAll(/"((?:\\\\|[^"\\])+)"/g)) names.add(m[1].replace(/\\\\/g, "\\"));

const ok = (tex) => { try { katex.renderToString(tex, { displayMode: true, throwOnError: true, strict: "ignore" }); return true; } catch { return false; } };
const out = [];
for (const name of [...names].sort()) {
  let candidates;
  if (!name.startsWith("\\")) {
    // An environment name.
    if (!/^[a-zA-Z*]+$/.test(name)) continue;
    candidates = [`\\begin{${name}}a&b\\\\c&d\\end{${name}}`, `\\begin{${name}}{cc}a&b\\\\c&d\\end{${name}}`, `\\begin{${name}}{2}a&b\\\\c&d\\end{${name}}`, `\\begin{${name}}a\\end{${name}}`];
  } else {
    const n = name;
    candidates = [`${n}`, `${n}{a}`, `${n}{a}{b}`, `${n}{a}{b}{c}`, `${n}{red}{a}`, `${n}{1em}`, `${n}{1em}{1em}`, `${n}[1]{a}`, `${n}(`, `\\left${n}a\\right.`, `a${n}b`, `${n}{a}{b}{c}{d}{e}{f}`, `\\text{${n}}`, `\\text{${n}{a}}`, `${n}{red}{blue}{a}`, `${n}{Na}`, `${n}{a}{b}{c}{d}`, `${n}{0.5}{red}{a}`];
  }
  const tex = candidates.find(ok);
  if (tex) out.push(`${name}\t${tex}`);
}
process.stdout.write(out.join("\n") + "\n");
