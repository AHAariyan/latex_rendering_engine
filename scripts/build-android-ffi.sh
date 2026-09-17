#!/usr/bin/env bash
# Cross-compiles the C ABI library (used by Flutter) for every Android ABI.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-platforms/flutter/mathcore_flutter/android/src/main/jniLibs}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1)"
  export ANDROID_NDK_HOME
fi
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$OUT" build --release -p mathffi
ls -la "$OUT"/*/libmathcore_ffi.so
