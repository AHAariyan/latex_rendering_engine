//! Flutter SDK: `mathcore_dart` (pure dart:ffi) and `mathcore_flutter` (the
//! `MathText` widget) with the engine for Android, iOS and macOS, as
//! publishable pub packages under dist/flutter.

use crate::util::*;
use std::path::Path;

const ANDROID_ABIS: &[(&str, &str)] = &[
    ("arm64-v8a", "aarch64-linux-android"),
    ("armeabi-v7a", "armv7-linux-androideabi"),
    ("x86_64", "x86_64-linux-android"),
];

pub fn build(verify: bool) -> Result {
    require("flutter", "install Flutter: https://docs.flutter.dev/get-started")?;
    let version = version();
    let out = dist().join("flutter");
    reset_dir(&out)?;
    let plugin = root().join("platforms/flutter/mathcore_flutter");
    let dart_pkg = root().join("platforms/dart/mathcore_dart");

    step("Flutter: engine for Android");
    let sdk = android_sdk().ok_or("Android SDK not found: set ANDROID_HOME")?;
    let ndk = android_ndk(&sdk).ok_or("Android NDK not found")?;
    rustup_targets(&ANDROID_ABIS.iter().map(|a| a.1).collect::<Vec<_>>())?;
    let jni = plugin.join("android/src/main/jniLibs");
    reset_dir(&jni)?;
    let mut c = cmd("cargo");
    c.arg("ndk").env("ANDROID_HOME", &sdk).env("ANDROID_NDK_HOME", &ndk);
    for (abi, _) in ANDROID_ABIS {
        c.args(["-t", abi]);
    }
    c.args(["--platform", "24", "-o"])
        .arg(&jni)
        .args(["build", "--profile", "sdk", "-p", "mathffi"]);
    run(&mut c)?;

    if cfg!(target_os = "macos") {
        step("Flutter: engine for iOS and macOS as a dynamic framework");
        for (platform, which) in [
            ("ios", crate::apple::ApplePlatform::Ios),
            ("macos", crate::apple::ApplePlatform::MacOs),
        ] {
            let xcf = crate::apple::dynamic_xcframework(&version, which)?;
            // One copy serves both Swift Package Manager and CocoaPods.
            let dest = plugin.join(platform).join("mathcore_flutter/MathCoreFFI.xcframework");
            if dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
            }
            copy_dir(&xcf, &dest)?;
        }
    }

    if verify {
        step("Flutter: host engine for tests");
        cargo_sdk_build("mathffi", None)?;
        let lib = sdk_out(None).join(host_lib_name());
        let lib = lib.to_str().unwrap().to_string();

        step("Flutter: mathcore_dart analyze and test");
        run(cmd("dart").args(["pub", "get"]).current_dir(&dart_pkg))?;
        run(cmd("dart").args(["analyze", "--fatal-infos"]).current_dir(&dart_pkg))?;
        run(cmd("dart").arg("test").env("MATHCORE_LIB", &lib).current_dir(&dart_pkg))?;

        step("Flutter: mathcore_flutter analyze and widget tests");
        run(cmd("flutter").args(["pub", "get"]).current_dir(&plugin))?;
        run(cmd("flutter").args(["analyze", "--fatal-infos"]).current_dir(&plugin))?;
        run(cmd("flutter").arg("test").env("MATHCORE_LIB", &lib).current_dir(&plugin))?;

        example_app(&plugin)?;
    }

    step("Flutter: publishable packages");
    // Staged outside the repository: pub honours the repository's .gitignore,
    // which ignores dist/.
    let stage = std::env::temp_dir().join(format!("mathcore-pub-{version}"));
    reset_dir(&stage)?;
    let dart_out = stage.join("mathcore_dart");
    let plugin_out = stage.join("mathcore_flutter");
    copy_package(&dart_pkg, &dart_out)?;
    copy_package(&plugin, &plugin_out)?;
    set_pubspec_version(&dart_out, &version, None)?;
    set_pubspec_version(&plugin_out, &version, Some(&version))?;
    set_podspec_version(&plugin_out, &version)?;
    for p in [&dart_out, &plugin_out] {
        copy(&root().join("LICENSE"), &p.join("LICENSE"))?;
        write(
            &p.join("CHANGELOG.md"),
            &format!("## {version}\n\n- Release {version} of the mathcore engine.\n"),
        )?;
    }
    run(cmd("dart").args(["pub", "publish", "--dry-run"]).current_dir(&dart_out))?;
    // mathcore_flutter depends on the mathcore_dart of the same release,
    // which pub resolves from pub.dev once that is published; for the dry
    // run, point it at the copy beside it.
    write(
        &plugin_out.join("pubspec_overrides.yaml"),
        "dependency_overrides:\n  mathcore_dart:\n    path: ../mathcore_dart\n",
    )?;
    let dry = cmd("flutter")
        .args(["pub", "publish", "--dry-run"])
        .current_dir(&plugin_out)
        .output();
    let _ = std::fs::remove_file(plugin_out.join("pubspec_overrides.yaml"));
    let dry = dry.map_err(|e| e.to_string())?;
    let report = String::from_utf8_lossy(&dry.stdout).to_string() + &String::from_utf8_lossy(&dry.stderr);
    // The override itself is the one expected warning.
    let problems: Vec<&str> = report
        .lines()
        .filter(|l| l.contains("* ") && !l.contains("pubspec_overrides") && !l.contains("dependency_overrides"))
        .collect();
    if !problems.is_empty() {
        return Err(format!("pub validation found problems:\n{}", problems.join("\n")));
    }
    copy_dir(&stage, &out)?;
    eprintln!(
        "\nFlutter SDK {version}: {} and {}",
        out.join("mathcore_dart").display(),
        out.join("mathcore_flutter").display()
    );
    Ok(())
}

fn host_lib_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "libmathcore_ffi.dylib"
    } else if cfg!(target_os = "windows") {
        "mathcore_ffi.dll"
    } else {
        "libmathcore_ffi.so"
    }
}

/// An app that depends on the plugin, built for Android, iOS and macOS, and
/// an integration test run inside the macOS app: the only check that the
/// framework is found and loads in a real application.
fn example_app(plugin: &Path) -> Result {
    step("Flutter: example app");
    let app = root().join("target/flutter-example");
    if !app.join("pubspec.yaml").exists() {
        std::fs::create_dir_all(root().join("target")).map_err(|e| e.to_string())?;
        run(cmd("flutter")
            .args([
                "create",
                "--org",
                "dev.mathcore",
                "--project-name",
                "mathcore_example",
                "--platforms=android,ios,macos",
            ])
            .arg(&app))?;
    }
    copy(&plugin.join("example/main.dart"), &app.join("lib/main.dart"))?;
    write(
        &app.join("pubspec_overrides.yaml"),
        &format!(
            "dependency_overrides:\n  mathcore_flutter:\n    path: {}\n  mathcore_dart:\n    path: {}\n",
            plugin.display(),
            root().join("platforms/dart/mathcore_dart").display()
        ),
    )?;
    run(cmd("flutter")
        .args(["pub", "add", "mathcore_flutter", "dev:integration_test:{\"sdk\":\"flutter\"}"])
        .current_dir(&app))?;
    write(&app.join("integration_test/app_test.dart"), INTEGRATION_TEST)?;
    let _ = std::fs::remove_file(app.join("test/widget_test.dart"));
    // Flutter's Gradle plugin does not support the configuration cache, which a
    // user-level gradle.properties may turn on for every build.
    run(cmd("flutter")
        .args(["build", "apk", "--debug"])
        .env("GRADLE_OPTS", "-Dorg.gradle.configuration-cache=false")
        .current_dir(&app))?;
    if cfg!(target_os = "macos") {
        run(cmd("flutter").args(["build", "ios", "--simulator", "--debug"]).current_dir(&app))?;
        run(cmd("flutter")
            .args(["test", "integration_test/app_test.dart", "-d", "macos"])
            .current_dir(&app))?;
    }
    Ok(())
}

const INTEGRATION_TEST: &str = r#"import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:mathcore_flutter/mathcore_flutter.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('the engine loads inside the app and draws', (tester) async {
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(body: Column(children: [
        MathText(r'x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}', fontSize: 24),
        MathText(r'\ce{2H2 + O2 -> 2H2O}', fontSize: 24),
      ])),
    ));
    final sizes = tester.widgetList<CustomPaint>(find.byType(CustomPaint)).map((p) => p.size).where((s) => s.width > 50);
    expect(sizes.length, greaterThanOrEqualTo(2));
    expect(MathEngine.speech('x^2'), 'x squared');
  });
}
"#;

/// Copies a package without its build state.
fn copy_package(from: &Path, to: &Path) -> Result {
    reset_dir(to)?;
    let mut c = cmd("rsync");
    c.args([
        "-a",
        "--exclude",
        ".dart_tool",
        "--exclude",
        "build",
        "--exclude",
        "pubspec.lock",
        "--exclude",
        ".flutter-plugins*",
    ])
    .args(["--exclude", "pubspec_overrides.yaml"]);
    run(c.arg(format!("{}/", from.display())).arg(to))
}

fn set_pubspec_version(pkg: &Path, version: &str, dart_dep: Option<&str>) -> Result {
    let path = pkg.join("pubspec.yaml");
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(l) = lines.next() {
        if l.starts_with("version:") {
            out.push(format!("version: {version}"));
        } else if l.trim_start().starts_with("mathcore_dart:") && dart_dep.is_some() {
            // A `path:` form on the next line becomes a version constraint too.
            if lines.peek().is_some_and(|n| n.trim().starts_with("path:")) {
                lines.next();
            }
            out.push(format!("  mathcore_dart: ^{}", dart_dep.unwrap()));
        } else {
            out.push(l.to_string());
        }
    }
    write(&path, &(out.join("\n") + "\n"))
}

fn set_podspec_version(pkg: &Path, version: &str) -> Result {
    for platform in ["ios", "macos"] {
        let path = pkg.join(platform).join("mathcore_flutter.podspec");
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let text = text
            .lines()
            .map(|l| {
                if l.trim_start().starts_with("s.version") {
                    format!("  s.version          = '{version}'")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        write(&path, &(text + "\n"))?;
    }
    Ok(())
}
