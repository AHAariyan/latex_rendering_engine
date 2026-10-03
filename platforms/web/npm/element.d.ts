/** `<math-tex>`: typesets its text content. Importing this module registers it. */
export declare class MathTexElement extends HTMLElement {
  /** The TeX shown, after AsciiMath translation. */
  readonly tex: string;
  render(): void;
}
declare global {
  interface HTMLElementTagNameMap {
    "math-tex": MathTexElement;
  }
}
