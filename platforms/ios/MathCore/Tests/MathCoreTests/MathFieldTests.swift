#if canImport(UIKit)
import XCTest
@testable import MathCore

/// MathField on a device: the keyboard protocol, drawing and VoiceOver values.
final class MathFieldTests: XCTestCase {
    func testKeyboardInputBuildsAFormula() {
        let field = MathField()
        var changes: [String] = []
        field.onChange = { changes.append($0) }
        for ch in "1/2" { field.insertText(String(ch)) }
        XCTAssertEqual(field.latex, #"\frac{1}{2}"#)
        XCTAssertEqual(changes.last, #"\frac{1}{2}"#)
        XCTAssertTrue(field.hasText)
        field.deleteBackward()
        XCTAssertEqual(field.latex, #"\frac{1}{}"#)
        // Return leaves the denominator; typing continues after the fraction.
        field.insertText("3")
        field.insertText("\n")
        field.insertText("+x")
        XCTAssertEqual(field.latex, #"\frac{1}{3}+x"#)
        XCTAssertEqual(field.accessibilityValue, MathEditor(latex: field.latex).speech())
    }

    func testReadOnlyIgnoresInput() {
        let field = MathField()
        field.latex = "x^2"
        field.isEditable = false
        field.insertText("y")
        field.deleteBackward()
        XCTAssertEqual(field.latex, "x^{2}")
        XCTAssertNil(field.inputAccessoryView)
    }

    func testSizesToItsFormulaAndDraws() {
        let field = MathField()
        field.fontSize = 30
        let empty = field.intrinsicContentSize
        field.latex = #"\frac{a+b}{c} + \sqrt{x}"#
        let size = field.intrinsicContentSize
        XCTAssertGreaterThan(size.width, empty.width)
        XCTAssertGreaterThan(size.height, empty.height)
        field.frame = CGRect(origin: .zero, size: size)
        let image = UIGraphicsImageRenderer(size: size).image { ctx in field.layer.render(in: ctx.cgContext) }
        // Something was drawn: the bitmap is not blank.
        guard let data = image.cgImage?.dataProvider?.data, let bytes = CFDataGetBytePtr(data) else { return XCTFail("no bitmap") }
        let n = CFDataGetLength(data)
        XCTAssertTrue((0..<n).contains { bytes[$0] != 0 })
        XCTAssertNotNil(field.inputAccessoryView)
        XCTAssertTrue(field.canBecomeFirstResponder)
        XCTAssertEqual(field.keyCommands?.count, 10)
    }

    func testCopyAndPasteAsStructure() {
        // A private clipboard: the system one prompts, which a test cannot answer.
        let board = UIPasteboard.withUniqueName()
        let field = MathField()
        field.pasteboard = board
        field.latex = #"\sqrt{2}"#
        field.selectAll(nil)
        field.copy(nil)
        XCTAssertEqual(board.string, #"\sqrt{2}"#)
        let other = MathField()
        other.pasteboard = board
        other.insertText("1+")
        other.paste(nil)
        XCTAssertEqual(other.latex, #"1+\sqrt{2}"#)
    }
}
#endif
