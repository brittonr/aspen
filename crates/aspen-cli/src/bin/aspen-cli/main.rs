//! Aspen CLI - Command-line interface for Aspen distributed system.
//!
//! Non-interactive CLI tool that provides access to all Aspen cluster operations
//! including cluster management, key-value store, SQL queries, distributed
//! coordination primitives, and more.
//!
//! # Usage
//!
//! ```bash
//! # Connect and check cluster status
//! aspen-cli --ticket aspen... cluster status
//!
//! # Write and read a key
//! aspen-cli --ticket aspen... kv set mykey "hello world"
//! aspen-cli --ticket aspen... kv get mykey
//!
//! # JSON output for scripting
//! aspen-cli --ticket aspen... --json cluster metrics | jq '.current_leader'
//!
//! # Using environment variables
//! export ASPEN_TICKET="aspen..."
//! aspen-cli kv scan --limit 100
//! ```
//!
//! # Tiger Style
//!
//! - Explicit error handling with anyhow
//! - Bounded timeouts on all operations
//! - Fail-fast on invalid arguments
//! - Clean resource cleanup

mod cli;
mod client;
mod commands;
mod output;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::Cli;

/// Initialize tracing subscriber with environment-based filtering.
///
/// - `quiet`: Suppress all logging output (for scripting)
/// - `verbose`: Enable debug-level logging
fn init_tracing(quiet: bool, verbose: bool) {
    let filter = if quiet {
        EnvFilter::new("off")
    } else if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.global.quiet, cli.global.verbose);

    // Display version info with git hash when not in quiet mode
    if !cli.global.quiet {
        let git_hash = env!("GIT_HASH", "GIT_HASH not set");
        let build_time = env!("BUILD_TIME", "BUILD_TIME not set");
        eprintln!(
            "aspen-cli v{} ({}) built at {}",
            env!("CARGO_PKG_VERSION"),
            git_hash,
            build_time
        );
    }

    cli.run().await
}
