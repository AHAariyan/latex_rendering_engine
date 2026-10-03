import { chromium, firefox, webkit } from "playwright";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join } from "node:path";
const root = process.argv[2];
const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json" };
const server = createServer(async (req, res) => {
  try {
    const body = await readFile(join(root, decodeURIComponent(req.url.split("?")[0])));
    res.writeHead(200, { "content-type": types[extname(req.url)] ?? "application/octet-stream" }).end(body);
  } catch { res.writeHead(404).end(); }
}).listen(8899);
let failed = false;
for (const [name, engine] of [["Chromium", chromium], ["Firefox", firefox], ["WebKit (Safari)", webkit]]) {
  const browser = await engine.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await page.goto("http://localhost:8899/demo.html");
  await page.waitForFunction(() => [...document.querySelectorAll("math-tex")].every((e) => e.shadowRoot?.querySelector("svg, [part=error]")), null, { timeout: 20000 });
  const result = await page.evaluate(() => [...document.querySelectorAll("math-tex")].map((e) => {
    const svg = e.shadowRoot.querySelector("svg");
    const r = (svg ?? e).getBoundingClientRect();
    return { label: e.getAttribute("aria-label") ?? e.getAttribute("data-error"), w: Math.round(r.width), h: Math.round(r.height), svg: !!svg };
  }));
  
  const drawn = result.filter((r) => r.svg && r.w > 20).length;
  const wrapped = result[5];
  const spanish = result[6].label; const errorShown = result[7].label?.includes("unknown command");
  const ok = errors.length === 0 && drawn === 7 && wrapped.w <= 262 && wrapped.h > 40 && spanish === "x al cuadrado más y al cuadrado igual a z al cuadrado" && errorShown;
  if (!ok) failed = true;
  console.log(`${ok ? "PASS" : "FAIL"}  ${name}: ${drawn}/7 drawn, wrapped ${wrapped.w}x${wrapped.h}, es="${spanish}", errors=${errors.length ? errors.join(" | ") : "none"}`);
  await browser.close();
}
server.close();
process.exit(failed ? 1 : 0);
