// Canvas renderer for the mathcore wasm binding.
//
//   import init, { MathEngine } from "../pkg/mathwasm.js";   // wasm-pack --target web
//   await init();
//   const engine = new MathEngine();
//   const renderer = new CanvasRenderer(engine);
//   const layout = engine.render("\\frac{a}{b}", 32, true, 0xff000000);
//   renderer.draw(layout, ctx, 10, 10);
//
// Or skip the canvas entirely: element.innerHTML = engine.renderSvg(tex, 32, true, 0xff000000);

export class CanvasRenderer {
  constructor(engine) {
    this.engine = engine;
    this.upem = engine.unitsPerEm();
    this.paths = new Map();
  }

  /** Path2D for a glyph in font units, y down. */
  glyphPath(id) {
    if (this.paths.has(id)) return this.paths.get(id);
    const cmds = this.engine.glyphOutline(id);
    let path = null;
    if (cmds) {
      path = new Path2D();
      for (let i = 0; i < cmds.length; ) {
        switch (cmds[i]) {
          case 0: path.moveTo(cmds[i + 1], -cmds[i + 2]); i += 3; break;
          case 1: path.lineTo(cmds[i + 1], -cmds[i + 2]); i += 3; break;
          case 2: path.quadraticCurveTo(cmds[i + 1], -cmds[i + 2], cmds[i + 3], -cmds[i + 4]); i += 5; break;
          case 3: path.bezierCurveTo(cmds[i + 1], -cmds[i + 2], cmds[i + 3], -cmds[i + 4], cmds[i + 5], -cmds[i + 6]); i += 7; break;
          default: path.closePath(); i += 1;
        }
      }
    }
    this.paths.set(id, path);
    return path;
  }

  /** Size of a layout returned by engine.render(). */
  static size(layout) {
    return { width: layout[0], ascent: layout[1], descent: layout[2], height: layout[1] + layout[2] };
  }

  /** Draws a layout with its top-left corner at (left, top). */
  draw(layout, ctx, left = 0, top = 0) {
    const count = layout[3];
    const f32 = new Float32Array(1);
    const u32 = new Uint32Array(f32.buffer);
    for (let n = 0, i = 4; n < count; n++, i += 8) {
      f32[0] = layout[i + 7];
      const argb = u32[0];
      ctx.fillStyle = ctx.strokeStyle = `rgba(${(argb >> 16) & 255},${(argb >> 8) & 255},${argb & 255},${((argb >>> 24) & 255) / 255})`;
      switch (layout[i]) {
        case 0: {
          const path = this.glyphPath(layout[i + 1]);
          if (!path) break;
          const k = layout[i + 4] / this.upem;
          ctx.save();
          ctx.translate(left + layout[i + 2], top + layout[i + 3]);
          ctx.scale(k, k);
          ctx.fill(path);
          ctx.restore();
          break;
        }
        case 1:
          ctx.fillRect(left + layout[i + 2], top + layout[i + 3], layout[i + 4], layout[i + 5]);
          break;
        default:
          ctx.lineWidth = layout[i + 6];
          ctx.beginPath();
          ctx.moveTo(left + layout[i + 2], top + layout[i + 3]);
          ctx.lineTo(left + layout[i + 4], top + layout[i + 5]);
          ctx.stroke();
      }
    }
  }
}
