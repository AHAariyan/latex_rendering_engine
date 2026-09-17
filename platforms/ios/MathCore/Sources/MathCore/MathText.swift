#if canImport(SwiftUI) && canImport(UIKit)
import SwiftUI

/// SwiftUI view that typesets a TeX formula natively.
public struct MathText: View {
    let latex: String
    let fontSize: CGFloat
    let color: Color
    let displayMode: Bool
    let wrap: Bool

    /// `wrap` breaks a formula too wide for the offered width into lines.
    public init(_ latex: String, fontSize: CGFloat = 17, color: Color = .primary, displayMode: Bool = true, wrap: Bool = true) {
        self.latex = latex
        self.fontSize = fontSize
        self.color = color
        self.displayMode = displayMode
        self.wrap = wrap
    }

    public var body: some View {
        if wrap {
            GeometryReader { geo in
                content(maxWidth: geo.size.width)
            }
            .frame(height: measuredHeight)
        } else {
            content(maxWidth: nil)
        }
    }

    private var measuredHeight: CGFloat {
        (try? MathEngine.shared.render(latex, fontSize: fontSize, displayMode: displayMode))?.height ?? fontSize * 1.4
    }

    @ViewBuilder
    private func content(maxWidth: CGFloat?) -> some View {
        let argb = UIColor(color).argb
        switch Result(catching: {
            try MathEngine.shared.render(latex, fontSize: fontSize, displayMode: displayMode, color: argb, maxWidth: maxWidth)
        }) {
        case .success(let layout):
            Canvas { gc, _ in
                gc.withCGContext { ctx in MathEngine.shared.draw(layout, in: ctx) }
            }
            .frame(width: layout.width, height: layout.height)
            .accessibilityElement()
            .accessibilityLabel((try? MathEngine.speech(latex)) ?? latex)
        case .failure(let error):
            Text("\(error)").font(.caption).foregroundColor(.red)
        }
    }
}
#endif
