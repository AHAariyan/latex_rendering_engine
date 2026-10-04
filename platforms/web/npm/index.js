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
  MathEditor,
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

/** Languages speech is available in, as BCP 47 tags; any other reads in English. */
export const speechLanguages = Object.freeze(["en", "es", "fr", "de", "pt", "it", "nl", "sv", "pl", "ru", "uk", "el", "tr", "ar", "he", "fa", "ur", "hi", "bn", "mr", "gu", "pa", "ta", "te", "kn", "ml", "zh-Hans", "zh-Hant", "ja", "ko", "vi", "th", "id", "ms", "sw"]);

// ---- Fonts for other scripts ----
//
// The engine ships a math font with Latin and Greek letters. Prose in any
// other script (\text{বাংলা}, \text{مرحبا}, \text{你好}) needs a font that
// has it. `loadFontsFor` asks the engine which characters are missing and
// adds a Noto font for each script from a CDN; `setFontSource` points that at
// your own server (or returns null to turn downloads off), and
// `engine.addFont(bytes)` adds a font you already have.

const NOTO = "https://cdn.jsdelivr.net/gh/notofonts/notofonts.github.io/fonts/";
const CJK = "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@main/Sans/SubsetOTF/";
const noto = (family) => `${NOTO}${family}/hinted/ttf/${family}-Regular.ttf`;

// [first, last, Noto family] by Unicode block.
const SCRIPTS = [
  [0x00c0, 0x052f, "NotoSans"], [0x1e00, 0x1fff, "NotoSans"],
  [0x0530, 0x058f, "NotoSansArmenian"], [0x0590, 0x05ff, "NotoSansHebrew"],
  [0x0600, 0x06ff, "NotoNaskhArabic"], [0x0750, 0x077f, "NotoNaskhArabic"], [0x08a0, 0x08ff, "NotoNaskhArabic"],
  [0xfb50, 0xfdff, "NotoNaskhArabic"], [0xfe70, 0xfeff, "NotoNaskhArabic"],
  [0x0700, 0x074f, "NotoSansSyriac"], [0x0780, 0x07bf, "NotoSansThaana"],
  [0x0900, 0x097f, "NotoSansDevanagari"], [0x0980, 0x09ff, "NotoSansBengali"], [0x0a00, 0x0a7f, "NotoSansGurmukhi"],
  [0x0a80, 0x0aff, "NotoSansGujarati"], [0x0b00, 0x0b7f, "NotoSansOriya"], [0x0b80, 0x0bff, "NotoSansTamil"],
  [0x0c00, 0x0c7f, "NotoSansTelugu"], [0x0c80, 0x0cff, "NotoSansKannada"], [0x0d00, 0x0d7f, "NotoSansMalayalam"],
  [0x0d80, 0x0dff, "NotoSansSinhala"], [0x0e00, 0x0e7f, "NotoSansThai"], [0x0e80, 0x0eff, "NotoSansLao"],
  [0x0f00, 0x0fff, "NotoSerifTibetan"], [0x1000, 0x109f, "NotoSansMyanmar"], [0x10a0, 0x10ff, "NotoSansGeorgian"],
  [0x1200, 0x139f, "NotoSansEthiopic"], [0x1780, 0x17ff, "NotoSansKhmer"],
];

/** The default font for a code point: a Noto face by script; CJK by the page's language. */
export function defaultFontSource(codePoint, language = "") {
  const lang = language.toLowerCase();
  const cjk = (region) => `${CJK}${region}/NotoSans${region}-Regular.otf`;
  if ((codePoint >= 0x1100 && codePoint <= 0x11ff) || (codePoint >= 0x3130 && codePoint <= 0x318f) || (codePoint >= 0xac00 && codePoint <= 0xd7af)) return cjk("KR");
  if (codePoint >= 0x3040 && codePoint <= 0x30ff) return cjk("JP");
  if ((codePoint >= 0x2e80 && codePoint <= 0x9fff) || (codePoint >= 0xf900 && codePoint <= 0xfaff) || (codePoint >= 0xff00 && codePoint <= 0xffef) || codePoint >= 0x20000) {
    if (lang.startsWith("ja")) return cjk("JP");
    if (lang.startsWith("ko")) return cjk("KR");
    if (/^zh-(hant|tw|hk|mo)/.test(lang)) return cjk("TC");
    return cjk("SC");
  }
  const hit = SCRIPTS.find(([a, b]) => codePoint >= a && codePoint <= b);
  return hit ? noto(hit[2]) : null;
}

let fontSource = defaultFontSource;

/** Where fonts for other scripts come from: `(codePoint, language) => url | null`. */
export function setFontSource(source) {
  fontSource = source ?? (() => null);
}

const fontBytes = new Map(); // url -> Promise<Uint8Array | null>, shared by all engines
const engineFonts = new WeakMap(); // engine -> Set of urls added

/**
 * Adds fonts to `engine` until every character of `tex` can be drawn or no
 * source has the rest. Resolves to true when it added any (render again).
 */
export async function loadFontsFor(engine, tex, displayMode = true, language = "") {
  let added = false;
  let tried = engineFonts.get(engine);
  if (!tried) engineFonts.set(engine, (tried = new Set()));
  for (let round = 0; round < 12; round++) {
    let missing;
    try {
      missing = engine.missingChars(tex, displayMode);
    } catch {
      return added;
    }
    const urls = [...new Set([...missing].map((c) => fontSource(c.codePointAt(0), language)).filter((u) => u && !tried.has(u)))];
    if (urls.length === 0) return added;
    for (const url of urls) {
      tried.add(url);
      if (!fontBytes.has(url)) {
        fontBytes.set(url, fetch(url).then((r) => (r.ok ? r.arrayBuffer() : null)).then((b) => b && new Uint8Array(b)).catch(() => null));
      }
      const bytes = await fontBytes.get(url);
      if (bytes) {
        try {
          engine.addFont(bytes);
          added = true;
        } catch {
          // Not a font the engine can read; leave the characters as boxes.
        }
      }
    }
  }
  return added;
}

/** The formula as a tree a screen reader can walk; see index.d.ts. */
export function speechTree(tex, verbosity = Verbosity.brief, macros, language) {
  return JSON.parse(speechTreeJson(tex, verbosity, macros, language));
}
