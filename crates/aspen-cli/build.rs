use std::process::Command;

fn main() {
    // Get git commit hash
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap_or_else(|_| {
            // If git fails, use "unknown"
            std::process::Command::new("echo")
                .arg("unknown")
                .output()
                .unwrap()
        });

    let git_hash = String::from_utf8(output.stdout)
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Also get build timestamp using the date command
    let timestamp_output = Command::new("date")
        .args(&["+%Y-%m-%d %H:%M:%S UTC"])
        .output()
        .unwrap_or_else(|_| {
            std::process::Command::new("echo")
                .arg("unknown")
                .output()
                .unwrap()
        });

    let timestamp = String::from_utf8(timestamp_output.stdout)
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    println!("cargo:rustc-env=BUILD_TIME={}", timestamp);

    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}