# MathCore for iOS

Swift package over the mathcore C ABI. Drawing is Core Graphics: glyph
outlines are cached as `CGPath` per glyph and filled with a translate+scale.

```swift
// SwiftUI. Wraps to the offered width unless you pass wrap: false.
MathText(#"\frac{a}{b} + \sqrt{x^2 + y^2}"#, fontSize: 24)

// UIKit
let view = MathView()
view.latex = #"x^2"#
view.fontSize = 20

// Engine
let layout = try MathEngine.shared.render(#"\sum_{i=1}^n i"#, fontSize: 40)
MathEngine.shared.draw(layout, in: context)
```

## Build the native library

On macOS with Xcode:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
scripts/build-ios.sh          # writes platforms/ios/MathCore/MathCoreFFI.xcframework
cd platforms/ios/MathCore && swift test
```

This package was written on Linux and has not been compiled with Xcode yet;
expect to fix small Swift compile issues on first build.
