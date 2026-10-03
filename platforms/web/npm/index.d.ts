export {
  MathEngine,
  mathml,
  speech,
  speechWith,
  asciimathToTex,
  nemeth,
  version,
} from "./wasm/mathwasm.js";
export { speechTree as speechTreeJson } from "./wasm/mathwasm.js";
export { CanvasRenderer } from "./renderer.js";

/**
 * Loads the engine once; later calls return the same promise. Browsers and
 * bundlers fetch the .wasm next to the module, Node reads it from disk. Pass
 * bytes, a URL or a Response to load it from elsewhere.
 */
export function load(wasm?: BufferSource | URL | string | Response | WebAssembly.Module): Promise<void>;

/** How much scaffolding a listener hears. */
export declare const Verbosity: Readonly<{ verbose: 0; brief: 1; superbrief: 2 }>;

/** One part of a formula for a screen reader to step through. */
export interface SpeechNode {
  /** "formula", "fraction", "root", "scripts", "matrix", "symbol", ... */
  role: string;
  /** Place in the parent: "numerator", "superscript", "row 2", or "". */
  label: string;
  text: string;
  /** Byte range of the source, for highlighting the part being read. */
  start: number;
  end: number;
  children: SpeechNode[];
}

/** Languages speech is available in, as BCP 47 tags; any other reads in English. */
export declare const speechLanguages: readonly string[];

/** The formula as a tree a screen reader can walk part by part. `language` is a BCP 47 tag. */
export function speechTree(tex: string, verbosity?: 0 | 1 | 2, macros?: string, language?: string): SpeechNode;

/** The default font for a code point: a Noto face by script (CJK by `language`), or null. */
export function defaultFontSource(codePoint: number, language?: string): string | null;
/** Where fonts for other scripts come from; null turns downloads off. */
export function setFontSource(source: ((codePoint: number, language: string) => string | null) | null): void;
/**
 * Adds fonts to `engine` until every character of `tex` can be drawn (Bengali,
 * Arabic, CJK...). Resolves to true when fonts were added: render again.
 */
export function loadFontsFor(engine: import("./wasm/mathwasm.js").MathEngine, tex: string, displayMode?: boolean, language?: string): Promise<boolean>;
