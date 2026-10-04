// <math-field> driven by real keys and clicks in three browser engines:
// node field.mjs <npm package dir with field.html copied in>
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
}).listen(8897);
let failed = false;
for (const [name, engine] of [["Chromium", chromium], ["Firefox", firefox], ["WebKit (Safari)", webkit]]) {
  const browser = await engine.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto("http://localhost:8897/field.html");
  await page.waitForFunction(() => [...document.querySelectorAll("math-field")].every((e) => e.shadowRoot?.querySelector("svg")));
  const value = (id) => page.evaluate((id) => document.getElementById(id).value, id);
  const checks = [];
  const check = (what, got, want) => checks.push([what, got, want, got === want]);

  // Typing: the quadratic formula, keys only.
  await page.click("#a");
  await page.evaluate(() => { window.inputs = 0; document.getElementById("a").addEventListener("input", () => window.inputs++); });
  await page.keyboard.type("x=(-b+sqrtb^2");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.type("-4ac");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.type(")/2a");
  check("typed formula", await value("a"), "x=\\frac{\\left( -b+\\sqrt{b^{2}-4ac} \\right)}{2a}");
  check("input events fired", (await page.evaluate(() => window.inputs)) > 10, true);
  // The caret is in the denominator: below the fraction bar, visible.
  const caret = await page.evaluate(() => {
    const f = document.getElementById("a");
    const c = f.shadowRoot.querySelector(".caret").getBoundingClientRect();
    const s = f.shadowRoot.querySelector("svg").getBoundingClientRect();
    return { shown: c.height > 5, low: c.top > s.top + s.height / 2, inside: c.left > s.left && c.left <= s.right + 2 };
  });
  check("caret in the denominator", JSON.stringify(caret), JSON.stringify({ shown: true, low: true, inside: true }));
  check("spoken position", await page.evaluate(() => document.getElementById("a").shadowRoot.querySelector(".live").textContent), "denominator, a");

  // Undo, select all, replace.
  await page.keyboard.press("Backspace");
  check("backspace", (await value("a")).endsWith("{2}"), true);
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.type("1/2");
  check("select all and replace", await value("a"), "\\frac{1}{2}");
  await page.keyboard.press("ControlOrMeta+z");
  await page.keyboard.press("ControlOrMeta+z");
  check("undo", await value("a"), "1");

  // A click places the cursor: left edge of the second field, then type.
  const box = await page.evaluate(() => { const r = document.getElementById("b").shadowRoot.querySelector("svg").getBoundingClientRect(); return { x: r.left, y: r.top, w: r.width, h: r.height }; });
  await page.mouse.click(box.x + box.w + 3, box.y + box.h / 2);
  await page.keyboard.type("+x");
  check("click after, then type", await value("b"), "\\frac{1}{2}+x");
  check("label in the page's language", await page.evaluate(() => document.getElementById("b").shadowRoot.querySelector("textarea").getAttribute("aria-label")), "1 sobre 2 más x");

  // Read-only: keys change nothing.
  await page.click("#c");
  await page.keyboard.type("y");
  await page.keyboard.press("Backspace");
  check("readonly", await value("c"), "x^{2}");

  await page.screenshot({ path: join(root, `field-${name.split(" ")[0]}.png`) });
  const bad = checks.filter((c) => !c[3]);
  if (bad.length || errors.length) failed = true;
  console.log(`${bad.length || errors.length ? "FAIL" : "PASS"}  ${name}: ${checks.length - bad.length}/${checks.length} checks, errors=${errors.join(" | ") || "none"}`);
  for (const [what, got, want] of bad) console.log(`   ${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
  await browser.close();
}
server.close();
process.exit(failed ? 1 : 0);
