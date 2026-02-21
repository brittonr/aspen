//! `cargo aspen-plugin` — scaffolding and tooling for Aspen WASM plugins.

mod build;
mod check;
mod init;
mod keygen;
mod sign;
mod templates;
mod verify;

use clap::Parser;
use clap::Subcommand;

/// Cargo subcommand for Aspen WASM plugin development.
///
/// When invoked as `cargo aspen-plugin <cmd>`, cargo passes "aspen-plugin"
/// as the first argument, so we wrap in a top-level enum.
#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    /// Aspen WASM plugin development tools.
    #[command(name = "aspen-plugin")]
    AspenPlugin(AspenPluginArgs),
}

#[derive(Parser)]
struct AspenPluginArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new plugin project from a template.
    Init {
        /// Plugin name (kebab-case).
        name: String,
        /// Template to use.
        #[arg(long, default_value = "basic")]
        template: String,
        /// Plugin description.
        #[arg(long, default_value = "An Aspen WASM plugin")]
        description: String,
        /// Dispatch priority (900–999).
        #[arg(long, default_value_t = 950)]
        priority: u32,
        /// Output directory (defaults to ./<name>).
        #[arg(long)]
        output: Option<String>,
    },

    /// Build the plugin for wasm32-unknown-unknown.
    Build {
        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },

    /// Validate plugin.json and Cargo.toml.
    Check,

    /// Sign the built WASM binary with an Ed25519 key.
    Sign {
        /// Path to Ed25519 secret key file (hex-encoded).
        #[arg(long)]
        key: String,
    },

    /// Verify the WASM binary matches the signature in plugin.json.
    Verify {
        /// Expected author public key (hex). If omitted, uses key from plugin.json.
        #[arg(long)]
        key: Option<String>,
    },

    /// Generate a new Ed25519 keypair for plugin signing.
    Keygen {
        /// Output path for the secret key.
        #[arg(long)]
        output: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let Cargo::AspenPlugin(args) = Cargo::parse();

    match args.command {
        Command::Init {
            name,
            template,
            description,
            priority,
            output,
        } => init::run(&name, &template, &description, priority, output.as_deref()),
        Command::Build { release } => build::run(release),
        Command::Check => check::run(),
        Command::Sign { key } => sign::run(&key),
        Command::Verify { key } => verify::run(key.as_deref()),
        Command::Keygen { output } => keygen::run(output.as_deref()),
    }
}
