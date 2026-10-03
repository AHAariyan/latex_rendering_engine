//! The SDK pipeline: one command builds, verifies and packages the engine
//! for a platform, from the same source, at the same version.
//!
//!     cargo xtask sdk <platform>... [--no-verify]
//!     cargo xtask version [<new-version>] [--check]
//!     cargo xtask ci
//!
//! Platforms: c, apple, android, flutter, web, react-native, all.
//! Output goes to dist/<platform>/.

mod android;
mod apple;
mod c;
mod flutter;
mod react_native;
mod util;
mod version;
mod web;

use util::*;

const PLATFORMS: &[&str] = &["c", "apple", "android", "flutter", "web", "react-native"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("sdk") => sdk(&args[1..]),
        Some("ci") => ci(),
        Some("version") => version::command(&args[1..]),
        Some("-h" | "--help") | None => {
            eprintln!("{}", HELP);
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\n{HELP}")),
    };
    if let Err(e) = result {
        eprintln!("\n\x1b[1;31merror:\x1b[0m {e}");
        std::process::exit(1);
    }
}

const HELP: &str = "cargo xtask sdk <platform>... [--no-verify]   build, test and package SDKs into dist/
    platforms: c, apple, android, flutter, web, react-native, all
cargo xtask version [<new>] [--check]          show, set or check the version every SDK carries
cargo xtask ci                                 formatting, lints, tests and the corpus gate";

fn sdk(args: &[String]) -> Result {
    let verify = !args.iter().any(|a| a == "--no-verify");
    let mut platforms: Vec<&str> = args.iter().map(String::as_str).filter(|a| !a.starts_with("--")).collect();
    if platforms.is_empty() {
        return Err(format!("name a platform: {}, or all", PLATFORMS.join(", ")));
    }
    if platforms.contains(&"all") {
        platforms = PLATFORMS.to_vec();
    }
    for p in &platforms {
        match *p {
            "c" => c::build(verify)?,
            "apple" | "ios" => apple::build(verify)?,
            "web" => web::build(verify)?,
            "android" => android::build(verify)?,
            "flutter" => flutter::build(verify)?,
            "react-native" | "rn" => react_native::build(verify)?,
            other => return Err(format!("unknown platform `{other}`; one of {}", PLATFORMS.join(", "))),
        }
    }
    checksums()?;
    eprintln!("\n\x1b[1;32mdone:\x1b[0m {} in {}", platforms.join(", "), dist().display());
    Ok(())
}

/// dist/SHA256SUMS for every release archive, for `shasum -c`.
fn checksums() -> Result {
    let mut lines = Vec::new();
    let mut dirs = vec![dist()];
    while let Some(d) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = p.to_string_lossy().to_string();
            if p.is_dir() && !name.ends_with(".xcframework") && !name.contains("/maven") && !name.contains("/package") {
                dirs.push(p);
            } else if [".tar.gz", ".zip", ".tgz", ".aar"].iter().any(|x| name.ends_with(x)) {
                let rel = p.strip_prefix(dist()).unwrap().display().to_string();
                lines.push(format!("{}  {rel}", sha256(&p)?));
            }
        }
    }
    lines.sort_by(|a, b| a[66..].cmp(&b[66..]));
    write(&dist().join("SHA256SUMS"), &(lines.join("\n") + "\n"))
}

fn ci() -> Result {
    step("format");
    run(cmd("cargo").args(["fmt", "--all", "--check"]))?;
    step("lints");
    run(cmd("cargo").args(["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]))?;
    step("versions");
    version::command(&["--check".to_string()])?;
    step("tests");
    run(cmd("cargo").args(["test", "--workspace"]))?;
    run(cmd("cargo").args(["test", "--release", "-p", "mathcore", "--test", "robustness"]))?;
    Ok(())
}
