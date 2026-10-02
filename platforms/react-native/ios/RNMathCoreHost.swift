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

    @objc public static func speech(_ tex: String, verbosity: Double) throws -> String {
        try MathEngine.speech(tex, verbosity: level(verbosity))
    }

    @objc public static func speechTreeJson(_ tex: String, verbosity: Double) throws -> String {
        try MathEngine.speechTreeJson(tex, verbosity: level(verbosity))
    }

    @objc public static func mathml(_ tex: String, displayMode: Bool) throws -> String {
        try MathEngine.mathml(tex, displayMode: displayMode)
    }

    @objc public static func asciimathToTex(_ source: String) throws -> String {
        try MathEngine.asciimathToTex(source)
    }
}
