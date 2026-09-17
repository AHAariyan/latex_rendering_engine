#if canImport(SwiftUI) && canImport(UIKit)
import SwiftUI

/// SwiftUI view that typesets a TeX formula natively.
public struct MathText: View {
    let latex: String
    let fontSize: CGFloat
    let color: Color
    let displayMode: Bool

    public init(_ latex: String, fontSize: CGFloat = 17, color: Color = .primary, displayMode: Bool = true) {
        self.latex = latex
        self.fontSize = fontSize
        self.color = color
        self.displayMode = displayMode
    }

    public var body: some View {
        let argb = UIColor(color).argb
        switch Result(catching: { try MathEngine.shared.render(latex, fontSize: fontSize, displayMode: displayMode, color: argb) }) {
        case .success(let layout):
            Canvas { gc, _ in
                gc.withCGContext { ctx in MathEngine.shared.draw(layout, in: ctx) }
            }
            .frame(width: layout.width, height: layout.height)
        case .failure(let error):
            Text("\(error)").font(.caption).foregroundColor(.red)
        }
    }
}
#endif
