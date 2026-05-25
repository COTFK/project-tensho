use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GIT_HASH");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=VERCEL_GIT_COMMIT_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    let git_hash = std::env::var("GIT_HASH")
        .ok()
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .or_else(|| std::env::var("VERCEL_GIT_COMMIT_SHA").ok())
        .or_else(short_git_hash)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
}

fn short_git_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let hash = String::from_utf8(output.stdout).ok()?;
    let hash = hash.trim();

    if hash.is_empty() {
        None
    } else {
        Some(hash.to_string())
    }
}