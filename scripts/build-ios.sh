#!/usr/bin/env bash
# Builds MathCoreFFI.xcframework (device + simulator) from the C ABI crate.
# Requires macOS with Xcode and: rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-platforms/ios/MathCore/MathCoreFFI.xcframework}"
HDR=$(mktemp -d)
cp crates/mathffi/include/mathcore.h "$HDR/"
cp platforms/ios/MathCore/Sources/CMathCore/include/module.modulemap "$HDR/"

for t in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  cargo build --release -p mathffi --target "$t"
done
SIM=$(mktemp -d)
lipo -create target/aarch64-apple-ios-sim/release/libmathcore_ffi.a target/x86_64-apple-ios/release/libmathcore_ffi.a -output "$SIM/libmathcore_ffi.a"

rm -rf "$OUT"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libmathcore_ffi.a -headers "$HDR" \
  -library "$SIM/libmathcore_ffi.a" -headers "$HDR" \
  -output "$OUT"
echo "wrote $OUT"
