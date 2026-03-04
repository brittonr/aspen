//! CLI command definitions for aspen-sops.

use std::path::PathBuf;

#[cfg(feature = "keyservice")]
use aspen_sops::constants::DEFAULT_SOCKET_PATH;
use aspen_sops::constants::DEFAULT_TRANSIT_KEY;
use aspen_sops::constants::DEFAULT_TRANSIT_MOUNT;
use clap::Parser;
use clap::Subcommand;

/// SOPS backend using Aspen Transit for key management.
///
/// Encrypt/decrypt SOPS files using Aspen's distributed Transit engine
/// as the key management backend. Compatible with the SOPS file format.
#[derive(Parser, Debug)]
#[command(name = "aspen-sops", version, about)]
pub struct Cli {
    /// Enable verbose logging.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Encrypt a plaintext file using Aspen Transit.
    Encrypt {
        /// Input file to encrypt.
        file: PathBuf,

        /// Aspen cluster ticket.
        #[arg(long, env = "ASPEN_CLUSTER_TICKET")]
        cluster_ticket: String,

        /// Transit key name for data key encryption.
        #[arg(long, default_value = DEFAULT_TRANSIT_KEY)]
        transit_key: String,

        /// Transit mount point.
        #[arg(long, default_value = DEFAULT_TRANSIT_MOUNT)]
        transit_mount: String,

        /// Also encrypt for age recipients (offline fallback).
        #[arg(long = "age")]
        age_recipients: Vec<String>,

        /// Only encrypt values matching this regex.
        #[arg(long)]
        encrypted_regex: Option<String>,

        /// Encrypt the file in place.
        #[arg(long, short)]
        in_place: bool,
    },

    /// Decrypt a SOPS-encrypted file.
    Decrypt {
        /// Input file to decrypt.
        file: PathBuf,

        /// Aspen cluster ticket (uses metadata ticket if not provided).
        #[arg(long, env = "ASPEN_CLUSTER_TICKET")]
        cluster_ticket: Option<String>,

        /// Output to file instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Extract a single value by dotted path.
        #[arg(long)]
        extract: Option<String>,

        /// Age identity file for fallback decryption.
        #[cfg(feature = "age-fallback")]
        #[arg(long)]
        age_identity: Option<PathBuf>,
    },

    /// Decrypt, edit in $EDITOR, and re-encrypt.
    Edit {
        /// SOPS-encrypted file to edit.
        file: PathBuf,

        /// Aspen cluster ticket.
        #[arg(long, env = "ASPEN_CLUSTER_TICKET")]
        cluster_ticket: String,

        /// Editor command.
        #[arg(long)]
        editor: Option<String>,

        /// Transit key name.
        #[arg(long, default_value = DEFAULT_TRANSIT_KEY)]
        transit_key: String,

        /// Transit mount point.
        #[arg(long, default_value = DEFAULT_TRANSIT_MOUNT)]
        transit_mount: String,
    },

    /// Re-wrap data key with the latest Transit key version.
    Rotate {
        /// SOPS-encrypted file.
        file: PathBuf,

        /// Aspen cluster ticket.
        #[arg(long, env = "ASPEN_CLUSTER_TICKET")]
        cluster_ticket: Option<String>,

        /// Rotate in place.
        #[arg(long, short)]
        in_place: bool,
    },

    /// Add or remove key groups from a SOPS file.
    UpdateKeys {
        /// SOPS-encrypted file.
        file: PathBuf,

        /// Aspen cluster ticket for new Transit recipient.
        #[arg(long, env = "ASPEN_CLUSTER_TICKET")]
        cluster_ticket: Option<String>,

        /// Transit key name for new recipient.
        #[arg(long)]
        transit_key: Option<String>,

        /// Transit mount point.
        #[arg(long, default_value = DEFAULT_TRANSIT_MOUNT)]
        transit_mount: String,

        /// Add age recipient.
        #[arg(long)]
        add_age: Vec<String>,

        /// Remove age recipient by public key.
        #[arg(long)]
        remove_age: Vec<String>,

        /// Update in place.
        #[arg(long, short)]
        in_place: bool,
    },

    /// Start a gRPC key service for Go SOPS compatibility.
    #[cfg(feature = "keyservice")]
    Keyservice {
        /// Aspen cluster ticket.
        #[arg(long, env = "ASPEN_CLUSTER_TICKET")]
        cluster_ticket: String,

        /// Transit key name.
        #[arg(long, default_value = DEFAULT_TRANSIT_KEY)]
        transit_key: String,

        /// Transit mount point.
        #[arg(long, default_value = DEFAULT_TRANSIT_MOUNT)]
        transit_mount: String,

        /// Unix socket path.
        #[arg(long, default_value = DEFAULT_SOCKET_PATH)]
        socket: PathBuf,
    },
}
