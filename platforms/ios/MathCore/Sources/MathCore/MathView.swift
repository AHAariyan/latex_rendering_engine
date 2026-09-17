#if canImport(UIKit)
import UIKit

/// A UIView that typesets a TeX formula natively and sizes itself to it.
public final class MathView: UIView {
    public var engine: MathEngine = .shared { didSet { relayout() } }
    public var latex: String = "" { didSet { if latex != oldValue { relayout() } } }
    public var fontSize: CGFloat = 17 { didSet { relayout() } }
    public var textColor: UIColor = .label { didSet { relayout() } }
    public var displayMode: Bool = true { didSet { relayout() } }
    /// Break the formula to fit this width. Zero renders one line of any width.
    public var maxWidth: CGFloat = 0 { didSet { relayout() } }
    /// Non-nil when the current `latex` failed to parse.
    public private(set) var error: String?

    private var layoutResult: MathLayout?

    public override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        contentMode = .redraw
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        isOpaque = false
        contentMode = .redraw
    }

    private func relayout() {
        do {
            error = nil
            layoutResult = latex.isEmpty
                ? nil
                : try engine.render(latex, fontSize: fontSize, displayMode: displayMode, color: textColor.argb,
                                    maxWidth: maxWidth > 0 ? maxWidth : nil)
        } catch {
            self.error = "\(error)"
            layoutResult = nil
        }
        isAccessibilityElement = true
        accessibilityTraits = .staticText
        accessibilityLabel = (try? MathEngine.speech(latex)) ?? latex
        invalidateIntrinsicContentSize()
        setNeedsDisplay()
    }

    public override var intrinsicContentSize: CGSize { layoutResult?.size ?? .zero }

    /// Re-breaks the formula when the width the superview offers changes.
    public override func layoutSubviews() {
        super.layoutSubviews()
        let available = bounds.width
        if available > 0, abs(available - maxWidth) > 0.5 {
            maxWidth = available
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
    var argb: UInt32 {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        getRed(&r, green: &g, blue: &b, alpha: &a)
        return (UInt32(a * 255) << 24) | (UInt32(r * 255) << 16) | (UInt32(g * 255) << 8) | UInt32(b * 255)
    }
}
#endif
