#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-head CLI composes explicit operator records, capability namespaces, and fail-closed adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "operator command names retain the public world-head protocol spelling"
)]

use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_head::LocalWorldHeadSigningAdapter;
use molten::world_head::LocalWorldHeadStore;
use molten::world_head::WorldHeadConflictPort;
use molten::world_head::WorldHeadStatePort;
use molten::world_head::canonical_world_head_claim;
use molten::world_head::parse_canonical_world_head_claim;
use molten::world_head::sign_world_head_claim;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadClaim;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadPurpose;
use molten_core::world_head::WorldHeadSignerRole;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;
use serde::Deserialize;
use serde::Serialize;

const HEX_CHARACTERS_PER_BYTE: usize = 2;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldHeadCommand {
    Plan {
        #[arg(long)]
        branch: String,
        #[arg(long, default_value = "local")]
        branch_class: String,
        #[arg(long)]
        expected_head: Option<String>,
        #[arg(long)]
        successor_head: String,
        #[arg(long)]
        expected_generation: u64,
        #[arg(long)]
        successor_generation: u64,
        #[arg(long)]
        purpose: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long = "source-head")]
        source_heads: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    Sign {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        claim: PathBuf,
        #[arg(long, default_value = "maintainer")]
        role: String,
        #[arg(long)]
        profile_ref: String,
        #[arg(long)]
        entropy_profile_ref: String,
        #[arg(long)]
        backend_ref: String,
        #[arg(long, default_value = "molten")]
        producer_id: String,
        #[arg(long)]
        allow_key_generation: bool,
        #[arg(long)]
        out: PathBuf,
    },
    Inspect {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        branch: String,
    },
    Advance {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        claim: PathBuf,
        #[arg(long)]
        signature: PathBuf,
    },
    Conflicts {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        branch: String,
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    Reconcile {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        branch: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureDocument {
    producer_id: String,
    key_id: String,
    public_key_hex: String,
    signature_hex: String,
    key_generation: u64,
    role: String,
    authority_admitted: bool,
}

pub(crate) fn run_world_head_command(command: WorldHeadCommand) -> Result<()> {
    match command {
        WorldHeadCommand::Plan {
            branch,
            branch_class,
            expected_head,
            successor_head,
            expected_generation,
            successor_generation,
            purpose,
            policy_ref,
            source_heads,
            out,
        } => plan(PlanInput {
            branch,
            branch_class,
            expected_head,
            successor_head,
            expected_generation,
            successor_generation,
            purpose,
            policy_ref,
            source_heads,
            out,
        }),
        WorldHeadCommand::Sign {
            state_root,
            claim,
            role,
            profile_ref,
            entropy_profile_ref,
            backend_ref,
            producer_id,
            allow_key_generation,
            out,
        } => sign(SignInput {
            state_root,
            claim,
            role,
            profile_ref,
            entropy_profile_ref,
            backend_ref,
            producer_id,
            allow_key_generation,
            out,
        }),
        WorldHeadCommand::Inspect { state_root, branch } => inspect(&state_root, &branch),
        WorldHeadCommand::Advance {
            state_root,
            claim,
            signature,
        } => advance(&state_root, &claim, &signature),
        WorldHeadCommand::Conflicts {
            state_root,
            branch,
            out_dir,
        } => conflicts(&state_root, &branch, out_dir.as_deref()),
        WorldHeadCommand::Reconcile { state_root, branch } => reconcile(&state_root, &branch),
    }
}

struct PlanInput {
    branch: String,
    branch_class: String,
    expected_head: Option<String>,
    successor_head: String,
    expected_generation: u64,
    successor_generation: u64,
    purpose: String,
    policy_ref: String,
    source_heads: Vec<String>,
    out: PathBuf,
}

fn plan(input: PlanInput) -> Result<()> {
    let claim = WorldHeadClaim {
        branch_id: WorldBranchId::new(input.branch).map_err(head_reference_error)?,
        branch_class: WorldBranchClass::parse(&input.branch_class).map_err(head_reference_error)?,
        expected_head: input.expected_head.map(WorldCommitRef::new).transpose().map_err(commit_reference_error)?,
        successor_head: WorldCommitRef::new(input.successor_head).map_err(commit_reference_error)?,
        expected_generation: input.expected_generation,
        successor_generation: input.successor_generation,
        purpose: WorldHeadPurpose::parse(&input.purpose).map_err(head_reference_error)?,
        policy_ref: WorldHeadPolicyRef::new(input.policy_ref).map_err(head_reference_error)?,
        source_heads: input
            .source_heads
            .into_iter()
            .map(WorldCommitRef::new)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(commit_reference_error)?,
    };
    let canonical = canonical_world_head_claim(&claim)?;
    std::fs::write(&input.out, &canonical.bytes)?;
    println!("claim_ref={}", canonical.claim_ref);
    println!("claim_out={}", input.out.display());
    println!("mutation=not-performed");
    Ok(())
}

struct SignInput {
    state_root: PathBuf,
    claim: PathBuf,
    role: String,
    profile_ref: String,
    entropy_profile_ref: String,
    backend_ref: String,
    producer_id: String,
    allow_key_generation: bool,
    out: PathBuf,
}

fn sign(input: SignInput) -> Result<()> {
    let claim_bytes = std::fs::read(&input.claim)?;
    let claim = parse_canonical_world_head_claim(&claim_bytes)?;
    let role = WorldHeadSignerRole::parse(&input.role).map_err(head_reference_error)?;
    let root = NodeStateRoot::open_existing(&input.state_root)?;
    let secrets = root.namespace(NodeStateNamespaceKind::Secrets)?;
    let mut signer = LocalWorldHeadSigningAdapter::new(
        &secrets,
        input.profile_ref,
        input.entropy_profile_ref,
        input.backend_ref,
        input.producer_id,
        input.allow_key_generation,
    )?;
    let (_, carrier, statement_ref) = sign_world_head_claim(&mut signer, &claim.claim, role)?;
    let document = SignatureDocument {
        producer_id: carrier.producer_id,
        key_id: carrier.key_id,
        public_key_hex: bytes_to_hex(&carrier.public_key_bytes),
        signature_hex: bytes_to_hex(&carrier.signature_bytes),
        key_generation: carrier.key_generation,
        role: carrier.role.as_str().to_string(),
        authority_admitted: false,
    };
    let output = serde_json::to_vec_pretty(&document)
        .map_err(|error| MoltenError::invalid_harness(format!("serialize world-head signature: {error}")))?;
    std::fs::write(&input.out, output)?;
    println!("claim_ref={}", claim.claim_ref);
    println!("statement_ref={statement_ref}");
    println!("signature_out={}", input.out.display());
    println!("authorization=not-granted");
    Ok(())
}

fn inspect(state_root: &Path, branch: &str) -> Result<()> {
    let branch = WorldBranchId::new(branch).map_err(head_reference_error)?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldHeadStore::open(&storage)?;
    let state = store
        .read_head(&branch)
        .map_err(|error| MoltenError::invalid_harness(format!("read world head: {error}")))?;
    if let Some(state) = state {
        println!("branch={}", state.branch_id);
        println!("class={}", state.branch_class.as_str());
        println!("head={}", state.head);
        println!("generation={}", state.generation);
        println!("policy_ref={}", state.policy_ref);
    } else {
        println!("branch={branch}");
        println!("state=absent");
    }
    Ok(())
}

fn advance(state_root: &Path, claim_path: &Path, signature_path: &Path) -> Result<()> {
    let claim = parse_canonical_world_head_claim(&std::fs::read(claim_path)?)?;
    let signature: SignatureDocument = serde_json::from_slice(&std::fs::read(signature_path)?)
        .map_err(|error| MoltenError::invalid_harness(format!("parse world-head signature: {error}")))?;
    let _ = bytes_from_hex(&signature.public_key_hex)?;
    let _ = bytes_from_hex(&signature.signature_hex)?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldHeadStore::open(&storage)?;
    let observed = store
        .read_head(&claim.claim.branch_id)
        .map_err(|error| MoltenError::invalid_harness(format!("read world head: {error}")))?;
    println!("claim_ref={}", claim.claim_ref);
    println!("observed_state={}", if observed.is_some() { "present" } else { "absent" });
    println!("decision=denied");
    println!("issue=current-authority-adapter-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone world-head advance is disabled until a current authority adapter is composed",
    ))
}

fn conflicts(state_root: &Path, branch: &str, out_dir: Option<&Path>) -> Result<()> {
    let branch = WorldBranchId::new(branch).map_err(head_reference_error)?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldHeadStore::open(&storage)?;
    let records = store
        .read_conflicts(&branch)
        .map_err(|error| MoltenError::invalid_harness(format!("read world-head conflicts: {error}")))?;
    println!("branch={branch}");
    println!("conflict_count={}", records.len());
    if let Some(out_dir) = out_dir {
        std::fs::create_dir_all(out_dir)?;
        for bytes in &records {
            let reference = molten::preserves_rail::content_ref_from_bytes(bytes);
            let digest = molten::preserves_rail::content_ref_hex(&reference)?;
            std::fs::write(out_dir.join(format!("{digest}.preserves")), bytes)?;
        }
    }
    Ok(())
}

fn reconcile(state_root: &Path, branch: &str) -> Result<()> {
    inspect(state_root, branch)?;
    println!("reconciliation=manual-review-required");
    println!("automatic-head-selection=disabled");
    Ok(())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(HEX_CHARACTERS_PER_BYTE));
    for byte in bytes {
        use std::fmt::Write;
        let write_result = write!(&mut output, "{byte:02x}");
        assert!(write_result.is_ok(), "writing to String must succeed");
    }
    output
}

fn bytes_from_hex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(HEX_CHARACTERS_PER_BYTE) {
        return Err(MoltenError::invalid_harness("hex value has an odd length"));
    }
    value
        .as_bytes()
        .chunks_exact(HEX_CHARACTERS_PER_BYTE)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| MoltenError::invalid_harness("hex value is not UTF-8"))?;
            u8::from_str_radix(text, 16).map_err(|_| MoltenError::invalid_harness("hex value contains an invalid byte"))
        })
        .collect()
}

fn head_reference_error(error: molten_core::world_head::WorldHeadReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world-head reference: {error}"))
}

fn commit_reference_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
