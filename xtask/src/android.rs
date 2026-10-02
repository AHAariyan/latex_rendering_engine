//! Android SDK: the `dev.mathcore:mathcore-android` AAR (Compose `MathText`,
//! `MathView`, TalkBack navigation) with the engine for every ABI, published
//! to a local Maven repository under dist/android/maven.

use crate::util::*;
use std::path::PathBuf;

const ABIS: &[(&str, &str)] = &[
    ("arm64-v8a", "aarch64-linux-android"),
    ("armeabi-v7a", "armv7-linux-androideabi"),
    ("x86_64", "x86_64-linux-android"),
];

/// Lowest API level the library supports (mathview's minSdk).
const MIN_SDK: &str = "24";

pub fn build(verify: bool) -> Result {
    let sdk = android_sdk().ok_or("Android SDK not found: set ANDROID_HOME")?;
    let ndk = android_ndk(&sdk).ok_or("Android NDK not found: install one with sdkmanager \"ndk;28.2.13676358\"")?;
    require("cargo-ndk", "cargo install cargo-ndk")?;
    let java_home = java_home()?;
    let version = version();
    let out = dist().join("android");
    reset_dir(&out)?;
    let project = root().join("platforms/android");

    step("Android: engine for every ABI");
    rustup_targets(&ABIS.iter().map(|a| a.1).collect::<Vec<_>>())?;
    let jni_libs = project.join("mathview/src/main/jniLibs");
    reset_dir(&jni_libs)?;
    let mut c = cmd("cargo");
    c.arg("ndk").env("ANDROID_HOME", &sdk).env("ANDROID_NDK_HOME", &ndk);
    for (abi, _) in ABIS {
        c.args(["-t", abi]);
    }
    c.args(["--platform", MIN_SDK, "-o"])
        .arg(&jni_libs)
        .args(["build", "--profile", "sdk", "-p", "mathjni"]);
    run(&mut c)?;
    for (abi, _) in ABIS {
        let so = jni_libs.join(abi).join("libmathcore_android.so");
        if !so.exists() {
            return Err(format!("missing {}", so.display()));
        }
    }

    let mut tasks = vec![":mathview:publishReleasePublicationToDistRepository"];
    let mut extra: Vec<String> = Vec::new();
    if verify {
        step("Android: engine for this machine, for JVM unit tests");
        cargo_sdk_build("mathjni", None)?;
        extra.push(format!("-PhostLibDir={}", sdk_out(None).display()));
        tasks.extend([":mathview:testDebugUnitTest", ":mathview:lintRelease", ":app:assembleDebug"]);
    }

    step("Android: AAR, tests, lint, demo app");
    write(&project.join("local.properties"), &format!("sdk.dir={}\n", sdk.display()))?;
    let mut g = cmd(project.join("gradlew").to_str().unwrap());
    g.current_dir(&project)
        .env("JAVA_HOME", &java_home)
        .env("ANDROID_HOME", &sdk)
        .args(&tasks)
        .args(["-PskipCargo", "--console=plain", "--warning-mode=summary"])
        .arg(format!("-PmathcoreVersion={version}"))
        .arg(format!("-PdistRepo={}", out.join("maven").display()))
        .args(&extra);
    run(&mut g)?;

    let aar = out.join(format!(
        "maven/dev/mathcore/mathcore-android/{version}/mathcore-android-{version}.aar"
    ));
    if !aar.exists() {
        return Err(format!("expected {}", aar.display()));
    }
    copy(&aar, &out.join(format!("mathcore-android-{version}.aar")))?;
    eprintln!(
        "\nAndroid SDK {version}: dev.mathcore:mathcore-android in {}",
        out.join("maven").display()
    );
    Ok(())
}

/// A JDK of version 17 or later for Gradle: `JAVA_HOME`, else the newest the
/// system knows of.
fn java_home() -> Result<PathBuf> {
    if let Ok(h) = std::env::var("JAVA_HOME") {
        return Ok(PathBuf::from(h));
    }
    if cfg!(target_os = "macos") {
        for v in ["21", "17"] {
            if let Ok(p) = output(cmd("/usr/libexec/java_home").args(["-v", v])) {
                return Ok(PathBuf::from(p));
            }
        }
    }
    Err("no JDK 17+ found: set JAVA_HOME".into())
}
