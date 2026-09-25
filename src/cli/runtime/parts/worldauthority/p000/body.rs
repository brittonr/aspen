use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_branch_authority::encode_receipt;
use molten::world_branch_authority::plan_receipt;
use molten_core::world_branch_authority::CapabilityKind;
use molten_core::world_branch_authority::CurrentAuthorityFacts;
use molten_core::world_branch_authority::NormalizedCapabilityScope;
use molten_core::world_branch_authority::WorldBranchAction;
use molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic;
use molten_core::world_branch_authority::WorldBranchAuthorityFacts;
use molten_core::world_branch_authority::WorldBranchAuthorityPlan;
use molten_core::world_branch_authority::WorldBranchMode;
use molten_core::world_branch_authority::deny_world_branch_authority_plan;
use molten_core::world_branch_authority::plan_world_branch_authority;
use serde::Deserialize;

const OPERATOR_REQUEST_SCHEMA: &str = "molten.world-branch-authority-operator-request.v1";
const MAXIMUM_OPERATOR_REQUEST_BYTES: u64 = 1_048_576;
const MAXIMUM_POLICY_BYTES: u64 = 4_194_304;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldAuthorityCommand {
    Plan {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityInspect {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        policy: PathBuf,
    },
    Activate {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    Transfer {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    Simulate {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    Recover {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperatorRequest {
    schema: String,
    capability_kind: String,
    action: String,
    source_branch_ref: String,
    destination_branch_ref: String,
    capability_ref: String,
    source_scope: ScopeDto,
    destination_scope: ScopeDto,
    policy_generation: u64,
    mapping_lossless: bool,
    current: CurrentDto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeDto {
    resource: String,
    abilities: Vec<String>,
    limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentDto {
    observation_ref: String,
    policy: bool,
    capability: bool,
    revocation: bool,
    replay: bool,
    scope: bool,
    ucan_verified: bool,
}

// r[impl molten.world_branch_authority.evidence]
pub(crate) fn run_world_authority_command(command: WorldAuthorityCommand) -> Result<()> {
    match command {
        WorldAuthorityCommand::Plan { request, policy, out } => write_plan(&request, &policy, &out),
        WorldAuthorityCommand::AuthorityInspect { request, policy } => inspect_authority(&request, &policy),
        WorldAuthorityCommand::Activate {
            request,
            policy,
            receipt_out,
        } => denied_runtime_command(DeniedCommandInput {
            request_path: &request,
            policy_path: &policy,
            receipt_out: &receipt_out,
            expected_mode: None,
            missing_runtime_diagnostic: WorldBranchAuthorityDiagnostic::MissingObligationEvidence,
            operation: "activation",
        }),
        WorldAuthorityCommand::Transfer {
            request,
            policy,
            receipt_out,
        } => denied_runtime_command(DeniedCommandInput {
            request_path: &request,
            policy_path: &policy,
            receipt_out: &receipt_out,
            expected_mode: Some(WorldBranchMode::Linear),
            missing_runtime_diagnostic: WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous,
            operation: "transfer",
        }),
        WorldAuthorityCommand::Simulate {
            request,
            policy,
            receipt_out,
        } => denied_runtime_command(DeniedCommandInput {
            request_path: &request,
            policy_path: &policy,
            receipt_out: &receipt_out,
            expected_mode: Some(WorldBranchMode::SimulationOnly),
            missing_runtime_diagnostic: WorldBranchAuthorityDiagnostic::SimulationAdapterMissing,
            operation: "simulation",
        }),
        WorldAuthorityCommand::Recover {
            request,
            policy,
            receipt_out,
        } => denied_runtime_command(DeniedCommandInput {
            request_path: &request,
            policy_path: &policy,
            receipt_out: &receipt_out,
            expected_mode: None,
            missing_runtime_diagnostic: WorldBranchAuthorityDiagnostic::ActivationOutcomeUnknown,
            operation: "recovery",
        }),
    }
}

fn write_plan(request_path: &Path, policy_path: &Path, out: &Path) -> Result<()> {
    let plan = load_plan(request_path, policy_path)?;
    let receipt = plan_receipt(&plan);
    let (receipt_ref, bytes) = encode_receipt(&receipt)?;
    std::fs::write(out, bytes).map_err(MoltenError::from)?;
    println!("plan_ref={}", plan.plan_ref);
    println!("receipt_ref={receipt_ref}");
    println!("allowed={}", plan.allowed);
    println!("diagnostic={}", receipt.diagnostic);
    Ok(())
}

fn inspect_authority(request_path: &Path, policy_path: &Path) -> Result<()> {
    let plan = load_plan(request_path, policy_path)?;
    let receipt = plan_receipt(&plan);
    println!("plan_ref={}", plan.plan_ref);
    println!("policy_ref={}", plan.policy_ref);
    println!("capability_ref={}", plan.capability_ref);
    println!("mode={}", plan.mode.map_or("none", WorldBranchMode::as_str));
    println!("allowed={}", plan.allowed);
    println!("diagnostic={}", receipt.diagnostic);
    println!("obligation_count={}", plan.obligations.len());
    println!("non_claim_count={}", plan.non_claims.len());
    Ok(())
}

struct DeniedCommandInput<'a> {
    request_path: &'a Path,
    policy_path: &'a Path,
    receipt_out: &'a Path,
    expected_mode: Option<WorldBranchMode>,
    missing_runtime_diagnostic: WorldBranchAuthorityDiagnostic,
    operation: &'a str,
}

fn denied_runtime_command(input: DeniedCommandInput<'_>) -> Result<()> {
    let DeniedCommandInput {
        request_path,
        policy_path,
        receipt_out,
        expected_mode,
        missing_runtime_diagnostic,
        operation,
    } = input;
    let planned = load_plan(request_path, policy_path)?;
    let diagnostic = if planned.allowed && expected_mode.is_some_and(|expected| planned.mode != Some(expected)) {
        WorldBranchAuthorityDiagnostic::ActionModeMismatch
    } else if planned.allowed {
        missing_runtime_diagnostic
    } else {
        planned.diagnostic
    };
    let denied = if planned.allowed {
        deny_world_branch_authority_plan(planned, diagnostic)
    } else {
        planned
    };
    let receipt = plan_receipt(&denied);
    let (receipt_ref, bytes) = encode_receipt(&receipt)?;
    std::fs::write(receipt_out, bytes).map_err(MoltenError::from)?;
    println!("decision=denied");
    println!("receipt_ref={receipt_ref}");
    println!("diagnostic={}", receipt.diagnostic);
    Err(MoltenError::invalid_harness(format!(
        "world branch {operation} requires an admitted runtime adapter"
    )))
}

fn load_plan(request_path: &Path, policy_path: &Path) -> Result<WorldBranchAuthorityPlan> {
    let request_bytes = read_bounded(request_path, MAXIMUM_OPERATOR_REQUEST_BYTES, "world branch authority request")?;
    let policy_bytes = read_bounded(policy_path, MAXIMUM_POLICY_BYTES, "world branch policy")?;
    let request = parse_request(&request_bytes)?;
    let policy = std::str::from_utf8(&policy_bytes)
        .map_err(|_| MoltenError::invalid_harness("world branch policy is not UTF-8"))?;
    let (facts, current) = request.into_facts()?;
    Ok(plan_world_branch_authority(policy, &facts, &current))
}

fn parse_request(bytes: &[u8]) -> Result<OperatorRequest> {
    let request: OperatorRequest = serde_json::from_slice(bytes)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid world authority request: {error}")))?;
    if request.schema != OPERATOR_REQUEST_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported world authority request schema"));
    }
    Ok(request)
}

impl OperatorRequest {
    fn into_facts(self) -> Result<(WorldBranchAuthorityFacts, CurrentAuthorityFacts)> {
        let facts = WorldBranchAuthorityFacts {
            capability_kind: parse_capability_kind(&self.capability_kind)?,
            action: parse_action(&self.action)?,
            source_branch_ref: self.source_branch_ref,
            destination_branch_ref: self.destination_branch_ref,
            capability_ref: self.capability_ref,
            source_scope: self.source_scope.into_scope()?,
            destination_scope: self.destination_scope.into_scope()?,
            policy_generation: self.policy_generation,
            mapping_lossless: self.mapping_lossless,
        };
        let current = CurrentAuthorityFacts {
            observation_ref: self.current.observation_ref,
            policy_current: self.current.policy,
            capability_current: self.current.capability,
            revocation_current: self.current.revocation,
            replay_current: self.current.replay,
            scope_current: self.current.scope,
            ucan_verified: self.current.ucan_verified,
        };
        Ok((facts, current))
    }
}

impl ScopeDto {
    fn into_scope(self) -> Result<NormalizedCapabilityScope> {
        NormalizedCapabilityScope::new(self.resource, self.abilities, self.limit)
            .map_err(|_| MoltenError::invalid_harness("world authority scope is invalid"))
    }
}
