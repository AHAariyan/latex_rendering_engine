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
        guard case let .glyph(font, id, _, _, _, _) = layout.items[0] else { return XCTFail("glyph expected") }
        let a = MathEngine.shared.glyphPath(font: font, glyph: id)
        let b = MathEngine.shared.glyphPath(font: font, glyph: id)
        XCTAssertNotNil(a)
        XCTAssertTrue(a === b)
    }

    func testHitTestingMapsAPointToTheSource() throws {
        let tex = #"\frac{a}{b} + x"#
        let layout = try MathEngine.shared.render(tex, fontSize: 32, hitTesting: true)
        XCTAssertFalse(layout.regions.isEmpty)
        // The first glyph is the numerator's `a` (the fraction bar is a rule).
        let firstGlyph = layout.items.lazy.compactMap { item -> (CGFloat, CGFloat)? in
            if case let .glyph(_, _, x, y, _, _) = item { return (x, y) }
            return nil
        }.first
        guard let (x, y) = firstGlyph else { return XCTFail("glyph expected") }
        let point = CGPoint(x: x + 1, y: y - 5)
        XCTAssertEqual(layout.hitTest(point)?.text(in: tex), "a")
        XCTAssertEqual(layout.hit(point).first?.text(in: tex), #"\frac{a}{b}"#)
        XCTAssertNil(layout.hitTest(CGPoint(x: -10, y: -10)))
        XCTAssertTrue(try MathEngine.shared.render(tex, fontSize: 32).regions.isEmpty)
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

final class MathCoreFeatureTests: XCTestCase {
    func testSpeechVerbosityAndTree() throws {
        XCTAssertEqual(try MathEngine.speech(#"\frac{1}{2}"#, verbosity: .verbose), "the fraction 1 over 2, end fraction")
        let tree = try MathEngine.speechTree(#"\frac{a+b}{c} = 1"#)
        XCTAssertEqual(tree.role, "formula")
        XCTAssertEqual(tree.children.first?.children.first?.label, "numerator")
    }

    func testHighlightCoversAPart() throws {
        let tex = #"a + \frac{b}{c}"#
        let layout = try MathEngine.shared.render(tex, fontSize: 32, hitTesting: true)
        let start = tex.utf8.distance(from: tex.utf8.startIndex, to: tex.range(of: #"\frac"#)!.lowerBound)
        let rects = layout.highlight(start: start, end: tex.utf8.count)
        XCTAssertEqual(rects.count, 1)
        XCTAssertGreaterThan(rects[0].minX, 20)
    }

    func testAsciiMathBudgetAndCache() throws {
        XCTAssertEqual(try MathEngine.asciimathToTex("x/y"), #"\frac{x}{y}"#)
        let engine = try MathEngine()
        engine.setBudget(maxNodes: 10)
        XCTAssertThrowsError(try engine.render(String(repeating: "x+", count: 50) + "x", fontSize: 20))
        engine.setBudget()
        engine.setCacheCapacity(0)
        XCTAssertNoThrow(try engine.render(String(repeating: "x+", count: 50) + "x", fontSize: 20))
    }

    func testChemistryAndText() throws {
        XCTAssertNoThrow(try MathEngine.shared.render(#"\ce{2H2 + O2 -> 2H2O}"#, fontSize: 20))
        XCTAssertNoThrow(try MathEngine.shared.render(#"\text{if $x>0$ then}"#, fontSize: 20))
    }
}
