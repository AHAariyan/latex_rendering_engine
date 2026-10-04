import CoreGraphics
import Foundation
import MathCoreFFI

/// The model behind a math input field: forward text, keys and taps to it,
/// lay it out, and draw the formula with the caret and selection it reports.
/// `MathField` is the ready-made view; use this directly to build your own.
///
/// Typing follows the usual conventions: `/` makes a fraction of the term
/// before it, `^` and `_` open a script, `(` opens a pair, `\` starts a
/// command name, and `sqrt`, `pi`, `sin`... become what they name.
public final class MathEditor {
    private let handle: OpaquePointer

    public init(latex: String = "") {
        handle = math_editor_new()
        if !latex.isEmpty { self.latex = latex }
    }

    deinit { math_editor_free(handle) }

    /// The content as TeX. Setting it puts the cursor at the end.
    public var latex: String {
        get { MathEditor.take(math_editor_tex(handle)) }
        set { newValue.withCString { math_editor_set_tex(handle, $0) } }
    }

    /// The selection as TeX, empty without one.
    public var selectedLatex: String { MathEditor.take(math_editor_selected_tex(handle)) }

    /// Types text at the cursor.
    public func type(_ text: String) { text.withCString { math_editor_type(handle, $0) } }

    /// Inserts TeX as structure (a pasted `\frac{a}{b}` stays a fraction).
    public func insert(latex: String) { latex.withCString { math_editor_insert_tex(handle, $0) } }

    /// Runs a command by name: `frac`, `sqrt`, `nthroot`, `alpha`...
    public func command(_ name: String) { name.withCString { math_editor_command(handle, $0) } }

    /// A key the editor handles.
    public enum Key: String {
        case left = "ArrowLeft", right = "ArrowRight", up = "ArrowUp", down = "ArrowDown"
        case home = "Home", end = "End", backspace = "Backspace", delete = "Delete", enter = "Enter"
    }

    /// Sends a key; with `shift`, arrows extend the selection.
    public func key(_ key: Key, shift: Bool = false) {
        _ = key.rawValue.withCString { math_editor_key(handle, $0, shift, false) }
    }

    public func selectAll() { _ = "a".withCString { math_editor_key(handle, $0, false, true) } }
    public func undo() { _ = "z".withCString { math_editor_key(handle, $0, false, true) } }
    public func redo() { _ = "z".withCString { math_editor_key(handle, $0, true, true) } }

    /// The editor drawn: the formula, the caret and the selection.
    public struct Layout {
        public let formula: MathLayout
        public let caret: CGRect
        public let selection: [CGRect]
    }

    /// Lays the editor out. Draw `formula` with `engine.draw`, then the
    /// selection and the caret; `tap` refers to this layout.
    public func layout(engine: MathEngine = .shared, fontSize: CGFloat, displayMode: Bool = true,
                       color: UInt32 = 0xFF00_0000) throws -> Layout {
        let formula = try engine.render(editor: handle, tex: latex, fontSize: fontSize, displayMode: displayMode, color: color)
        var c = [Float](repeating: 0, count: 4)
        math_editor_caret(handle, &c)
        var n = 0
        var selection: [CGRect] = []
        if let p = math_editor_selection(handle, &n) {
            for i in stride(from: 0, to: n, by: 4) {
                selection.append(CGRect(x: CGFloat(p[i]), y: CGFloat(p[i + 1]), width: CGFloat(p[i + 2]), height: CGFloat(p[i + 3])))
            }
            math_buffer_free(p, n)
        }
        return Layout(formula: formula,
                      caret: CGRect(x: CGFloat(c[0]), y: CGFloat(c[1]), width: CGFloat(c[2]), height: CGFloat(c[3])),
                      selection: selection)
    }

    /// Moves the cursor to a point of the last layout.
    public func tap(at point: CGPoint) { math_editor_tap(handle, Float(point.x), Float(point.y)) }

    /// What a screen reader says for the cursor's place ("denominator, 2"),
    /// in a language (a BCP 47 tag; the user's preferred one when nil).
    public func cursorDescription(language: String? = nil) -> String {
        (language ?? MathEngine.deviceLanguage).withCString { MathEditor.take(math_editor_describe(handle, $0)) }
    }

    /// The whole formula read aloud.
    public func speech(language: String? = nil) -> String {
        (language ?? MathEngine.deviceLanguage).withCString { MathEditor.take(math_editor_speech(handle, $0)) }
    }

    private static func take(_ p: UnsafeMutablePointer<CChar>?) -> String {
        guard let p else { return "" }
        defer { math_string_free(p) }
        return String(cString: p)
    }
}
