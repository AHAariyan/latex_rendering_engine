// swift-tools-version:5.9
// Swift package for the mathcore native TeX math engine.
//
// The Rust engine is linked as a binary target (MathCoreFFI.xcframework),
// produced by scripts/build-ios.sh. Run that script once before building.
import PackageDescription

let package = Package(
    name: "MathCore",
    platforms: [.iOS(.v13), .macOS(.v11)],
    products: [
        .library(name: "MathCore", targets: ["MathCore"]),
    ],
    targets: [
        .binaryTarget(name: "MathCoreFFI", path: "MathCoreFFI.xcframework"),
        .target(name: "MathCore", dependencies: ["MathCoreFFI"], path: "Sources/MathCore"),
        .testTarget(name: "MathCoreTests", dependencies: ["MathCore"], path: "Tests/MathCoreTests"),
    ]
)
