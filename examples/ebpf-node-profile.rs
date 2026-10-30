//! Experimental operator-only provisioning. Source identity never grants observer permission.
//!
//! Sequence: actual `molten node init --state-root ROOT`, `source`, author the explicit
//! closed selection JSON, `inspect-intent`, independently approve the exact read policy,
//! then `provision`. Run authoring separately from the unprivileged node process.
//! `source` requires the real signed input-addressed startup-source-binding Nix output,
//! complete retained/store closure, and independently provisioned source-gate trust.
//! The parent-built main executable must equal that output's bin/molten bytes. The
//! actual node must still admit /proc/self/exe; authoring cannot mint its opaque token.
//! Copy neither signing seeds nor raw credentials to an evidence archive. Missing
//! trust, signed output, closure, explicit policy or exact bytes is a hard failure.
#[path = "ebpf-node-proof/inputs.rs"]
mod inputs;
#[path = "ebpf-node-proof/operator.rs"]
mod operator;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[derive(clap::Parser)]
#[command(about = "Experimental explicit observer provisioning; never node execution evidence")]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Verify the independently signed real source output; does not admit this process.
    Source(Source),
    /// Explicitly sign and install one exact independently authorized observer selection.
    Provision(Provision),
    /// Revoke the installed exact observer token through its independent trust file.
    Revoke {
        #[arg(long)]
        node_root: std::path::PathBuf,
    },
    /// Print canonical intent identity for explicit exact-resource policy authoring; grants
    /// nothing.
    InspectIntent {
        #[arg(long)]
        intent: std::path::PathBuf,
    },
}

#[derive(clap::Args)]
struct Source {
    #[arg(long)]
    node_root: std::path::PathBuf,
    #[arg(long)]
    source_trust: std::path::PathBuf,
    #[arg(long)]
    signed_output: std::path::PathBuf,
}

#[derive(clap::Args)]
struct Provision {
    #[command(flatten)]
    source: Source,
    #[arg(long)]
    issuer_key: std::path::PathBuf,
    #[arg(long)]
    holder_key: std::path::PathBuf,
    #[arg(long)]
    policy: std::path::PathBuf,
    #[arg(long)]
    profile: std::path::PathBuf,
    #[arg(long)]
    endpoint: std::path::PathBuf,
    #[arg(long)]
    intent: std::path::PathBuf,
    #[arg(long)]
    nonce_file: std::path::PathBuf,
    #[arg(long)]
    not_before: u64,
    #[arg(long)]
    expires: u64,
}

fn main() -> std::process::ExitCode {
    match run(<Arguments as clap::Parser>::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Never print signing input, policy, tokens, credentials, or key bytes.
            eprintln!("experimental observer provisioning denied: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: Arguments) -> Result<()> {
    match args.command {
        Command::Source(args) => {
            let (_, validation) = inputs::source(&args)?;
            println!(
                "{}",
                serde_json::json!({
                    "schema": "experimental-observer-selected-source-v1",
                    "binding_ref": validation.binding_ref,
                    "selected_validation_ref": validation.selected_output_validation_ref,
                    "expected_startup_validation_ref": validation.expected_startup_validation_ref,
                    "claim": "authoring-only-not-current-process-admission"
                })
            );
            Ok(())
        }
        Command::Provision(args) => provision(args),
        Command::Revoke { node_root } => operator::revoke(&node_root),
        Command::InspectIntent { intent } => {
            let selection = molten::fabric_observability::nodeadmission::decode_node_observation_selection(
                &inputs::read(&intent, 16_384)?,
            )?;
            println!(
                "{}",
                serde_json::json!({
                    "selection_ref": molten::fabric_observability::nodeadmission::node_observation_selection_ref(&selection)?,
                    "resource": molten::fabric_observability::nodeadmission::node_observation_resource(&selection)?,
                    "contract": "linux-observation-reader", "ability": "read", "authority": false
                })
            );
            Ok(())
        }
    }
}

fn provision(args: Provision) -> Result<()> {
    let (root, validation) = inputs::source(&args.source)?;
    let intent = inputs::read(&args.intent, 16_384)?;
    let selection = molten::fabric_observability::nodeadmission::decode_node_observation_selection(&intent)?;
    if selection.source_validation_ref != validation.expected_startup_validation_ref {
        return Err("intent source reference differs from actual signed source admission".into());
    }
    let profile = inputs::read(&args.profile, 65_536)?;
    let endpoint = inputs::read(&args.endpoint, 4_194_304)?;
    inputs::check_profile(&selection, &profile)?;
    if molten::preserves_rail::content_ref_from_bytes(&endpoint) != selection.endpoint_content_ref {
        return Err("immutable endpoint bytes differ from signed operator intent".into());
    }
    let (trust, credential) = operator::authorize(&args, &selection)?;
    inputs::install(&root, &selection, &profile, &endpoint)?;
    let authority = molten::node_state::RelativePath::parse("linux-observation-trust")?;
    root.write_new(&authority, &trust)?;
    root.write_new(
        &molten::fabric_observability::nodeadmission::node_observer_credential_path(&selection)?,
        &credential,
    )?;
    // Publish bounded startup data last; neither this file nor its source reference grants authority.
    root.write_new(&molten::node_state::RelativePath::parse("linux-observation-startup.json")?, &intent)?;
    println!(
        "{}",
        serde_json::json!({
            "schema": "experimental-observer-provisioned-v1",
            "selection_ref": molten::fabric_observability::nodeadmission::node_observation_selection_ref(&selection)?,
            "source_validation_ref": selection.source_validation_ref,
            "profile_ref": selection.checked_profile_ref,
            "endpoint_content_ref": selection.endpoint_content_ref,
            "claim": "signed-intent-only-not-node-readiness"
        })
    );
    Ok(())
}
