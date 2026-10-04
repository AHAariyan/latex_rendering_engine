import UIKit

/// The engine's UIKit view behind the React Native component, with the
/// properties and callbacks Objective-C++ needs. Props arrive one at a time;
/// `commit()` applies them in one layout pass.
@objc(RNMathCoreHostView)
public final class RNMathCoreHostView: UIView {
    private let math = MathView()

    @objc public var latex: String = ""
    @objc public var fontSize: CGFloat = 17
    @objc public var color: UIColor = .label
    @objc public var displayMode: Bool = true
    @objc public var wrap: Bool = true
    @objc public var asciimath: Bool = false
    @objc public var speechVerbosity: Int = 1

    /// Content size in points, whenever it changes.
    @objc public var onSize: ((CGFloat, CGFloat) -> Void)?
    /// Byte range of the source under a tap.
    @objc public var onTap: ((Int, Int) -> Void)? {
        didSet { math.onTap = onTap == nil ? nil : { [weak self] r in self?.onTap?(r.start, r.end) } }
    }
    @objc public var onError: ((String) -> Void)?

    /// The last size reported, so it can be sent again to a listener that
    /// attached after it was measured; width -1 until then.
    @objc public private(set) var reported = CGSize(width: -1, height: -1)
    private var reportedError: String?

    public override init(frame: CGRect) {
        super.init(frame: frame)
        addSubview(math)
    }

    required init?(coder: NSCoder) { fatalError("not used from a storyboard") }

    /// Fabric reuses views: forget what was reported for the last formula.
    @objc public func prepareForReuse() {
        reported = CGSize(width: -1, height: -1)
        reportedError = nil
    }

    @objc public func commit() {
        let tex = asciimath ? ((try? MathEngine.asciimathToTex(latex)) ?? latex) : latex
        math.fontSize = fontSize
        math.textColor = color
        math.displayMode = displayMode
        math.speechVerbosity = SpeechVerbosity(rawValue: Int32(speechVerbosity)) ?? .brief
        math.maxWidth = wrap ? bounds.width : 0
        math.latex = tex
        if math.error != reportedError {
            reportedError = math.error
            if let e = math.error { onError?(e) }
        }
        report()
    }

    public override func layoutSubviews() {
        super.layoutSubviews()
        if wrap, abs(math.maxWidth - bounds.width) > 0.5 {
            math.maxWidth = bounds.width
            report()
        }
        let size = math.intrinsicContentSize
        math.frame = CGRect(x: 0, y: 0, width: max(size.width, wrap ? bounds.width : 0), height: size.height)
    }

    private func report() {
        let size = math.intrinsicContentSize
        guard abs(size.width - reported.width) > 0.25 || abs(size.height - reported.height) > 0.25 else { return }
        reported = size
        onSize?(size.width, size.height)
    }
}

/// The engine's calls that need no view, for the TurboModule.
@objc(RNMathCoreBridge)
public final class RNMathCoreBridge: NSObject {
    private static func level(_ v: Double) -> SpeechVerbosity { SpeechVerbosity(rawValue: Int32(v)) ?? .brief }

    @objc public static func speech(_ tex: String, verbosity: Double, language: String) throws -> String {
        try MathEngine.speech(tex, verbosity: level(verbosity), language: language.isEmpty ? nil : language)
    }

    @objc public static func speechTreeJson(_ tex: String, verbosity: Double, language: String) throws -> String {
        try MathEngine.speechTreeJson(tex, verbosity: level(verbosity), language: language.isEmpty ? nil : language)
    }

    @objc public static func mathml(_ tex: String, displayMode: Bool) throws -> String {
        try MathEngine.mathml(tex, displayMode: displayMode)
    }

    @objc public static func nemeth(_ tex: String) throws -> String {
        try MathEngine.nemeth(tex)
    }

    @objc public static func asciimathToTex(_ source: String) throws -> String {
        try MathEngine.asciimathToTex(source)
    }
}

/// The engine's MathField behind the React Native component.
@objc(RNMathCoreFieldHost)
public final class RNMathCoreFieldHost: UIView {
    private let field = MathField()

    @objc public var value: String = ""
    @objc public var fontSize: CGFloat = 20
    @objc public var color: UIColor = .label
    @objc public var cursorColor: UIColor?
    @objc public var placeholder: String = ""
    @objc public var editable: Bool = true

    /// Content size in points, whenever it changes.
    @objc public var onSize: ((CGFloat, CGFloat) -> Void)?
    /// The new TeX after every change.
    @objc public var onChange: ((String) -> Void)?

    @objc public private(set) var reported = CGSize(width: -1, height: -1)
    private var applied: String?

    public override init(frame: CGRect) {
        super.init(frame: frame)
        addSubview(field)
        field.onChange = { [weak self] tex in
            self?.applied = tex
            self?.onChange?(tex)
            self?.report()
        }
    }

    required init?(coder: NSCoder) { fatalError("not used from a storyboard") }

    @objc public func prepareForReuse() {
        reported = CGSize(width: -1, height: -1)
        applied = nil
        field.latex = ""
    }

    @objc public func commit() {
        field.fontSize = fontSize
        field.textColor = color
        if let c = cursorColor { field.tintColor = c }
        field.placeholder = placeholder
        field.isEditable = editable
        // The value JavaScript echoes back after a change is the one held:
        // leave the cursor where it is.
        if value != applied, value != field.latex {
            field.latex = value
        }
        applied = value
        report()
    }

    @objc public func runCommand(_ name: String) { field.command(name) }
    @objc public func typeText(_ text: String) { field.type(text) }
    @objc public func focusField() { field.becomeFirstResponder() }
    @objc public func blurField() { field.resignFirstResponder() }

    public override func layoutSubviews() {
        super.layoutSubviews()
        let size = field.intrinsicContentSize
        field.frame = CGRect(x: 0, y: 0, width: max(size.width, bounds.width), height: max(size.height, bounds.height))
    }

    private func report() {
        let size = field.intrinsicContentSize
        setNeedsLayout()
        guard abs(size.width - reported.width) > 0.25 || abs(size.height - reported.height) > 0.25 else { return }
        reported = size
        onSize?(size.width, size.height)
    }
}
