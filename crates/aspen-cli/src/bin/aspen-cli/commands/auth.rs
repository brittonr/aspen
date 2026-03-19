//! Nostr authentication commands.
//!
//! Login with a Nostr identity to attribute forge operations to your npub.
//! The nsec is used locally to sign a challenge — it is never sent to the server.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Subcommand;

use crate::client::AspenClient;

/// Nostr authentication commands.
#[derive(Subcommand)]
pub enum AuthCommand {
    /// Login with your Nostr identity.
    ///
    /// Signs a challenge locally with your nsec to prove npub ownership.
    /// The nsec is never transmitted to the server.
    Login {
        /// Path to a file containing the nsec (hex or bech32).
        /// If not provided, reads from stdin.
        #[arg(long)]
        nsec_file: Option<PathBuf>,
    },

    /// Show current authentication status.
    Status,

    /// Remove stored authentication token.
    Logout,
}

impl AuthCommand {
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            Self::Login { nsec_file } => login(client, nsec_file, json).await,
            Self::Status => status(json).await,
            Self::Logout => logout(json).await,
        }
    }
}

/// Default token storage path.
fn token_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")).join("aspen").join("token")
}

/// Read the stored token, if any.
pub fn read_stored_token() -> Option<String> {
    std::fs::read_to_string(token_path()).ok()
}

async fn login(client: &AspenClient, nsec_file: Option<PathBuf>, json: bool) -> Result<()> {
    // Read nsec
    let nsec_str = match nsec_file {
        Some(path) => {
            std::fs::read_to_string(&path).with_context(|| format!("failed to read nsec from {}", path.display()))?
        }
        None => {
            eprint!("Enter nsec (hex): ");
            std::io::stderr().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            line
        }
    };
    let nsec_hex = nsec_str.trim();

    // Derive npub from nsec
    let secret_key = nostr::SecretKey::from_hex(nsec_hex).context("invalid nsec hex (expected 64 hex chars)")?;
    let keys = nostr::Keys::new(secret_key);
    let npub_hex = keys.public_key().to_hex();

    // Step 1: get challenge
    let resp = client
        .send(ClientRpcRequest::NostrAuthChallenge {
            npub_hex: npub_hex.clone(),
        })
        .await
        .context("auth challenge request failed")?;

    let (challenge_id, challenge_hex) = match resp {
        ClientRpcResponse::NostrAuthChallengeResult {
            challenge_id,
            challenge_hex,
        } => (challenge_id, challenge_hex),
        _ => anyhow::bail!("unexpected response to auth challenge"),
    };

    // Step 2: sign challenge locally
    let challenge_bytes = hex::decode(&challenge_hex).context("invalid challenge hex")?;
    let signature_hex = sign_challenge_locally(&keys, &challenge_bytes);

    // Step 3: verify with server
    let resp = client
        .send(ClientRpcRequest::NostrAuthVerify {
            npub_hex: npub_hex.clone(),
            challenge_id,
            signature_hex,
        })
        .await
        .context("auth verify request failed")?;

    let token = match resp {
        ClientRpcResponse::NostrAuthVerifyResult {
            is_success: true,
            token: Some(t),
            ..
        } => t,
        ClientRpcResponse::NostrAuthVerifyResult { error: Some(e), .. } => anyhow::bail!("authentication failed: {e}"),
        _ => anyhow::bail!("unexpected response to auth verify"),
    };

    // Store token
    let path = token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &token).context("failed to write token")?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "ok",
                "npub": npub_hex,
                "token_path": path.display().to_string(),
            })
        );
    } else {
        println!("Logged in as npub:{}", &npub_hex[..12]);
        println!("Token stored at {}", path.display());
    }
    Ok(())
}

async fn status(json: bool) -> Result<()> {
    let token = read_stored_token();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "logged_in": token.is_some(),
                "token_path": token_path().display().to_string(),
            })
        );
    } else if token.is_some() {
        println!("Logged in (token at {})", token_path().display());
    } else {
        println!("Not logged in");
    }
    Ok(())
}

async fn logout(json: bool) -> Result<()> {
    let path = token_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
        if json {
            println!("{{\"status\":\"logged_out\"}}");
        } else {
            println!("Logged out (token removed)");
        }
    } else if json {
        println!("{{\"status\":\"not_logged_in\"}}");
    } else {
        println!("Not logged in");
    }
    Ok(())
}

/// Sign a challenge with a Nostr key using secp256k1 Schnorr (BIP-340).
fn sign_challenge_locally(keys: &nostr::Keys, challenge_bytes: &[u8]) -> String {
    use nostr::secp256k1::Message;
    use nostr::secp256k1::Secp256k1;

    let msg_hash = bitcoin_hashes::sha256::Hash::hash(challenge_bytes);
    let msg = Message::from_digest(msg_hash.to_byte_array());
    let secp = Secp256k1::new();
    let keypair = keys.secret_key().keypair(&secp);
    let sig = secp.sign_schnorr(&msg, &keypair);
    hex::encode(sig.serialize())
}
