//! One version for every SDK. The workspace's `[workspace.package] version`
//! is the source; the manifests that cannot read it carry a copy, which
//! `cargo xtask version <new>` rewrites and `--check` verifies.

use crate::util::*;
use std::path::PathBuf;

/// (file, line prefix, how the version is written after the prefix)
fn copies(v: &str) -> Vec<(PathBuf, &'static str, String)> {
    let r = root();
    vec![
        (r.join("platforms/dart/mathcore_dart/pubspec.yaml"), "version:", format!(" {v}")),
        (
            r.join("platforms/flutter/mathcore_flutter/pubspec.yaml"),
            "version:",
            format!(" {v}"),
        ),
        (
            r.join("platforms/flutter/mathcore_flutter/pubspec.yaml"),
            "  mathcore_dart:",
            format!(" ^{v}"),
        ),
        (
            r.join("platforms/flutter/mathcore_flutter/ios/mathcore_flutter.podspec"),
            "  s.version ",
            format!("         = '{v}'"),
        ),
        (
            r.join("platforms/flutter/mathcore_flutter/macos/mathcore_flutter.podspec"),
            "  s.version ",
            format!("         = '{v}'"),
        ),
        (
            r.join("platforms/react-native/package.json"),
            "  \"version\":",
            format!(" \"{v}\","),
        ),
    ]
}

pub fn command(args: &[String]) -> Result {
    let check = args.iter().any(|a| a == "--check");
    let new = args.iter().find(|a| !a.starts_with("--"));
    let current = version();
    if let Some(new) = new {
        if !is_semver(new) {
            return Err(format!("`{new}` is not a version like 1.2.3 or 1.2.3-beta.1"));
        }
        let cargo = root().join("Cargo.toml");
        let text = std::fs::read_to_string(&cargo).map_err(|e| e.to_string())?;
        let (head, tail) = text.split_once("[workspace.package]").ok_or("no [workspace.package]")?;
        let tail = tail.replacen(&format!("version = \"{current}\""), &format!("version = \"{new}\""), 1);
        write(&cargo, &format!("{head}[workspace.package]{tail}"))?;
        for (file, prefix, value) in copies(new) {
            rewrite(&file, prefix, &value)?;
        }
        eprintln!("version {current} -> {new}");
        return Ok(());
    }
    if check {
        let mut wrong = Vec::new();
        for (file, prefix, value) in copies(&current) {
            let Ok(text) = std::fs::read_to_string(&file) else { continue };
            let expected = format!("{prefix}{value}");
            if !text.lines().any(|l| l == expected) {
                wrong.push(format!("{}: expected `{expected}`", file.strip_prefix(root()).unwrap().display()));
            }
        }
        if !wrong.is_empty() {
            return Err(format!(
                "versions disagree with {current}:\n  {}\nRun `cargo xtask version {current}`.",
                wrong.join("\n  ")
            ));
        }
        eprintln!("every manifest is at {current}");
        return Ok(());
    }
    println!("{current}");
    Ok(())
}

fn rewrite(file: &PathBuf, prefix: &str, value: &str) -> Result {
    let Ok(text) = std::fs::read_to_string(file) else { return Ok(()) };
    let mut found = false;
    let out: Vec<String> = text
        .lines()
        .map(|l| {
            if !found && l.starts_with(prefix) {
                found = true;
                format!("{prefix}{value}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        return Err(format!("{}: no line starting with `{prefix}`", file.display()));
    }
    write(file, &(out.join("\n") + "\n"))
}

fn is_semver(v: &str) -> bool {
    let core = v.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}
