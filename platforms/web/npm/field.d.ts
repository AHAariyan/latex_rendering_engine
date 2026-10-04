/** `<math-field>`: an editable formula. `input` fires on every change, `change` on blur after one. */
export declare class MathFieldElement extends HTMLElement {
  /** The formula as TeX. */
  value: string;
  /** The formula read aloud, in the field's language. */
  readonly speech: string;
  /** Runs an editor command (`frac`, `sqrt`, `nthroot`, `alpha`...), for toolbar buttons. */
  command(name: string): void;
  /** Types text as the keyboard would, for on-screen keys. */
  type(text: string): void;
}

declare global {
  interface HTMLElementTagNameMap {
    "math-field": MathFieldElement;
  }
}
