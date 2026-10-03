// Text in six scripts through <math-tex>, fonts fetched from the CDN, in
// three browser engines: node scripts.mjs <npm package dir with scripts.html copied in>
import { chromium, firefox, webkit } from "playwright";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join } from "node:path";
const root = process.argv[2];
const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
const server = createServer(async (req, res) => {
  try {
    const body = await readFile(join(root, decodeURIComponent(req.url.split("?")[0])));
    res.writeHead(200, { "content-type": types[extname(req.url)] ?? "application/octet-stream" }).end(body);
  } catch { res.writeHead(404).end(); }
}).listen(8898);
let failed = false;
for (const [name, engine] of [["Chromium", chromium], ["Firefox", firefox], ["WebKit (Safari)", webkit]]) {
  const browser = await engine.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto("http://localhost:8898/scripts.html");
  // Missing characters are drawn as hollow boxes (four rules); none may remain.
  const ok = await page
    .waitForFunction(() => {
      const els = [...document.querySelectorAll("math-tex")];
      return els.every((e) => { const s = e.shadowRoot?.querySelector("svg"); return s && s.querySelectorAll("rect").length === 0 && s.querySelectorAll("use").length > 6; });
    }, null, { timeout: 90000 })
    .then(() => true, () => false);
  const labels = await page.evaluate(() => [...document.querySelectorAll("math-tex")].map((e) => e.getAttribute("aria-label")));
  await page.screenshot({ path: join(root, `scripts-${name.split(" ")[0]}.png`), fullPage: true });
  if (!ok || errors.length) failed = true;
  console.log(`${ok && !errors.length ? "PASS" : "FAIL"}  ${name}  errors=${errors.join(" | ") || "none"}`);
  console.log("   " + labels.join("\n   "));
  await browser.close();
}
server.close();
process.exit(failed ? 1 : 0);
