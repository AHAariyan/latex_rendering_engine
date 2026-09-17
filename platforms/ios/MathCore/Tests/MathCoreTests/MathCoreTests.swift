import XCTest
@testable import MathCore

final class MathCoreTests: XCTestCase {
    func testRendersFraction() throws {
        let layout = try MathEngine.shared.render(#"\frac{a}{b} + x^2"#, fontSize: 32)
        XCTAssertGreaterThan(layout.width, 0)
        var rules = 0, glyphs = 0
        for item in layout.items {
            if case .rule = item { rules += 1 }
            if case .glyph = item { glyphs += 1 }
        }
        XCTAssertEqual(rules, 1)
        XCTAssertEqual(glyphs, 5)
    }

    func testParseErrorThrows() {
        XCTAssertThrowsError(try MathEngine.shared.render(#"\frac{a"#, fontSize: 20)) { error in
            XCTAssertTrue("\(error)".contains("parse error"))
        }
    }

    func testGlyphPathIsCached() throws {
        let layout = try MathEngine.shared.render("x", fontSize: 32)
        guard case let .glyph(id, _, _, _, _) = layout.items[0] else { return XCTFail("glyph expected") }
        let a = MathEngine.shared.glyphPath(id)
        let b = MathEngine.shared.glyphPath(id)
        XCTAssertNotNil(a)
        XCTAssertTrue(a === b)
    }

    func testDrawsIntoBitmap() throws {
        let layout = try MathEngine.shared.render(#"\sqrt{2}"#, fontSize: 40)
        let w = Int(layout.width.rounded(.up)) + 4, h = Int(layout.height.rounded(.up)) + 4
        let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        // Flip to y-down like UIKit.
        ctx.translateBy(x: 0, y: CGFloat(h)); ctx.scaleBy(x: 1, y: -1)
        MathEngine.shared.draw(layout, in: ctx, at: CGPoint(x: 2, y: 2))
        let data = ctx.data!.assumingMemoryBound(to: UInt8.self)
        var ink = 0
        for i in stride(from: 3, to: w * h * 4, by: 4) where data[i] > 0 { ink += 1 }
        XCTAssertGreaterThan(ink, 50)
    }
}
