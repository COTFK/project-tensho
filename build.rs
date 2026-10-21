use std::io::ErrorKind;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.env");
    load_dotenv();

    println!("cargo:rerun-if-env-changed=GIT_HASH");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=VERCEL_GIT_COMMIT_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    let git_hash = std::env::var("GIT_HASH")
        .ok()
        .or_else(|| std::env::var("GITHUB_SHA").ok().and_then(shorten_hash))
        .or_else(|| {
            std::env::var("VERCEL_GIT_COMMIT_SHA")
                .ok()
                .and_then(shorten_hash)
        })
        .or_else(short_git_hash)
        .unwrap_or_else(|| "unknown".to_string());

    let courtier_url = std::env::var("COURTIER_API_URL").expect("COURTIER_API_URL should be set");

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=COURTIER_API_URL={courtier_url}");
}

fn load_dotenv() {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(err)) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => panic!("failed to load .env: {err}"),
    }
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

fn shorten_hash(hash: String) -> Option<String> {
    let hash = hash.trim();

    if hash.is_empty() {
        None
    } else {
        Some(hash.chars().take(7).collect())
    }
}
