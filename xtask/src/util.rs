//! Shared helpers: running tools, locating SDKs, reading the version.

use std::path::{Path, PathBuf};
use std::process::Command;

pub type Result<T = ()> = std::result::Result<T, String>;

/// The repository root (the parent of `xtask/`).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Where packaged SDKs go.
pub fn dist() -> PathBuf {
    root().join("dist")
}

/// The single version every SDK carries: `[workspace.package] version`.
pub fn version() -> String {
    let toml = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let section = toml.split("[workspace.package]").nth(1).expect("[workspace.package]");
    section
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("version")
                .map(|v| v.trim().trim_start_matches('=').trim().trim_matches('"').to_string())
        })
        .expect("workspace version")
}

pub fn step(title: &str) {
    eprintln!("\n\x1b[1;36m==> {title}\x1b[0m");
}

/// A command with cargo's and the user's tool directories on PATH, so tools
/// installed with `cargo install` and Homebrew's rustup are found.
pub fn cmd(program: &str) -> Command {
    let mut c = Command::new(program);
    let home = std::env::var("HOME").unwrap_or_default();
    let path = std::env::var("PATH").unwrap_or_default();
    c.env("PATH", format!("{home}/.cargo/bin:/opt/homebrew/opt/rustup/bin:{path}"));
    c.current_dir(root());
    c
}

/// Runs a command, echoing it, failing with its exit status.
pub fn run(c: &mut Command) -> Result {
    eprintln!("\x1b[2m$ {}\x1b[0m", describe(c));
    let status = c
        .status()
        .map_err(|e| format!("cannot run {}: {e}", c.get_program().to_string_lossy()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{}` failed with {status}", describe(c)))
    }
}

/// Runs a command and returns its standard output.
pub fn output(c: &mut Command) -> Result<String> {
    let out = c
        .output()
        .map_err(|e| format!("cannot run {}: {e}", c.get_program().to_string_lossy()))?;
    if !out.status.success() {
        return Err(format!("`{}` failed: {}", describe(c), String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn describe(c: &Command) -> String {
    std::iter::once(c.get_program().to_string_lossy().to_string())
        .chain(c.get_args().map(|a| a.to_string_lossy().to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn have(program: &str) -> bool {
    cmd("sh")
        .args(["-c", &format!("command -v {program}")])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Fails with an installation hint when a tool is missing.
pub fn require(program: &str, hint: &str) -> Result {
    if have(program) {
        Ok(())
    } else {
        Err(format!("`{program}` not found: {hint}"))
    }
}

pub fn rustup_targets(targets: &[&str]) -> Result {
    let mut c = cmd("rustup");
    c.args(["target", "add"]).args(targets);
    output(&mut c).map(|_| ())
}

/// `cargo build --profile sdk` for a crate and target.
pub fn cargo_sdk_build(package: &str, target: Option<&str>) -> Result {
    let mut c = cmd("cargo");
    c.args(["build", "--profile", "sdk", "-p", package]);
    if let Some(t) = target {
        c.args(["--target", t]);
    }
    run(&mut c)
}

/// Where `cargo_sdk_build` put its artifacts.
pub fn sdk_out(target: Option<&str>) -> PathBuf {
    match target {
        Some(t) => root().join("target").join(t).join("sdk"),
        None => root().join("target").join("sdk"),
    }
}

pub fn host_target() -> String {
    let out = output(cmd("rustc").arg("-vV")).unwrap_or_default();
    out.lines().find_map(|l| l.strip_prefix("host: ")).unwrap_or("unknown").to_string()
}

/// The Android SDK: `ANDROID_HOME`, `ANDROID_SDK_ROOT`, or the usual places.
pub fn android_sdk() -> Option<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(PathBuf::from)
        .chain([format!("{home}/Library/Android/sdk"), format!("{home}/Android/Sdk")].map(PathBuf::from))
        .find(|p| p.join("platform-tools").exists() || p.join("ndk").exists())
}

/// The newest NDK under the SDK, unless `ANDROID_NDK_HOME` says otherwise.
pub fn android_ndk(sdk: &Path) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ANDROID_NDK_HOME") {
        return Some(PathBuf::from(p));
    }
    let mut versions: Vec<PathBuf> = std::fs::read_dir(sdk.join("ndk"))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    versions.sort_by_key(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>())
            .unwrap_or_default()
    });
    versions.pop()
}

pub fn reset_dir(p: &Path) -> Result {
    if p.exists() {
        std::fs::remove_dir_all(p).map_err(|e| format!("cannot clear {}: {e}", p.display()))?;
    }
    std::fs::create_dir_all(p).map_err(|e| format!("cannot create {}: {e}", p.display()))
}

pub fn copy(from: &Path, to: &Path) -> Result {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))
}

pub fn copy_dir(from: &Path, to: &Path) -> Result {
    run(cmd("mkdir").arg("-p").arg(to))?;
    run(cmd("cp").arg("-R").arg(format!("{}/.", from.display())).arg(to))
}

pub fn write(path: &Path, content: &str) -> Result {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, content).map_err(|e| format!("write {}: {e}", path.display()))
}

pub fn sha256(path: &Path) -> Result<String> {
    let out = if have("shasum") {
        output(cmd("shasum").args(["-a", "256"]).arg(path))?
    } else {
        output(cmd("sha256sum").arg(path))?
    };
    Ok(out.split_whitespace().next().unwrap_or_default().to_string())
}
