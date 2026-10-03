// Runs every formula through KaTeX, the reference for what a web math engine
// accepts. One line per formula: `index  ok|reject  micros  message`.
//   npm install --prefix tools/corpus katex@0.16.11
//   node tools/corpus/katex_check.mjs formulas.txt > katex.tsv
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const katex = createRequire(import.meta.url)("katex");
const lines = readFileSync(process.argv[2], "utf8").split("\n");
if (lines.at(-1) === "") lines.pop();
const out = [];
lines.forEach((tex, i) => {
  const t0 = process.hrtime.bigint();
  let status = "ok", msg = "";
  try {
    katex.renderToString(tex, { displayMode: true, throwOnError: true, strict: "ignore", trust: false });
  } catch (e) {
    status = "reject";
    msg = String(e.message).replace(/[\t\n]/g, " ").slice(0, 200);
  }
  out.push(`${i}\t${status}\t${(process.hrtime.bigint() - t0) / 1000n}\t${msg}`);
});
process.stdout.write(out.join("\n") + "\n");
