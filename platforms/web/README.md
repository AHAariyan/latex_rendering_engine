# mathcore for the web

The same engine compiled to WebAssembly with `wasm-bindgen`.

```sh
cargo install wasm-pack
cd crates/mathwasm && wasm-pack build --release --target web --out-dir ../../platforms/web/pkg
cd ../../platforms/web && python3 -m http.server   # open http://localhost:8000
```

```js
import init, { MathEngine } from "./pkg/mathwasm.js";
await init();
const engine = new MathEngine();
el.innerHTML = engine.renderSvg("\\frac{a}{b}", 32, true, 0xff000000);   // SVG string
const layout = engine.render("\\frac{a}{b}", 32, true, 0xff000000);       // Float32Array for canvas
new CanvasRenderer(engine).draw(layout, ctx, 0, 0);                        // see mathcore.js
```

`node test.mjs` runs the smoke test against a `--target nodejs` build in
`pkg-node/`. The package is about 1 MB, of which the embedded font is 730 KB.
