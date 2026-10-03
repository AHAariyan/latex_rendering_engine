// <math-tex>: a custom element that typesets its text content.
//
//   <script type="module">import "mathcore/element";</script>
//   The roots of <math-tex>ax^2+bx+c</math-tex> are
//   <math-tex display>x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}</math-tex>
//
// Attributes: `display` (block, breaks to the container width), `asciimath`
// (the content is AsciiMath), `size` (px; defaults to the surrounding font
// size), `verbosity` (verbose | brief | superbrief). The element inherits the
// text colour and reads the formula to screen readers in the page's language
// (the nearest `lang` attribute): en, es, fr, de, pt, bn or hi.
import { load, MathEngine, asciimathToTex, speechWith, Verbosity } from "./index.js";

let engine = null;
const ready = load().then(() => {
  engine = new MathEngine();
});

function argb(cssColor) {
  const m = cssColor.match(/rgba?\(([^)]+)\)/);
  if (!m) return 0xff000000;
  const [r, g, b, a = 1] = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
  return ((Math.round(a * 255) << 24) | (r << 16) | (g << 8) | b) >>> 0;
}

export class MathTexElement extends HTMLElement {
  static observedAttributes = ["display", "asciimath", "size", "verbosity"];

  constructor() {
    super();
    this._root = this.attachShadow({ mode: "open" });
    this._source = null;
    this._width = 0;
  }

  connectedCallback() {
    if (this._source === null) this._source = this.textContent;
    this.setAttribute("role", "math");
    this._observer = new MutationObserver(() => {
      this._source = this.textContent;
      this.render();
    });
    this._observer.observe(this, { characterData: true, childList: true, subtree: true });
    if (this.hasAttribute("display")) {
      this._resize = new ResizeObserver(([e]) => {
        const w = Math.floor(e.contentRect.width);
        if (w !== this._width) {
          this._width = w;
          this.render();
        }
      });
      this._resize.observe(this);
    }
    ready.then(() => this.render());
  }

  disconnectedCallback() {
    this._observer?.disconnect();
    this._resize?.disconnect();
  }

  attributeChangedCallback() {
    if (this.isConnected) this.render();
  }

  /** The TeX this element shows, after AsciiMath translation. */
  get tex() {
    const src = (this._source ?? this.textContent).trim();
    return this.hasAttribute("asciimath") ? asciimathToTex(src) : src;
  }

  render() {
    if (!engine) return;
    const style = getComputedStyle(this);
    const size = Number(this.getAttribute("size")) || parseFloat(style.fontSize) || 16;
    const display = this.hasAttribute("display");
    try {
      const tex = this.tex;
      const width = display ? this._width || this.clientWidth : 0;
      const svg = engine.renderSvg(tex, size, display, argb(style.color), null, width);
      const metrics = engine.render(tex, size, display, argb(style.color), null, width);
      const descent = metrics[2];
      this._root.innerHTML =
        `<style>:host{display:${display ? "block" : "inline-block"};${display ? "text-align:center;margin:1em 0" : ""}}` +
        `svg{display:inline-block;vertical-align:${display ? "top" : `${-descent}px`};overflow:visible}</style>${svg}`;
      const level = Verbosity[this.getAttribute("verbosity")] ?? Verbosity.brief;
      // The element's language, as the page declares it, else the browser's.
      const language = this.closest("[lang]")?.getAttribute("lang") || navigator.language || "en";
      this.setAttribute("aria-label", speechWith(tex, level, null, language));
      this.removeAttribute("data-error");
    } catch (e) {
      this._root.innerHTML = `<span part="error" style="color:#b00020;font:0.85em monospace">${String(e.message ?? e)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")}</span>`;
      this.setAttribute("data-error", String(e.message ?? e));
    }
  }
}

if (typeof customElements !== "undefined" && !customElements.get("math-tex")) {
  customElements.define("math-tex", MathTexElement);
}
