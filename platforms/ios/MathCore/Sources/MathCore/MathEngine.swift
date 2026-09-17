import CoreGraphics
import Foundation
import MathCoreFFI

/// A drawable item of a laid-out formula. Pixels, y down, origin top-left.
public enum MathItem {
    /// `font` is which font of the engine's chain the glyph belongs to; 0 is the primary.
    case glyph(font: UInt16, id: UInt16, x: CGFloat, y: CGFloat, emSize: CGFloat, color: UInt32)
    case rule(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, color: UInt32)
    case line(x1: CGFloat, y1: CGFloat, x2: CGFloat, y2: CGFloat, thickness: CGFloat, color: UInt32)
}

/// Where a piece of the source ended up on screen. Regions nest, so a point
/// usually falls in several and the smallest is the innermost sub-expression.
public struct MathSourceRegion {
    /// Byte range of the source that produced this piece.
    public let start: Int
    public let end: Int
    public let frame: CGRect
    /// Nesting level; 0 is a top-level atom.
    public let depth: Int

    /// Slices the source this region came from.
    public func text(in latex: String) -> String {
        let utf8 = Array(latex.utf8)
        let lo = min(start, utf8.count), hi = min(end, utf8.count)
        return String(decoding: utf8[lo..<max(lo, hi)], as: UTF8.self)
    }
}

/// A laid-out formula. The baseline sits at `ascent` from the top.
public struct MathLayout {
    public let width: CGFloat
    public let ascent: CGFloat
    public let descent: CGFloat
    public let items: [MathItem]
    /// Empty unless the layout was rendered with `hitTesting`.
    public let regions: [MathSourceRegion]
    public var height: CGFloat { ascent + descent }
    public var size: CGSize { CGSize(width: width, height: height) }

    /// Every region containing the point, outermost first.
    public func hit(_ point: CGPoint) -> [MathSourceRegion] {
        regions.filter { $0.frame.contains(point) }
            .sorted { $0.frame.width * $0.frame.height > $1.frame.width * $1.frame.height }
    }

    /// The smallest piece of source under the point.
    public func hitTest(_ point: CGPoint) -> MathSourceRegion? { hit(point).last }

    /// The piece of source under the point, or the closest one when the point
    /// falls in the space between atoms. A tap is never pixel-exact, so this is
    /// what a view should call.
    public func hitNearest(_ point: CGPoint) -> MathSourceRegion? {
        if let exact = hitTest(point) { return exact }
        func distance(_ r: CGRect) -> CGFloat {
            let dx = max(r.minX - point.x, point.x - r.maxX, 0)
            let dy = max(r.minY - point.y, point.y - r.maxY, 0)
            return (dx * dx + dy * dy).squareRoot()
        }
        return regions.min {
            let (a, b) = (distance($0.frame), distance($1.frame))
            return a == b ? $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height : a < b
        }
    }
}

public struct MathParseError: Error, CustomStringConvertible {
    public let message: String
    public var description: String { message }
}

/// A TeX math typesetting engine backed by the native mathcore library.
///
/// One engine per font; `MathEngine.shared` uses the bundled Latin Modern
/// Math. Glyph outlines are cached as `CGPath` in font units and drawn with
/// a translate+scale per glyph, so drawing is one `CGContext.fillPath` each.
public final class MathEngine {
    public static let shared = try! MathEngine()

    private var handle: OpaquePointer
    /// Keyed by font and glyph, since a formula may draw from more than one font.
    private var paths: [UInt32: CGPath?] = [:]
    private var upem: [UInt16: CGFloat] = [:]
    private let lock = NSLock()

    /// Engine with the bundled font.
    public init() throws {
        guard let h = math_engine_new_bundled() else { throw MathEngine.lastError() }
        handle = h
    }

    /// Font units per em of one font of the chain; a fallback may differ.
    public func unitsPerEm(_ font: UInt16 = 0) -> CGFloat {
        if let v = upem[font] { return v }
        let v = CGFloat(math_engine_units_per_em(handle, font))
        upem[font] = v
        return v
    }

    /// Engine for any OpenType font with a MATH table.
    public convenience init(fontData: Data) throws {
        try self.init(mathFont: fontData, textFont: nil)
    }

    /// Engine with a math font and a font for `\text{}`. Pass nil for the math
    /// font to keep the bundled one; a text font sets prose in your own face.
    public init(mathFont: Data?, textFont: Data?) throws {
        func withBytes<R>(_ d: Data?, _ body: (UnsafePointer<UInt8>?, Int) -> R) -> R {
            guard let d else { return body(nil, 0) }
            return d.withUnsafeBytes { body($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        }
        let h = withBytes(mathFont) { mp, ml in
            withBytes(textFont) { tp, tl in
                math_engine_new_with_text_font(mp, ml, tp, tl)
            }
        }
        guard let h else { throw MathEngine.lastError() }
        handle = h
    }

    deinit { math_engine_free(handle) }

    private static func lastError() -> MathParseError {
        MathParseError(message: math_last_error().map { String(cString: $0) } ?? "unknown native error")
    }

    /// Lays out `tex` at `fontSize` points. `color` is 0xAARRGGBB. When
    /// `maxWidth` is given, a wider formula is broken into lines before
    /// relations and binary operators.
    public func render(_ tex: String, fontSize: CGFloat, displayMode: Bool = true,
                       color: UInt32 = 0xFF00_0000, macros: [String: String] = [:],
                       maxWidth: CGFloat? = nil, hitTesting: Bool = false) throws -> MathLayout {
        lock.lock(); defer { lock.unlock() }
        let rgba = ((color & 0x00FF_FFFF) << 8) | ((color >> 24) & 0xFF)
        let macroText: String? = macros.isEmpty ? nil : macros.map { "\($0.key)=\($0.value)" }.joined(separator: "\n")
        let width = Float(maxWidth ?? 0)
        let result: UnsafeMutablePointer<MathResult>? = tex.withCString { texP in
            if let m = macroText {
                return m.withCString {
                    math_engine_render(handle, texP, Float(fontSize), displayMode, rgba, $0, width, hitTesting)
                }
            }
            return math_engine_render(handle, texP, Float(fontSize), displayMode, rgba, nil, width, hitTesting)
        }
        guard let r = result else { throw MathEngine.lastError() }
        defer { math_result_free(r) }
        let res = r.pointee
        var items: [MathItem] = []
        items.reserveCapacity(res.count)
        for i in 0..<res.count {
            let it = res.items[i]
            let argb = ((it.color & 0xFF) << 24) | (it.color >> 8)
            switch it.kind {
            case 0:
                items.append(.glyph(font: it.font, id: it.glyph, x: CGFloat(it.x), y: CGFloat(it.y),
                                    emSize: CGFloat(it.w), color: argb))
            case 1: items.append(.rule(x: CGFloat(it.x), y: CGFloat(it.y), width: CGFloat(it.w), height: CGFloat(it.h), color: argb))
            default: items.append(.line(x1: CGFloat(it.x), y1: CGFloat(it.y), x2: CGFloat(it.w), y2: CGFloat(it.h),
                                       thickness: CGFloat(it.thickness), color: argb))
            }
        }
        var regions: [MathSourceRegion] = []
        regions.reserveCapacity(res.region_count)
        for i in 0..<res.region_count {
            let g = res.regions[i]
            regions.append(MathSourceRegion(
                start: Int(g.start),
                end: Int(g.end),
                frame: CGRect(x: CGFloat(g.x), y: CGFloat(g.y), width: CGFloat(g.width), height: CGFloat(g.height)),
                depth: Int(g.depth)
            ))
        }
        return MathLayout(width: CGFloat(res.width), ascent: CGFloat(res.ascent), descent: CGFloat(res.descent),
                          items: items, regions: regions)
    }

    /// Presentation MathML for a formula, for assistive technology. Needs no engine.
    public static func mathml(_ tex: String, displayMode: Bool = true) throws -> String {
        try string { tex.withCString { math_mathml($0, displayMode, nil) } }
    }

    /// A spoken sentence for a formula, for `accessibilityLabel`. Needs no engine.
    public static func speech(_ tex: String) throws -> String {
        try string { tex.withCString { math_speech($0, nil) } }
    }

    private static func string(_ call: () -> UnsafeMutablePointer<CChar>?) throws -> String {
        guard let p = call() else { throw MathEngine.lastError() }
        defer { math_string_free(p) }
        return String(cString: p)
    }

    /// Glyph outline in font units with y pointing down, cached.
    public func glyphPath(font: UInt16 = 0, glyph: UInt16) -> CGPath? {
        lock.lock(); defer { lock.unlock() }
        let key = UInt32(font) << 16 | UInt32(glyph)
        if let cached = paths[key] { return cached }
        var len = 0
        guard let buf = math_engine_glyph_outline(handle, font, glyph, &len) else { paths[key] = .some(nil); return nil }
        defer { math_buffer_free(buf, len) }
        let path = CGMutablePath()
        var i = 0
        while i < len {
            switch Int(buf[i]) {
            case 0: path.move(to: CGPoint(x: CGFloat(buf[i + 1]), y: -CGFloat(buf[i + 2]))); i += 3
            case 1: path.addLine(to: CGPoint(x: CGFloat(buf[i + 1]), y: -CGFloat(buf[i + 2]))); i += 3
            case 2: path.addQuadCurve(to: CGPoint(x: CGFloat(buf[i + 3]), y: -CGFloat(buf[i + 4])),
                                      control: CGPoint(x: CGFloat(buf[i + 1]), y: -CGFloat(buf[i + 2]))); i += 5
            case 3: path.addCurve(to: CGPoint(x: CGFloat(buf[i + 5]), y: -CGFloat(buf[i + 6])),
                                  control1: CGPoint(x: CGFloat(buf[i + 1]), y: -CGFloat(buf[i + 2])),
                                  control2: CGPoint(x: CGFloat(buf[i + 3]), y: -CGFloat(buf[i + 4]))); i += 7
            default: path.closeSubpath(); i += 1
            }
        }
        paths[key] = path
        return path
    }

    /// Draws `layout` into a y-down context with its top-left corner at `origin`.
    public func draw(_ layout: MathLayout, in ctx: CGContext, at origin: CGPoint = .zero) {
        func setColor(_ argb: UInt32) {
            let a = CGFloat((argb >> 24) & 0xFF) / 255, r = CGFloat((argb >> 16) & 0xFF) / 255
            let g = CGFloat((argb >> 8) & 0xFF) / 255, b = CGFloat(argb & 0xFF) / 255
            ctx.setFillColor(red: r, green: g, blue: b, alpha: a)
            ctx.setStrokeColor(red: r, green: g, blue: b, alpha: a)
        }
        for item in layout.items {
            switch item {
            case let .glyph(font, id, x, y, em, color):
                guard let path = glyphPath(font: font, glyph: id) else { continue }
                setColor(color)
                ctx.saveGState()
                ctx.translateBy(x: origin.x + x, y: origin.y + y)
                let k = em / unitsPerEm(font)
                ctx.scaleBy(x: k, y: k)
                ctx.addPath(path)
                ctx.fillPath()
                ctx.restoreGState()
            case let .rule(x, y, w, h, color):
                setColor(color)
                ctx.fill(CGRect(x: origin.x + x, y: origin.y + y, width: w, height: h))
            case let .line(x1, y1, x2, y2, t, color):
                setColor(color)
                ctx.setLineWidth(t)
                ctx.move(to: CGPoint(x: origin.x + x1, y: origin.y + y1))
                ctx.addLine(to: CGPoint(x: origin.x + x2, y: origin.y + y2))
                ctx.strokePath()
            }
        }
    }
}
