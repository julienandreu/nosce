use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/");

    let commit_hash = git(&["rev-parse", "HEAD"]);
    let commit_short = git(&["rev-parse", "--short", "HEAD"]);
    let commit_date = git(&["log", "-1", "--format=%cs"]);
    let git_describe = git(&["describe", "--tags", "--always", "--dirty"]);
    let build_timestamp = chrono::Utc::now().to_rfc3339();
    let target = std::env::var("TARGET").unwrap_or_default();

    println!("cargo:rustc-env=NOSCE_COMMIT_HASH={commit_hash}");
    println!("cargo:rustc-env=NOSCE_COMMIT_SHORT={commit_short}");
    println!("cargo:rustc-env=NOSCE_COMMIT_DATE={commit_date}");
    println!("cargo:rustc-env=NOSCE_GIT_DESCRIBE={git_describe}");
    println!("cargo:rustc-env=NOSCE_BUILD_TIMESTAMP={build_timestamp}");
    println!("cargo:rustc-env=NOSCE_TARGET={target}");
}

fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}
