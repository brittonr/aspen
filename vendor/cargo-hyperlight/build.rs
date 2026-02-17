use std::process::Command;

fn main() {
    let git_hash = Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .output()
        .map(|output| {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                "unknown".to_string()
            }
        })
        .unwrap_or_else(|_| "unknown".to_string());

    let git_date = Command::new("git")
        .args(["log", "-1", "--format=%cs"])
        .output()
        .map(|output| {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                "unknown".to_string()
            }
        })
        .unwrap_or_else(|_| "unknown".to_string());

    // Make the git info available to the program
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=GIT_DATE={}", git_date);

    // Re-run build script if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
