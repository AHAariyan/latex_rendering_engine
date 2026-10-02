//! Web SDK: the `mathcore` npm package. One ES module that works in browsers,
//! bundlers and Node, the `<math-tex>` element, a canvas renderer, and types.

use crate::util::*;

pub const NPM_NAME: &str = "mathcore";

pub fn build(verify: bool) -> Result {
    require("wasm-pack", "cargo install wasm-pack")?;
    require("npm", "install Node.js 18 or later")?;
    let version = version();
    let out = dist().join("web");
    reset_dir(&out)?;
    let pkg = out.join("package");

    step("Web: WebAssembly build");
    rustup_targets(&["wasm32-unknown-unknown"])?;
    run(cmd("wasm-pack")
        .args([
            "build",
            "--release",
            "--target",
            "web",
            "--no-pack",
            "--out-name",
            "mathwasm",
            "--out-dir",
        ])
        .arg(pkg.join("wasm"))
        .current_dir(root().join("crates/mathwasm")))?;
    let _ = std::fs::remove_file(pkg.join("wasm/.gitignore"));
    // wasm-bindgen declares `[Symbol.dispose]()`, which only type-checks with
    // TypeScript's esnext.disposable lib. The method still exists at runtime;
    // dropping its declaration keeps the package usable under common configs.
    let dts = pkg.join("wasm/mathwasm.d.ts");
    let text = std::fs::read_to_string(&dts).map_err(|e| e.to_string())?;
    let kept: Vec<&str> = text.lines().filter(|l| !l.contains("Symbol.dispose")).collect();
    write(&dts, &(kept.join("\n") + "\n"))?;

    step("Web: package");
    let src = root().join("platforms/web");
    for f in ["index.js", "index.d.ts", "element.js", "element.d.ts", "renderer.d.ts"] {
        copy(&src.join("npm").join(f), &pkg.join(f))?;
    }
    copy(&src.join("mathcore.js"), &pkg.join("renderer.js"))?;
    copy(&root().join("LICENSE"), &pkg.join("LICENSE"))?;
    write(&pkg.join("README.md"), &readme())?;
    write(&pkg.join("package.json"), &package_json(&version))?;
    let tarball = output(
        cmd("npm")
            .args(["pack", "--silent", "--pack-destination"])
            .arg(&out)
            .current_dir(&pkg),
    )?;
    let tarball = out.join(tarball.lines().last().unwrap_or_default());

    if verify {
        step("Web: install the tarball and use it from Node");
        let app = root().join("target/sdk-web-check");
        reset_dir(&app)?;
        write(&app.join("package.json"), r#"{"name":"check","private":true,"type":"module"}"#)?;
        run(cmd("npm")
            .args(["install", "--silent", "--no-audit", "--no-fund"])
            .arg(&tarball)
            .arg("typescript@5")
            .current_dir(&app))?;
        write(&app.join("check.mjs"), CHECK)?;
        run(cmd("node").arg("check.mjs").current_dir(&app))?;
        step("Web: the type declarations compile");
        write(&app.join("check.ts"), CHECK_TS)?;
        run(cmd("npx")
            .args([
                "tsc",
                "--noEmit",
                "--strict",
                "--module",
                "nodenext",
                "--moduleResolution",
                "nodenext",
                "--target",
                "es2022",
            ])
            .args(["--lib", "es2022,dom", "--skipLibCheck", "false", "check.ts"])
            .current_dir(&app))?;
    }
    eprintln!("\nnpm package {NPM_NAME}@{version}: {}", tarball.display());
    Ok(())
}

fn package_json(version: &str) -> String {
    format!(
        r#"{{
  "name": "{NPM_NAME}",
  "version": "{version}",
  "description": "Native-quality TeX math typesetting in WebAssembly: TeX, AsciiMath and chemistry to SVG or canvas, with screen-reader speech. No fonts to host, no CSS.",
  "keywords": ["math", "tex", "latex", "katex", "mathjax", "asciimath", "mhchem", "svg", "wasm", "accessibility"],
  "license": "MIT",
  "repository": {{ "type": "git", "url": "git+https://github.com/AHAariyan/latex_rendering_engine.git", "directory": "platforms/web" }},
  "type": "module",
  "main": "./index.js",
  "types": "./index.d.ts",
  "exports": {{
    ".": {{ "types": "./index.d.ts", "default": "./index.js" }},
    "./element": {{ "types": "./element.d.ts", "default": "./element.js" }},
    "./renderer": {{ "types": "./renderer.d.ts", "default": "./renderer.js" }},
    "./mathwasm_bg.wasm": "./wasm/mathwasm_bg.wasm",
    "./package.json": "./package.json"
  }},
  "files": ["index.js", "index.d.ts", "element.js", "element.d.ts", "renderer.js", "renderer.d.ts", "wasm/", "README.md", "LICENSE"],
  "sideEffects": ["./element.js"],
  "engines": {{ "node": ">=18" }}
}}
"#
    )
}

fn readme() -> String {
    r#"# mathcore

TeX math typeset by a native engine compiled to WebAssembly: the TeXbook's
layout rules with every dimension from the OpenType MATH table, the approach
of XeTeX, LuaTeX and Microsoft Word. Fonts are inside the module, so there is
nothing to host and no CSS to load.

```sh
npm install mathcore
```

## The element

```html
<script type="module">import "mathcore/element";</script>
The roots of <math-tex>ax^2+bx+c</math-tex> are
<math-tex display>x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}</math-tex>
<math-tex asciimath>sum_(i=1)^n i^2</math-tex>
<math-tex>\ce{2H2 + O2 -> 2H2O}</math-tex>
```

Inline formulas sit on the text baseline and take its size and colour;
`display` formulas break to the container's width. Every element carries a
spoken `aria-label`.

## The API

```js
import { load, MathEngine, speech, speechTree, asciimathToTex } from "mathcore";
await load();
const engine = new MathEngine();
el.innerHTML = engine.renderSvg("\\int_0^1 x^2\\,dx", 32, true, 0xff000000);
speech("x^2 + y^2 = z^2");        // "x squared plus y squared equals z squared"
speechTree("\\frac{a+b}{c}");     // parts with source ranges, for exploring
asciimathToTex("sqrt(x)/2");      // "\\frac{\\sqrt{x}}{2}"
```

`engine.render(...)` returns a flat layout for `CanvasRenderer` (from
`mathcore/renderer`) with hit testing: a tap maps back to the source.

Supports KaTeX's command set and more: text mode with `$...$`, sizes,
`\tag`, mhchem `\ce` and `\pu`, AsciiMath, line breaking, MathML output.
"#
    .to_string()
}

const CHECK: &str = r#"import { load, MathEngine, speech, speechWith, speechTree, asciimathToTex, mathml, Verbosity, CanvasRenderer, version } from "mathcore";
import assert from "node:assert/strict";
await load();
await load(); // idempotent
const engine = new MathEngine();
const svg = engine.renderSvg("x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}", 32, true, 0xff000000, null, 0);
assert.match(svg, /^<svg/);
const layout = engine.render("\\ce{2H2 + O2 -> 2H2O}", 20, true, 0xff000000, null, 0, true);
assert.ok(CanvasRenderer.size(layout).width > 50);
assert.equal(speech("x^2 + y^2 = z^2"), "x squared plus y squared equals z squared");
assert.equal(speechWith("\\frac{1}{2}", Verbosity.verbose, null), "the fraction 1 over 2, end fraction");
assert.equal(speechTree("\\frac{a+b}{c}").children[0].label, "numerator");
assert.equal(asciimathToTex("x/y"), "\\frac{x}{y}");
assert.match(mathml("x", true), /<math/);
assert.throws(() => engine.renderSvg("\\nosuch", 20, true, 0, null, 0), /unknown command/);
console.log(`mathcore ${version()} OK from the packed tarball`);
"#;

const CHECK_TS: &str = r#"import { load, MathEngine, speechTree, type SpeechNode, Verbosity } from "mathcore";
import { CanvasRenderer } from "mathcore/renderer";
import type { MathTexElement } from "mathcore/element";
await load();
const engine: MathEngine = new MathEngine();
const svg: string = engine.renderSvg("x", 16, false, 0xff000000, null, 0);
const tree: SpeechNode = speechTree("x", Verbosity.brief);
const size = CanvasRenderer.size(engine.render("x", 16, true, 0xff000000, null, 0, false));
const el: MathTexElement | null = null;
export { svg, tree, size, el };
"#;
