// swift-tools-version: 5.9
// Swift Package Manager support for the Flutter plugin: the engine is a
// dynamic framework that Dart opens by name.
import PackageDescription

let package = Package(
    name: "mathcore_flutter",
    platforms: [.macOS("10.14")],
    products: [
        .library(name: "mathcore-flutter", targets: ["mathcore_flutter"]),
    ],
    targets: [
        .binaryTarget(name: "MathCoreFFI", path: "MathCoreFFI.xcframework"),
        .target(name: "mathcore_flutter", dependencies: ["MathCoreFFI"]),
    ]
)
