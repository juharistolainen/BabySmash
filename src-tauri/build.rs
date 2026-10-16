use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    emit_build_info();
    tauri_build::build()
}

/// Compile-time facts for the About dialog, read back with `env!` in lib.rs.
fn emit_build_info() {
    let build_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let rustc_version = command_output(&rustc, &["--version"])
        .map(|v| v.trim_start_matches("rustc ").to_string())
        .unwrap_or_else(|| "unknown".into());
    let commit = command_output("git", &["rev-parse", "--short", "HEAD"]).unwrap_or_default();

    println!("cargo:rustc-env=BABYSMASH_BUILD_TIME={build_time}");
    println!("cargo:rustc-env=BABYSMASH_RUSTC_VERSION={rustc_version}");
    println!("cargo:rustc-env=BABYSMASH_GIT_COMMIT={commit}");
    println!(
        "cargo:rustc-env=BABYSMASH_TARGET={}",
        std::env::var("TARGET").unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=BABYSMASH_PROFILE={}",
        std::env::var("PROFILE").unwrap_or_default()
    );

    // Without these, Cargo only reruns this script when tauri-build's inputs
    // change, and the timestamp/commit would go stale. logs/HEAD moves on every
    // commit and checkout. Skip missing paths: Cargo would rerun on every build.
    for path in ["src", "../.git/logs/HEAD"] {
        if Path::new(path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}
