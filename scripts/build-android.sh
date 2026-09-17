#!/usr/bin/env bash
# Cross-compiles the JNI library for every Android ABI into the given jniLibs dir.
# Usage: scripts/build-android.sh [output-dir]   (default: platforms/android/mathview/src/main/jniLibs)
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-platforms/android/mathview/src/main/jniLibs}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1)"
  export ANDROID_NDK_HOME
fi
if ! command -v cargo-ndk >/dev/null; then
  echo "cargo-ndk missing: cargo install cargo-ndk && rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android" >&2
  exit 1
fi
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$OUT" build --release -p mathjni
ls -la "$OUT"/*/libmathcore_android.so
