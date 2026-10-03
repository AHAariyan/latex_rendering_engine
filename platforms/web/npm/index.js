// mathcore for the web: native-quality TeX math from WebAssembly.
//
//   import { load, MathEngine } from "mathcore";
//   await load();
//   el.innerHTML = new MathEngine().renderSvg("\\frac{a}{b}", 32, true, 0xff000000);
//
// Or use the element: import "mathcore/element" and write <math-tex>x^2</math-tex>.
import init from "./wasm/mathwasm.js";

export {
  MathEngine,
  mathml,
  speech,
  speechWith,
  speechTree as speechTreeJson,
  asciimathToTex,
  nemeth,
  version,
} from "./wasm/mathwasm.js";
export { CanvasRenderer } from "./renderer.js";
import { speechTree as speechTreeJson } from "./wasm/mathwasm.js";

let ready = null;

/**
 * Loads the engine once; later calls return the same promise. In a browser or
 * a bundler the .wasm file next to this module is fetched; in Node it is read
 * from disk. Pass bytes, a URL or a Response to load it from elsewhere.
 */
export function load(wasm) {
  if (!ready) {
    ready = (async () => {
      let source = wasm;
      if (source === undefined && typeof process !== "undefined" && process.versions?.node) {
        const { readFile } = await import("node:fs/promises");
        source = await readFile(new URL("./wasm/mathwasm_bg.wasm", import.meta.url));
      }
      await init(source === undefined ? undefined : { module_or_path: source });
    })();
  }
  return ready;
}

/** Verbosity levels for speech: how much scaffolding a listener hears. */
export const Verbosity = Object.freeze({ verbose: 0, brief: 1, superbrief: 2 });

/** The formula as a tree a screen reader can walk; see index.d.ts. */
export function speechTree(tex, verbosity = Verbosity.brief, macros) {
  return JSON.parse(speechTreeJson(tex, verbosity, macros));
}
