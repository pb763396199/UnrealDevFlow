use chrono::Utc;
use std::process::Command;

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    let package_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into());
    let git_sha =
        git_output(&["rev-parse", "--short=9", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git_output(&["status", "--porcelain"])
        .map(|status| if status.is_empty() { "" } else { "-dirty" })
        .unwrap_or("");
    let build_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    println!("cargo:rustc-env=UDF_BUILD_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=UDF_BUILD_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=UDF_BUILD_TIME_UTC={build_time}");
    println!(
        "cargo:rustc-env=UDF_VERSION_LONG={package_version} (git {git_sha}{dirty}, built {build_time})"
    );
}
