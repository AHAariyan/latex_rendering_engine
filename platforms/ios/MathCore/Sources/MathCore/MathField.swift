#if canImport(UIKit)
import SwiftUI
import UIKit

/// An editable formula: a text field for mathematics.
///
/// It takes the system keyboard (and a hardware one, with arrows, Shift to
/// select, Cmd+A/Z/C/V/X), shows a bar of math keys above it, and tells
/// VoiceOver the formula and, after each key, where the cursor is.
///
///     let field = MathField()
///     field.latex = "x^2"
///     field.onChange = { tex in ... }
public final class MathField: UIView, UIKeyInput {
    public var engine: MathEngine = .shared { didSet { relayout() } }
    public var fontSize: CGFloat = 20 { didSet { relayout() } }
    public var textColor: UIColor = .label { didSet { relayout() } }
    /// Shown, dimmed, while the field is empty.
    public var placeholder: String = "" { didSet { setNeedsDisplay() } }
    public var isEditable = true
    /// The language VoiceOver hears (a BCP 47 tag); nil follows the user's.
    public var speechLanguage: String?
    /// Called with the new TeX after every change.
    public var onChange: ((String) -> Void)?
    /// Space between the formula and the field's edge.
    public var contentInsets = UIEdgeInsets(top: 8, left: 10, bottom: 8, right: 10) { didSet { relayout() } }
    /// Where copy and paste go; the system clipboard unless you set another.
    public var pasteboard: UIPasteboard = .general
    /// The bar of math keys above the keyboard; set false to use your own.
    public var showsMathKeys = true

    public let editor = MathEditor()
    private var layoutResult: MathEditor.Layout?
    private let caretLayer = CALayer()

    /// The formula as TeX.
    public var latex: String {
        get { editor.latex }
        set { editor.latex = newValue; relayout() }
    }

    public override init(frame: CGRect) {
        super.init(frame: frame)
        setUp()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setUp()
    }

    private func setUp() {
        isOpaque = false
        contentMode = .redraw
        layer.addSublayer(caretLayer)
        caretLayer.isHidden = true
        addGestureRecognizer(UITapGestureRecognizer(target: self, action: #selector(tapped(_:))))
        isAccessibilityElement = true
        accessibilityTraits = []
        relayout()
    }

    // MARK: Layout and drawing

    private func relayout() {
        layoutResult = try? editor.layout(engine: engine, fontSize: fontSize, color: textColor.argb)
        accessibilityLabel = placeholder.isEmpty ? "math" : placeholder
        accessibilityValue = editor.speech(language: speechLanguage)
        invalidateIntrinsicContentSize()
        setNeedsDisplay()
        placeCaret()
    }

    public override var intrinsicContentSize: CGSize {
        let w = layoutResult?.formula.width ?? 0, h = layoutResult.map { $0.formula.ascent + $0.formula.descent } ?? fontSize
        return CGSize(width: max(w, fontSize * 2) + contentInsets.left + contentInsets.right,
                      height: max(h, fontSize * 1.2) + contentInsets.top + contentInsets.bottom)
    }

    /// Where the formula's top-left corner is drawn.
    private var origin: CGPoint {
        let h = layoutResult.map { $0.formula.ascent + $0.formula.descent } ?? 0
        return CGPoint(x: contentInsets.left, y: max(contentInsets.top, (bounds.height - h) / 2))
    }

    public override func draw(_ rect: CGRect) {
        guard let ctx = UIGraphicsGetCurrentContext(), let l = layoutResult else { return }
        let o = origin
        ctx.setFillColor(tintColor.withAlphaComponent(0.3).cgColor)
        for r in l.selection { ctx.fill(r.offsetBy(dx: o.x, dy: o.y)) }
        if editor.latex.isEmpty, !placeholder.isEmpty, !isFirstResponder {
            (placeholder as NSString).draw(at: o, withAttributes: [
                .font: UIFont.systemFont(ofSize: fontSize * 0.8), .foregroundColor: UIColor.placeholderText,
            ])
        } else {
            engine.draw(l.formula, in: ctx, at: o)
        }
    }

    private func placeCaret() {
        guard let l = layoutResult else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        caretLayer.backgroundColor = tintColor.cgColor
        caretLayer.frame = CGRect(x: origin.x + l.caret.minX, y: origin.y + l.caret.minY,
                                  width: max(l.caret.width, 2), height: l.caret.height)
        caretLayer.isHidden = !isFirstResponder || !isEditable
        CATransaction.commit()
        caretLayer.removeAnimation(forKey: "blink")
        guard !caretLayer.isHidden, !UIAccessibility.isReduceMotionEnabled else { return }
        let blink = CAKeyframeAnimation(keyPath: "opacity")
        blink.values = [1, 1, 0, 0]
        blink.keyTimes = [0, 0.5, 0.5, 1]
        blink.duration = 1.06
        blink.repeatCount = .infinity
        caretLayer.add(blink, forKey: "blink")
    }

    public override func layoutSubviews() {
        super.layoutSubviews()
        placeCaret()
    }

    public override func tintColorDidChange() {
        super.tintColorDidChange()
        setNeedsDisplay()
        placeCaret()
    }

    public override func traitCollectionDidChange(_ previous: UITraitCollection?) {
        super.traitCollectionDidChange(previous)
        relayout()
    }

    // MARK: Editing

    private func changed(_ edit: () -> Void) {
        guard isEditable else { return }
        let before = editor.latex
        edit()
        refresh(notify: editor.latex != before)
    }

    private func moved(_ move: () -> Void) {
        move()
        refresh(notify: false)
    }

    private func refresh(notify: Bool) {
        relayout()
        if notify { onChange?(editor.latex) }
        if UIAccessibility.isVoiceOverRunning {
            UIAccessibility.post(notification: .announcement, argument: editor.cursorDescription(language: speechLanguage))
        }
    }

    @objc private func tapped(_ g: UITapGestureRecognizer) {
        if !isFirstResponder { becomeFirstResponder() }
        let p = g.location(in: self), o = origin
        moved { editor.tap(at: CGPoint(x: p.x - o.x, y: p.y - o.y)) }
    }

    // MARK: UIKeyInput

    public var hasText: Bool { !editor.latex.isEmpty }
    public func insertText(_ text: String) {
        // Return leaves the slot the cursor is in.
        if text == "\n" { moved { editor.key(.enter) } } else { changed { editor.type(text) } }
    }
    public func deleteBackward() { changed { editor.key(.backspace) } }

    public var autocapitalizationType: UITextAutocapitalizationType = .none
    public var autocorrectionType: UITextAutocorrectionType = .no
    public var spellCheckingType: UITextSpellCheckingType = .no
    public var smartQuotesType: UITextSmartQuotesType = .no
    public var smartDashesType: UITextSmartDashesType = .no
    public var keyboardType: UIKeyboardType = .numbersAndPunctuation

    public override var canBecomeFirstResponder: Bool { true }

    @discardableResult
    public override func becomeFirstResponder() -> Bool {
        let ok = super.becomeFirstResponder()
        placeCaret()
        setNeedsDisplay()
        return ok
    }

    @discardableResult
    public override func resignFirstResponder() -> Bool {
        let ok = super.resignFirstResponder()
        placeCaret()
        setNeedsDisplay()
        return ok
    }

    // MARK: Hardware keyboard

    public override var keyCommands: [UIKeyCommand]? {
        var out: [UIKeyCommand] = []
        let arrows: [(String, MathEditor.Key)] = [
            (UIKeyCommand.inputLeftArrow, .left), (UIKeyCommand.inputRightArrow, .right),
            (UIKeyCommand.inputUpArrow, .up), (UIKeyCommand.inputDownArrow, .down),
        ]
        for (input, _) in arrows {
            for flags in [UIKeyModifierFlags(), .shift] {
                let c = UIKeyCommand(input: input, modifierFlags: flags, action: #selector(arrow(_:)))
                if #available(iOS 15, *) { c.wantsPriorityOverSystemBehavior = true }
                out.append(c)
            }
        }
        out.append(UIKeyCommand(input: "z", modifierFlags: .command, action: #selector(undoKey)))
        out.append(UIKeyCommand(input: "z", modifierFlags: [.command, .shift], action: #selector(redoKey)))
        return out
    }

    @objc private func arrow(_ c: UIKeyCommand) {
        let key: MathEditor.Key
        switch c.input {
        case UIKeyCommand.inputLeftArrow: key = .left
        case UIKeyCommand.inputRightArrow: key = .right
        case UIKeyCommand.inputUpArrow: key = .up
        default: key = .down
        }
        moved { editor.key(key, shift: c.modifierFlags.contains(.shift)) }
    }

    @objc private func undoKey() { changed { editor.undo() } }
    @objc private func redoKey() { changed { editor.redo() } }

    // MARK: Copy and paste

    public override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {
        switch action {
        case #selector(copy(_:)): return hasText
        case #selector(cut(_:)): return isEditable && !editor.selectedLatex.isEmpty
        case #selector(paste(_:)): return isEditable && pasteboard.hasStrings
        case #selector(selectAll(_:)): return hasText
        default: return super.canPerformAction(action, withSender: sender)
        }
    }

    public override func copy(_ sender: Any?) {
        pasteboard.string = editor.selectedLatex.isEmpty ? editor.latex : editor.selectedLatex
    }

    public override func cut(_ sender: Any?) {
        copy(sender)
        changed { editor.key(.backspace) }
    }

    public override func paste(_ sender: Any?) {
        guard let text = pasteboard.string else { return }
        changed { editor.insert(latex: text) }
    }

    public override func selectAll(_ sender: Any?) { moved { editor.selectAll() } }

    // MARK: Math keys

    private static let mathKeys: [(title: String, label: String, action: String)] = [
        ("a⁄b", "fraction", "cmd:frac"), ("x²", "squared", "type:^2"), ("xⁿ", "power", "type:^"), ("xₙ", "subscript", "type:_"),
        ("√", "square root", "cmd:sqrt"), ("( )", "parentheses", "type:("), ("π", "pi", "cmd:pi"),
        ("×", "times", "cmd:times"), ("÷", "divided by", "cmd:div"), ("≤", "less or equal", "cmd:le"), ("≥", "greater or equal", "cmd:ge"),
        ("∞", "infinity", "cmd:infty"), ("←", "left", "key:left"), ("→", "right", "key:right"),
    ]

    public override var inputAccessoryView: UIView? {
        guard showsMathKeys, isEditable else { return nil }
        return keyBar
    }

    private lazy var keyBar: UIView = {
        let scroll = UIScrollView(frame: CGRect(x: 0, y: 0, width: 320, height: 46))
        scroll.backgroundColor = .secondarySystemBackground
        scroll.showsHorizontalScrollIndicator = false
        scroll.autoresizingMask = .flexibleWidth
        let stack = UIStackView()
        stack.axis = .horizontal
        stack.spacing = 6
        stack.translatesAutoresizingMaskIntoConstraints = false
        for (i, k) in MathField.mathKeys.enumerated() {
            let b = UIButton(type: .system)
            b.setTitle(k.title, for: .normal)
            b.titleLabel?.font = .systemFont(ofSize: 20)
            b.accessibilityLabel = k.label
            b.backgroundColor = .systemBackground
            b.layer.cornerRadius = 6
            b.tag = i
            b.widthAnchor.constraint(greaterThanOrEqualToConstant: 44).isActive = true
            b.addTarget(self, action: #selector(mathKey(_:)), for: .touchUpInside)
            stack.addArrangedSubview(b)
        }
        scroll.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: scroll.contentLayoutGuide.leadingAnchor, constant: 6),
            stack.trailingAnchor.constraint(equalTo: scroll.contentLayoutGuide.trailingAnchor, constant: -6),
            stack.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor, constant: 5),
            stack.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor, constant: -5),
            stack.heightAnchor.constraint(equalTo: scroll.frameLayoutGuide.heightAnchor, constant: -10),
        ])
        return scroll
    }()

    @objc private func mathKey(_ b: UIButton) {
        let action = MathField.mathKeys[b.tag].action
        let value = String(action.drop { $0 != ":" }.dropFirst())
        if action.hasPrefix("cmd:") {
            changed { editor.command(value) }
        } else if action.hasPrefix("type:") {
            changed { editor.type(value) }
        } else {
            moved { editor.key(value == "left" ? .left : .right) }
        }
    }
}

/// `MathField` for SwiftUI: `MathInput(latex: $answer)`.
@available(iOS 13.0, *)
public struct MathInput: UIViewRepresentable {
    @Binding var latex: String
    var fontSize: CGFloat
    var placeholder: String

    public init(latex: Binding<String>, fontSize: CGFloat = 20, placeholder: String = "") {
        _latex = latex
        self.fontSize = fontSize
        self.placeholder = placeholder
    }

    public func makeUIView(context: Context) -> MathField {
        let f = MathField()
        f.setContentHuggingPriority(.required, for: .vertical)
        f.onChange = { tex in
            if tex != latex { latex = tex }
        }
        return f
    }

    public func updateUIView(_ f: MathField, context: Context) {
        f.fontSize = fontSize
        f.placeholder = placeholder
        if f.latex != latex { f.latex = latex }
    }
}
#endif
