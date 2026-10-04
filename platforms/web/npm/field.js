// <math-field>: an editable formula.
//
//   <script type="module">import "mathcore/field";</script>
//   <math-field value="x^2"></math-field>
//
// Type as you would write: `/` makes a fraction of the term before it, `^`
// and `_` open a script, `(` opens a pair, `\alpha` or just `sqrt`, `pi`,
// `sin` become what they name. Arrows walk into and out of structures.
//
// `field.value` is the TeX; `input` fires on every change and `change` when
// the field loses focus after one. Attributes: `value`, `size` (px; the
// surrounding font size by default), `readonly`, `placeholder`. A screen
// reader hears the formula as the field's label and, after each key, where
// the cursor is, in the page's language (the nearest `lang`).
import { load, MathEngine, MathEditor, loadFontsFor } from "./index.js";

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

const STYLE = `
:host{display:inline-block;min-width:2em;padding:4px 6px;border:1px solid color-mix(in srgb,currentColor 35%,transparent);
  border-radius:4px;cursor:text;vertical-align:middle;position:relative;line-height:0}
:host(:focus-within){outline:2px solid Highlight;outline-offset:1px}
.box{position:relative;display:inline-block}
svg{display:block;overflow:visible}
.caret{position:absolute;background:currentColor;display:none;pointer-events:none}
:host(:focus-within) .caret{display:block;animation:blink 1.06s steps(1) infinite}
:host([readonly]) .caret{display:none!important}
.sel{position:absolute;background:Highlight;opacity:.35;pointer-events:none}
.hint{position:absolute;left:0;top:50%;transform:translateY(-50%);opacity:.45;white-space:nowrap;line-height:normal;pointer-events:none}
textarea{position:absolute;left:0;top:0;width:1px;height:1px;opacity:0;border:0;padding:0;margin:0;resize:none;
  font-size:16px;overflow:hidden;white-space:pre}
.live{position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap}
@keyframes blink{50%{opacity:0}}
@media (prefers-reduced-motion:reduce){:host(:focus-within) .caret{animation:none}}`;

export class MathFieldElement extends HTMLElement {
  static observedAttributes = ["value", "size", "readonly", "placeholder"];

  constructor() {
    super();
    this._root = this.attachShadow({ mode: "open", delegatesFocus: true });
    this._root.innerHTML =
      `<style>${STYLE}</style><span class="box"><span class="svg"></span><span class="sels"></span>` +
      `<span class="caret"></span><span class="hint"></span></span>` +
      `<textarea autocapitalize="off" autocomplete="off" autocorrect="off" spellcheck="false" ` +
      `aria-roledescription="math input"></textarea><span class="live" aria-live="polite"></span>`;
    const q = (s) => this._root.querySelector(s);
    this._svg = q(".svg");
    this._sels = q(".sels");
    this._caret = q(".caret");
    this._hint = q(".hint");
    this._input = q("textarea");
    this._live = q(".live");
    this._editor = null;
    this._pending = this.getAttribute("value") ?? "";
    this._changed = false;
    this._composing = false;

    this._input.addEventListener("compositionstart", () => (this._composing = true));
    this._input.addEventListener("compositionend", () => {
      this._composing = false;
      this._take();
    });
    this._input.addEventListener("input", () => {
      if (!this._composing) this._take();
    });
    this._input.addEventListener("keydown", (e) => this._key(e));
    this._input.addEventListener("copy", (e) => this._copy(e, false));
    this._input.addEventListener("cut", (e) => this._copy(e, true));
    this._input.addEventListener("paste", (e) => {
      e.preventDefault();
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (text && this._editable()) this._edit(() => this._editor.insertTex(text));
    });
    this._input.addEventListener("blur", () => {
      if (this._changed) {
        this._changed = false;
        this.dispatchEvent(new Event("change", { bubbles: true }));
      }
    });
    this.addEventListener("pointerdown", (e) => {
      if (!this._editor) return;
      const r = this._svg.getBoundingClientRect();
      this._editor.tap(e.clientX - r.left, e.clientY - r.top);
      this._draw(true);
      // Keep focus in the hidden input (after the browser's own handling).
      e.preventDefault();
      this._input.focus({ preventScroll: true });
    });
  }

  connectedCallback() {
    ready.then(() => {
      if (!this._editor) {
        this._editor = new MathEditor();
        if (this._pending) this._editor.setTex(this._pending);
      }
      this._draw(false);
    });
  }

  attributeChangedCallback(name, _old, value) {
    if (name === "value" && value !== null && value !== this.value) this.value = value;
    else if (this._editor) this._draw(false);
  }

  /** The formula as TeX. */
  get value() {
    return this._editor ? this._editor.tex() : this._pending;
  }

  set value(tex) {
    this._pending = String(tex ?? "");
    if (this._editor) {
      this._editor.setTex(this._pending);
      this._draw(false);
    }
  }

  /** The formula read aloud, in the field's language. */
  get speech() {
    return this._editor ? this._editor.speech(this._language()) : "";
  }

  /** Runs an editor command (`frac`, `sqrt`, `nthroot`, `alpha`...), for toolbar buttons. */
  command(name) {
    if (this._editable()) this._edit(() => this._editor.command(name));
    this._input.focus({ preventScroll: true });
  }

  /** Types text as the keyboard would, for on-screen keys. */
  type(text) {
    if (this._editable()) this._edit(() => this._editor.typeText(text));
  }

  _editable() {
    return this._editor && !this.hasAttribute("readonly");
  }

  _language() {
    return this.closest("[lang]")?.getAttribute("lang") || navigator.language || "en";
  }

  _take() {
    const text = this._input.value;
    this._input.value = "";
    if (text && this._editable()) this._edit(() => this._editor.typeText(text));
  }

  _key(e) {
    if (!this._editor || this._composing) return;
    const command = e.ctrlKey || e.metaKey;
    const editing = ["Backspace", "Delete", "z", "Z", "y", "Y"].includes(e.key);
    if (editing && !this._editable()) {
      // Swallowed: Backspace in a read-only field must not leave the page.
      if (!command) e.preventDefault();
      return;
    }
    // Tab leaves the field unless there is a slot to leave first.
    if (e.key === "Tab") return;
    const before = this._editor.tex();
    if (this._editor.key(e.key, e.shiftKey, command)) {
      e.preventDefault();
      this._draw(true);
      if (this._editor.tex() !== before) this._notify();
    }
  }

  _copy(e, cut) {
    const tex = this._editor?.selectedTex() || this.value;
    e.clipboardData?.setData("text/plain", tex);
    e.preventDefault();
    if (cut && this._editable() && this._editor.selectedTex()) this._edit(() => this._editor.key("Backspace", false, false));
  }

  _edit(change) {
    change();
    this._draw(true);
    this._notify();
  }

  _notify() {
    this._changed = true;
    this.dispatchEvent(new Event("input", { bubbles: true }));
  }

  _draw(announce) {
    if (!engine || !this._editor) return;
    const style = getComputedStyle(this);
    const size = Number(this.getAttribute("size")) || parseFloat(style.fontSize) || 16;
    const language = this._language();
    let svg;
    try {
      svg = this._editor.renderSvg(engine, size, true, argb(style.color));
    } catch (err) {
      this.setAttribute("data-error", String(err.message ?? err));
      return;
    }
    this.removeAttribute("data-error");
    this._svg.innerHTML = svg;
    const [x, y, w, h] = this._editor.caret();
    Object.assign(this._caret.style, { left: `${x}px`, top: `${y}px`, width: `${Math.max(w, 1)}px`, height: `${h}px` });
    // Restart the blink so the caret is solid right after a key.
    this._caret.style.animation = "none";
    void this._caret.offsetWidth;
    this._caret.style.animation = "";
    const sel = this._editor.selection();
    let boxes = "";
    for (let i = 0; i < sel.length; i += 4) {
      boxes += `<span class="sel" style="left:${sel[i]}px;top:${sel[i + 1]}px;width:${sel[i + 2]}px;height:${sel[i + 3]}px"></span>`;
    }
    this._sels.innerHTML = boxes;
    const empty = this._editor.tex() === "";
    this._hint.textContent = empty ? (this.getAttribute("placeholder") ?? "") : "";
    this._hint.style.fontSize = `${size * 0.8}px`;
    this._input.readOnly = this.hasAttribute("readonly");
    this._input.setAttribute("aria-label", this._editor.speech(language) || this.getAttribute("placeholder") || "math");
    if (announce) this._live.textContent = this._editor.describe(language);
    // Text in another script needs its font: fetch it, then draw again.
    const tex = this._editor.tex();
    if (/[^\x00-\x7f]/.test(tex) && this._fontsFor !== tex) {
      this._fontsFor = tex;
      loadFontsFor(engine, tex, true, language).then((added) => added && this._draw(false));
    }
  }
}

if (typeof customElements !== "undefined" && !customElements.get("math-field")) {
  customElements.define("math-field", MathFieldElement);
}
