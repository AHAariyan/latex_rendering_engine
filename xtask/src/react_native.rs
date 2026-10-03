//! React Native SDK: `react-native-mathcore`, a Fabric view and TurboModule
//! over the engine's native views (Android `MathView`, iOS `MathView`), so a
//! React Native app gets the same rendering and screen-reader navigation.

use crate::util::*;
use std::path::Path;

const ANDROID_ABIS: &[(&str, &str)] = &[
    ("arm64-v8a", "aarch64-linux-android"),
    ("armeabi-v7a", "armv7-linux-androideabi"),
    ("x86_64", "x86_64-linux-android"),
];

/// The engine's Kotlin sources the React Native view builds on. Compose
/// (MathText) stays out: React Native apps do not carry it.
const KOTLIN: &[&str] = &[
    "NativeBridge.kt",
    "MathEngine.kt",
    "MathLayout.kt",
    "Accessibility.kt",
    "MathView.kt",
    "SystemFontFinder.kt",
];

pub fn build(verify: bool) -> Result {
    require("npm", "install Node.js 18 or later")?;
    let version = version();
    let pkg = root().join("platforms/react-native");
    let out = dist().join("react-native");
    reset_dir(&out)?;

    step("React Native: engine and native sources for Android");
    let sdk = android_sdk().ok_or("Android SDK not found: set ANDROID_HOME")?;
    let ndk = android_ndk(&sdk).ok_or("Android NDK not found")?;
    rustup_targets(&ANDROID_ABIS.iter().map(|a| a.1).collect::<Vec<_>>())?;
    let jni = pkg.join("android/src/main/jniLibs");
    reset_dir(&jni)?;
    let mut c = cmd("cargo");
    c.arg("ndk").env("ANDROID_HOME", &sdk).env("ANDROID_NDK_HOME", &ndk);
    for (abi, _) in ANDROID_ABIS {
        c.args(["-t", abi]);
    }
    c.args(["--platform", "24", "-o"])
        .arg(&jni)
        .args(["build", "--profile", "sdk", "-p", "mathjni"]);
    run(&mut c)?;
    let kotlin_src = root().join("platforms/android/mathview/src/main/java/dev/mathcore");
    let kotlin_dst = pkg.join("android/src/main/java/dev/mathcore");
    for f in KOTLIN {
        copy(&kotlin_src.join(f), &kotlin_dst.join(f))?;
    }

    if cfg!(target_os = "macos") {
        step("React Native: engine and native sources for iOS");
        let xcf = crate::apple::dynamic_xcframework(&version, crate::apple::ApplePlatform::Ios)?;
        let dest = pkg.join("ios/MathCoreFFI.xcframework");
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
        }
        copy_dir(&xcf, &dest)?;
        let swift = root().join("platforms/ios/MathCore/Sources/MathCore");
        reset_dir(&pkg.join("ios/MathCore"))?;
        for f in ["MathEngine.swift", "MathView.swift", "MathText.swift"] {
            copy(&swift.join(f), &pkg.join("ios/MathCore").join(f))?;
        }
    }

    step("React Native: package");
    copy(&root().join("LICENSE"), &pkg.join("LICENSE"))?;
    let tgz = output(
        cmd("npm")
            .args(["pack", "--silent", "--pack-destination"])
            .arg(&out)
            .current_dir(&pkg),
    )?;
    let tgz = out.join(tgz.lines().last().unwrap_or_default());
    let _ = std::fs::remove_file(pkg.join("LICENSE"));

    if verify {
        example_app(&tgz)?;
    }
    eprintln!("\nReact Native SDK {version}: {}", tgz.display());
    Ok(())
}

/// A new React Native app with the packed tarball installed, type-checked and
/// built for Android and the iOS simulator: codegen, Kotlin, Swift and
/// Objective-C++ all compiled as a user's app compiles them.
fn example_app(tgz: &Path) -> Result {
    let app = root().join("target/rn-example");
    if !app.join("package.json").exists() {
        step("React Native: example app");
        std::fs::create_dir_all(root().join("target")).map_err(|e| e.to_string())?;
        run(cmd("npx")
            .args([
                "--yes",
                "@react-native-community/cli@latest",
                "init",
                "MathcoreExample",
                "--directory",
            ])
            .arg(&app)
            .args(["--skip-git-init", "--install-pods", "false", "--pm", "npm"]))?;
    }
    run(cmd("npm").args(["install", "--no-audit", "--no-fund"]).arg(tgz).current_dir(&app))?;
    write(&app.join("App.tsx"), EXAMPLE_APP)?;

    step("React Native: TypeScript");
    run(cmd("npx").args(["tsc", "--noEmit"]).current_dir(&app))?;

    step("React Native: Android build");
    let sdk = android_sdk().ok_or("Android SDK not found: set ANDROID_HOME")?;
    run(cmd(app.join("android/gradlew").to_str().unwrap())
        .args(["assembleDebug", "--console=plain"])
        .env("GRADLE_OPTS", "-Dorg.gradle.configuration-cache=false")
        .env("ANDROID_HOME", &sdk)
        .env("JAVA_HOME", crate::android::java_home()?)
        .current_dir(app.join("android")))?;

    if cfg!(target_os = "macos") {
        step("React Native: iOS build");
        run(cmd("bundle").args(["install", "--quiet"]).current_dir(&app))?;
        run(cmd("bundle").args(["exec", "pod", "install"]).current_dir(app.join("ios")))?;
        run(cmd("xcodebuild")
            .args(["-quiet", "-workspace", "MathcoreExample.xcworkspace", "-scheme", "MathcoreExample"])
            .args([
                "-configuration",
                "Debug",
                "-sdk",
                "iphonesimulator",
                "-destination",
                "generic/platform=iOS Simulator",
            ])
            .args(["-derivedDataPath", "build", "CODE_SIGNING_ALLOWED=NO", "build"])
            .current_dir(app.join("ios")))?;
    }
    Ok(())
}

const EXAMPLE_APP: &str = r#"import { useState } from 'react';
import { ScrollView, Text, StyleSheet, View } from 'react-native';
import { MathText, speech, asciimathToTex, SpeechVerbosity } from 'react-native-mathcore';

export default function App() {
  const [tapped, setTapped] = useState('');
  return (
    <View style={styles.root}>
      <ScrollView contentContainerStyle={styles.content}>
        <MathText latex="x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}" fontSize={22} onTap={(_, src) => setTapped(src)} />
        <MathText latex="\ce{2H2 + O2 -> 2H2O}" fontSize={20} />
        <MathText latex="sum_(i=1)^n i^2 = (n(n+1)(2n+1))/6" asciimath fontSize={20} />
        <MathText
          latex="f(x) = a_0 + a_1 x + a_2 x^2 + a_3 x^3 + a_4 x^4 + a_5 x^5 + a_6 x^6 + a_7 x^7"
          fontSize={20}
          speechVerbosity={SpeechVerbosity.Verbose}
        />
        <Text>Tapped: {tapped}</Text>
        <Text>{speech('x^2 + y^2 = z^2')}</Text>
        <Text>{asciimathToTex('sqrt(x)/2')}</Text>
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, paddingTop: 60 },
  content: { padding: 16, gap: 12 },
});
"#;
