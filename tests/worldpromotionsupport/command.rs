use std::path::Path;
use std::process::Command;

pub fn run_promote_output(
    root: &Path,
    request_path: &Path,
    authority_path: &Path,
    closure_path: &Path,
    signer_hex: &str,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_molten"))
        .args([
            "world-promotion",
            "promote",
            "--state-root",
            root.to_str().expect("path"),
            "--request",
            request_path.to_str().expect("path"),
            "--authority",
            authority_path.to_str().expect("path"),
            "--closure",
            closure_path.to_str().expect("path"),
            "--trusted-signer",
            signer_hex,
        ])
        .output()
        .expect("command")
}

pub fn run_promote(
    root: &Path,
    request_path: &Path,
    authority_path: &Path,
    closure_path: &Path,
    signer_hex: &str,
) -> String {
    let output = run_promote_output(root, request_path, authority_path, closure_path, signer_hex);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs one `molten world-promotion` subcommand.
pub fn run_output(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_molten"))
        .arg("world-promotion")
        .args(arguments)
        .output()
        .expect("command")
}

/// Runs one `molten world-promotion` subcommand that must succeed and returns stdout.
pub fn run_success(arguments: &[&str]) -> String {
    let output = run_output(arguments);
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}
