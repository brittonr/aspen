use std::env;

use cargo_hyperlight::cargo;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_HASH: &str = env!("GIT_HASH");
const GIT_DATE: &str = env!("GIT_DATE");

fn main() {
    if env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("cargo-hyperlight {} ({} {})", VERSION, GIT_HASH, GIT_DATE);
        return;
    }

    let args = env::args_os().enumerate().filter_map(|(i, arg)| {
        // skip the binary name and the "hyperlight" subcommand if present
        if i == 0 || (i == 1 && arg == "hyperlight") {
            None
        } else {
            Some(arg)
        }
    });

    cargo()
        .expect("Failed to create cargo command")
        .args(args)
        .status()
        .expect("Failed to execute cargo")
}
