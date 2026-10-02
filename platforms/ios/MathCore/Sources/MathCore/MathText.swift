#if canImport(SwiftUI)
import SwiftUI
#if canImport(UIKit)
import UIKit
#elseif canImport(AppKit)
import AppKit
#endif

/// SwiftUI view that typesets a TeX formula natively.
///
/// With `wrap` (the default) a formula too wide for the offered width is
/// broken into lines, and the view takes the height of the broken formula.
/// VoiceOver reads the formula as a sentence. `onTap` reports the smallest
/// piece of source under the finger (iOS 16 / macOS 13 and later).
@available(iOS 15.0, macOS 12.0, tvOS 15.0, watchOS 8.0, *)
public struct MathText: View {
    let latex: String
    let fontSize: CGFloat
    let color: Color
    let displayMode: Bool
    let wrap: Bool
    let engine: MathEngine
    let onTap: ((MathSourceRegion) -> Void)?

    @State private var wrappedHeight: CGFloat?

    public init(_ latex: String, fontSize: CGFloat = 17, color: Color = .primary, displayMode: Bool = true,
                wrap: Bool = true, engine: MathEngine = .shared, onTap: ((MathSourceRegion) -> Void)? = nil) {
        self.latex = latex
        self.fontSize = fontSize
        self.color = color
        self.displayMode = displayMode
        self.wrap = wrap
        self.engine = engine
        self.onTap = onTap
    }

    public var body: some View {
        if wrap {
            GeometryReader { geo in
                content(maxWidth: geo.size.width)
                    .preference(key: HeightKey.self, value: height(maxWidth: geo.size.width))
            }
            .frame(height: wrappedHeight ?? height(maxWidth: nil) ?? fontSize * 1.4)
            .onPreferenceChange(HeightKey.self) { wrappedHeight = $0 }
        } else {
            content(maxWidth: nil)
        }
    }

    private func height(maxWidth: CGFloat?) -> CGFloat? {
        try? render(maxWidth: maxWidth).get().height
    }

    private func render(maxWidth: CGFloat?) -> Result<MathLayout, Error> {
        Result {
            try engine.render(latex, fontSize: fontSize, displayMode: displayMode, color: color.argb,
                              maxWidth: maxWidth, hitTesting: onTap != nil)
        }
    }

    @ViewBuilder
    private func content(maxWidth: CGFloat?) -> some View {
        switch render(maxWidth: maxWidth) {
        case .success(let layout):
            Canvas { gc, _ in
                gc.withCGContext { ctx in engine.draw(layout, in: ctx) }
            }
            .frame(width: layout.width, height: layout.height)
            .modifier(TapModifier(layout: layout, onTap: onTap))
            .accessibilityElement()
            .accessibilityAddTraits(.isStaticText)
            .accessibilityLabel(Text(verbatim: (try? MathEngine.speech(latex)) ?? latex))
        case .failure(let error):
            Text(verbatim: String(describing: error)).font(.caption).foregroundColor(.red)
        }
    }
}

@available(iOS 15.0, macOS 12.0, tvOS 15.0, watchOS 8.0, *)
private struct TapModifier: ViewModifier {
    let layout: MathLayout
    let onTap: ((MathSourceRegion) -> Void)?

    func body(content: Content) -> some View {
        if let onTap, #available(iOS 16.0, macOS 13.0, tvOS 16.0, watchOS 9.0, *) {
            content.gesture(SpatialTapGesture().onEnded { value in
                if let region = layout.hitNearest(value.location) { onTap(region) }
            })
        } else {
            content
        }
    }
}

private struct HeightKey: PreferenceKey {
    static var defaultValue: CGFloat? { nil }
    static func reduce(value: inout CGFloat?, nextValue: () -> CGFloat?) {
        value = nextValue() ?? value
    }
}

@available(iOS 15.0, macOS 12.0, tvOS 15.0, watchOS 8.0, *)
extension Color {
    /// 0xAARRGGBB in sRGB, resolving dynamic colours for the current appearance.
    var argb: UInt32 {
        #if canImport(UIKit)
        let cg = UIColor(self).cgColor
        #else
        let cg = NSColor(self).cgColor
        #endif
        let srgb = cg.converted(to: CGColorSpace(name: CGColorSpace.sRGB)!, intent: .defaultIntent, options: nil) ?? cg
        let c = srgb.components ?? [0, 0, 0, 1]
        let (r, g, b, a) = c.count >= 4 ? (c[0], c[1], c[2], c[3]) : (c[0], c[0], c[0], c.last ?? 1)
        func byte(_ v: CGFloat) -> UInt32 { UInt32((min(max(v, 0), 1) * 255).rounded()) }
        return byte(a) << 24 | byte(r) << 16 | byte(g) << 8 | byte(b)
    }
}
#endif
