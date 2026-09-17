import CoreGraphics
import Foundation
import MathCoreFFI

/// A drawable item of a laid-out formula. Pixels, y down, origin top-left.
public enum MathItem {
    case glyph(id: UInt16, x: CGFloat, y: CGFloat, emSize: CGFloat, color: UInt32)
    case rule(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat, color: UInt32)
    case line(x1: CGFloat, y1: CGFloat, x2: CGFloat, y2: CGFloat, thickness: CGFloat, color: UInt32)
}

/// A laid-out formula. The baseline sits at `ascent` from the top.
public struct MathLayout {
    public let width: CGFloat
    public let ascent: CGFloat
    public let descent: CGFloat
    public let items: [MathItem]
    public var height: CGFloat { ascent + descent }
    public var size: CGSize { CGSize(width: width, height: height) }
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
    public let unitsPerEm: CGFloat
    private var paths: [UInt16: CGPath?] = [:]
    private let lock = NSLock()

    /// Engine with the bundled font.
    public init() throws {
        guard let h = math_engine_new_bundled() else { throw MathEngine.lastError() }
        handle = h
        unitsPerEm = CGFloat(math_engine_units_per_em(h))
    }

    /// Engine for any OpenType font with a MATH table.
    public init(fontData: Data) throws {
        let h = fontData.withUnsafeBytes { buf -> OpaquePointer? in
            math_engine_new(buf.bindMemory(to: UInt8.self).baseAddress, buf.count)
        }
        guard let h else { throw MathEngine.lastError() }
        handle = h
        unitsPerEm = CGFloat(math_engine_units_per_em(h))
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
                       maxWidth: CGFloat? = nil) throws -> MathLayout {
        lock.lock(); defer { lock.unlock() }
        let rgba = ((color & 0x00FF_FFFF) << 8) | ((color >> 24) & 0xFF)
        let macroText: String? = macros.isEmpty ? nil : macros.map { "\($0.key)=\($0.value)" }.joined(separator: "\n")
        let width = Float(maxWidth ?? 0)
        let result: UnsafeMutablePointer<MathResult>? = tex.withCString { texP in
            if let m = macroText {
                return m.withCString { math_engine_render(handle, texP, Float(fontSize), displayMode, rgba, $0, width) }
            }
            return math_engine_render(handle, texP, Float(fontSize), displayMode, rgba, nil, width)
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
            case 0: items.append(.glyph(id: it.glyph, x: CGFloat(it.x), y: CGFloat(it.y), emSize: CGFloat(it.w), color: argb))
            case 1: items.append(.rule(x: CGFloat(it.x), y: CGFloat(it.y), width: CGFloat(it.w), height: CGFloat(it.h), color: argb))
            default: items.append(.line(x1: CGFloat(it.x), y1: CGFloat(it.y), x2: CGFloat(it.w), y2: CGFloat(it.h),
                                       thickness: CGFloat(it.thickness), color: argb))
            }
        }
        return MathLayout(width: CGFloat(res.width), ascent: CGFloat(res.ascent), descent: CGFloat(res.descent), items: items)
    }

    /// Glyph outline in font units with y pointing down, cached.
    public func glyphPath(_ glyph: UInt16) -> CGPath? {
        lock.lock(); defer { lock.unlock() }
        if let cached = paths[glyph] { return cached }
        var len = 0
        guard let buf = math_engine_glyph_outline(handle, glyph, &len) else { paths[glyph] = .some(nil); return nil }
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
        paths[glyph] = path
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
            case let .glyph(id, x, y, em, color):
                guard let path = glyphPath(id) else { continue }
                setColor(color)
                ctx.saveGState()
                ctx.translateBy(x: origin.x + x, y: origin.y + y)
                let k = em / unitsPerEm
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
