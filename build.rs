use std::process::Command;
use std::process::Output;

fn read_command_output(program: &str, args: &[&str]) -> Option<String> {
    let output: Output = Command::new(program).args(args).output().ok()?;
    String::from_utf8(output.stdout).ok().map(|text| text.trim().to_string())
}

fn main() {
    // Get git commit hash
    let git_hash =
        read_command_output("git", &["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Also get build timestamp using the date command
    let timestamp = read_command_output("date", &["+%Y-%m-%d %H:%M:%S UTC"]).unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_TIME={}", timestamp);

    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
