#if canImport(UIKit)
import UIKit

/// A UIView that typesets a TeX formula natively and sizes itself to it.
///
/// VoiceOver first reads the whole formula, then lets the user swipe through
/// its parts (each term, fraction, script or matrix row) with the part's
/// frame outlined, the way a sighted reader's eye moves over it.
public final class MathView: UIView {
    public var engine: MathEngine = .shared { didSet { relayout() } }
    public var latex: String = "" { didSet { if latex != oldValue { relayout() } } }
    public var fontSize: CGFloat = 17 { didSet { relayout() } }
    public var textColor: UIColor = .label { didSet { relayout() } }
    public var displayMode: Bool = true { didSet { relayout() } }
    /// Break the formula to fit this width. Zero renders one line of any width.
    public var maxWidth: CGFloat = 0 { didSet { relayout() } }
    /// How much scaffolding VoiceOver hears.
    public var speechVerbosity: SpeechVerbosity = .brief { didSet { relayout() } }
    /// The language VoiceOver hears the formula in (a BCP 47 tag); nil follows the user's.
    public var speechLanguage: String? { didSet { relayout() } }
    /// Called with the smallest piece of source under a tap.
    public var onTap: ((MathSourceRegion) -> Void)? {
        didSet { tapRecognizer.isEnabled = onTap != nil }
    }
    /// Non-nil when the current `latex` failed to parse.
    public private(set) var error: String?

    private var layoutResult: MathLayout?
    private lazy var tapRecognizer: UITapGestureRecognizer = {
        let r = UITapGestureRecognizer(target: self, action: #selector(tapped(_:)))
        r.isEnabled = false
        addGestureRecognizer(r)
        return r
    }()

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
        _ = tapRecognizer
    }

    private func relayout() {
        do {
            error = nil
            // Regions are always recorded: they place the VoiceOver parts.
            layoutResult = latex.isEmpty
                ? nil
                : try engine.render(latex, fontSize: fontSize, displayMode: displayMode, color: textColor.argb,
                                    maxWidth: maxWidth > 0 ? maxWidth : nil, hitTesting: true)
        } catch {
            self.error = "\(error)"
            layoutResult = nil
        }
        rebuildAccessibility()
        invalidateIntrinsicContentSize()
        setNeedsDisplay()
    }

    private func rebuildAccessibility() {
        let sentence = (try? MathEngine.speech(latex, verbosity: speechVerbosity, language: speechLanguage)) ?? latex
        guard let layout = layoutResult,
              let tree = try? MathEngine.speechTree(latex, verbosity: speechVerbosity, language: speechLanguage),
              tree.children.count > 1 else {
            isAccessibilityElement = true
            accessibilityTraits = .staticText
            accessibilityLabel = sentence
            accessibilityElements = nil
            return
        }
        isAccessibilityElement = false
        let whole = UIAccessibilityElement(accessibilityContainer: self)
        whole.accessibilityLabel = sentence
        whole.accessibilityLanguage = speechLanguage ?? MathEngine.deviceLanguage
        whole.accessibilityTraits = .staticText
        whole.accessibilityFrameInContainerSpace = bounds
        var elements: [Any] = [whole]
        for part in tree.children {
            let rects = layout.highlight(start: part.start, end: part.end)
            guard let first = rects.first else { continue }
            let e = UIAccessibilityElement(accessibilityContainer: self)
            e.accessibilityLabel = part.label.isEmpty ? part.text : "\(part.label): \(part.text)"
            e.accessibilityLanguage = whole.accessibilityLanguage
            e.accessibilityTraits = .staticText
            e.accessibilityFrameInContainerSpace = rects.dropFirst().reduce(first) { $0.union($1) }.insetBy(dx: -2, dy: -2)
            elements.append(e)
        }
        accessibilityElements = elements
    }

    @objc private func tapped(_ r: UITapGestureRecognizer) {
        guard let layout = layoutResult, let region = layout.hitNearest(r.location(in: self)) else { return }
        onTap?(region)
    }

    public override var intrinsicContentSize: CGSize { layoutResult?.size ?? .zero }

    /// Re-breaks the formula when the width the superview offers changes.
    public override func layoutSubviews() {
        super.layoutSubviews()
        let available = bounds.width
        if available > 0, abs(available - maxWidth) > 0.5 {
            maxWidth = available
        }
        if let whole = accessibilityElements?.first as? UIAccessibilityElement {
            whole.accessibilityFrameInContainerSpace = bounds
        }
    }

    public override func draw(_ rect: CGRect) {
        guard let l = layoutResult, let ctx = UIGraphicsGetCurrentContext() else { return }
        engine.draw(l, in: ctx)
    }

    public override func traitCollectionDidChange(_ previous: UITraitCollection?) {
        super.traitCollectionDidChange(previous)
        relayout() // dynamic colors such as .label may have changed
    }
}

extension UIColor {
    /// 0xAARRGGBB in sRGB for the current trait collection.
    var argb: UInt32 {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        if !getRed(&r, green: &g, blue: &b, alpha: &a) {
            var white: CGFloat = 0
            getWhite(&white, alpha: &a)
            (r, g, b) = (white, white, white)
        }
        func byte(_ v: CGFloat) -> UInt32 { UInt32((min(max(v, 0), 1) * 255).rounded()) }
        return byte(a) << 24 | byte(r) << 16 | byte(g) << 8 | byte(b)
    }
}
#endif
