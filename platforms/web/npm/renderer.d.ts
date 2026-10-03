import type { MathEngine } from "./wasm/mathwasm.js";

/** Draws `MathEngine.render` layouts on a 2D canvas, caching glyph outlines as Path2D. */
export declare class CanvasRenderer {
  constructor(engine: MathEngine);
  static size(layout: Float32Array | number[]): { width: number; ascent: number; descent: number; height: number };
  draw(layout: Float32Array | number[], ctx: CanvasRenderingContext2D, left?: number, top?: number): void;
}
