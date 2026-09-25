type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

const COVERAGE_FIELDS: usize = 5;
const EXEMPTION_FIELDS: usize = 3;
const JUNIT_TESTS_ATTRIBUTE: &str = "tests";
const JUNIT_FAILURES_ATTRIBUTE: &str = "failures";
const JUNIT_ERRORS_ATTRIBUTE: &str = "errors";
const JUNIT_SKIPPED_ATTRIBUTE: &str = "skipped";
const JUNIT_QUOTE: char = '"';
const NEXTEST_PROFILE_TABLE: &str = "profile";
const NEXTEST_INHERITS_FIELD: &str = "inherits";
const NEXTEST_DEFAULT_FILTER_FIELD: &str = "default-filter";
const NEXTEST_RETRIES_FIELD: &str = "retries";
const NEXTEST_FLAKY_RESULT_FIELD: &str = "flaky-result";
const NEXTEST_JUNIT_TABLE: &str = "junit";
const NEXTEST_JUNIT_PATH_FIELD: &str = "path";
const NEXTEST_ZERO_RETRIES: i64 = 0;
const MAX_NEXTEST_PROFILE_INHERITANCE_DEPTH: usize = 16;
const NEXTEST_FLAKY_PASS: &str = "pass";
const DIAGNOSTIC_JOIN_SEPARATOR: &str = "; ";
const CONFIG_LINT_FILES: &[(&str, bool)] = &[
    (".pre-commit-config.yaml", true),
    ("flake.nix", true),
    ("rust-toolchain.toml", true),
    ("README.md", true),
    ("docs/proof-workflow.md", true),
];
const CARGO_SOURCE_PREFIX: &str = "git+ssh://git@github.com/OnixResearch/";
const NIX_SOURCE_PREFIX: &str = "ssh://git@github.com/OnixResearch/";
const SOURCE_REVISION_SEPARATOR: char = '#';
const GIT_SUFFIX: &str = ".git";
const TOML_QUOTE: char = '"';
const EFFECTIVE_CONFIG_FIELD_PARTS: usize = 5;
const EFFECTIVE_CONFIG_FIELD_SEPARATOR: char = '|';
const EFFECTIVE_CONFIG_CAVEAT_SEPARATOR: char = ',';
const EFFECTIVE_CONFIG_NONE_REF: &str = "none";

#[derive(Debug, clap::Subcommand)]
pub(crate) enum TraceabilityCommand {
    Scan {
        #[arg(long, default_value = ".")]
        root: FilePath,
        #[arg(long)]
        changed_only: bool,
        #[arg(long = "coverage")]
        coverage: Vec<String>,
        #[arg(long = "exemption")]
        exemptions: Vec<String>,
        #[arg(long = "receipt")]
        receipts: Vec<FilePath>,
        #[arg(long = "require-receipt-backed")]
        require_receipt_backed: bool,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
        #[arg(long = "readback-out")]
        readback_out: Option<FilePath>,
    },
    VerificationRun(VerificationRunCommandInput),
    CiRunReceipt(CiRunReceiptCommandInput),
    NextestProfileMatrix {
        #[arg(long = "nextest-config")]
        nextest_config: FilePath,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
    },
    ConfigLint {
        #[arg(long, default_value = ".")]
        root: FilePath,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
    },
    EffectiveConfig {
        #[arg(long = "profile-ref")]
        profile_refs: Vec<String>,
        #[arg(long = "field")]
        fields: Vec<String>,
        #[arg(long = "release-mode")]
        release_mode: bool,
        #[arg(long)]
        out: Option<FilePath>,
        #[arg(long = "summary-out")]
        summary_out: Option<FilePath>,
    },
    ContextProfile(ContextProfileCommandInput),
}

pub(crate) fn run_traceability_command(command: TraceabilityCommand) -> Outcome<()> {
    match command {
        TraceabilityCommand::Scan {
            root,
            changed_only,
            coverage,
            exemptions,
            receipts,
            require_receipt_backed,
            out,
            summary_out,
            readback_out,
        } => run_scan(ScanInput {
            root,
            changed_only,
            coverage,
            exemptions,
            receipts,
            require_receipt_backed,
            out,
            summary_out,
            readback_out,
        }),
        TraceabilityCommand::VerificationRun(input) => run_verification_run(input),
        TraceabilityCommand::CiRunReceipt(input) => run_ci_run_receipt(input),
        TraceabilityCommand::NextestProfileMatrix {
            nextest_config,
            out,
            summary_out,
        } => run_nextest_profile_matrix(NextestProfileMatrixCommandInput {
            nextest_config,
            out,
            summary_out,
        }),
        TraceabilityCommand::ConfigLint { root, out, summary_out } => {
            run_config_lint(ConfigLintCommandInput { root, out, summary_out })
        }
        TraceabilityCommand::EffectiveConfig {
            profile_refs,
            fields,
            release_mode,
            out,
            summary_out,
        } => run_effective_config(EffectiveConfigCommandInput {
            profile_refs,
            fields,
            release_mode,
            out,
            summary_out,
        }),
        TraceabilityCommand::ContextProfile(input) => run_context_profile(input),
    }
}

struct ScanInput {
    root: FilePath,
    changed_only: bool,
    coverage: Vec<String>,
    exemptions: Vec<String>,
    receipts: Vec<FilePath>,
    require_receipt_backed: bool,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
    readback_out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct VerificationRunCommandInput {
    #[arg(long)]
    requirement: String,
    #[arg(long = "coverage-kind")]
    coverage_kind: String,
    #[arg(long)]
    target: String,
    #[arg(long = "argv")]
    argv: Vec<String>,
    #[arg(long = "profile-ref")]
    profile_ref: String,
    #[arg(long = "toolchain-ref")]
    toolchain_refs: Vec<String>,
    #[arg(long = "exit-status")]
    exit_status: i64,
    #[arg(long = "stdout-ref")]
    stdout_ref: String,
    #[arg(long = "stderr-ref")]
    stderr_ref: String,
    #[arg(long = "artifact-ref")]
    artifact_refs: Vec<String>,
    #[arg(long)]
    out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct CiRunReceiptCommandInput {
    #[arg(long = "source-marker")]
    source_marker: String,
    #[arg(long = "profile-id")]
    profile_id: String,
    #[arg(long = "command-surface")]
    command_surface: String,
    #[arg(long = "nextest-config")]
    nextest_config: FilePath,
    #[arg(long = "cargo-metadata")]
    cargo_metadata: FilePath,
    #[arg(long = "binaries-metadata")]
    binaries_metadata: FilePath,
    #[arg(long)]
    junit: FilePath,
    #[arg(long, default_value = "pass")]
    decision: String,
    #[arg(long = "caveat")]
    caveats: Vec<String>,
    #[arg(long)]
    out: Option<FilePath>,
}

struct NextestProfileMatrixCommandInput {
    nextest_config: FilePath,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

struct ConfigLintCommandInput {
    root: FilePath,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

struct EffectiveConfigCommandInput {
    profile_refs: Vec<String>,
    fields: Vec<String>,
    release_mode: bool,
    out: Option<FilePath>,
    summary_out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ContextProfileCommandInput {
    #[arg(long = "profile-id")]
    profile_id: String,
    #[arg(long = "profile-tier", default_value = "local")]
    profile_tier: String,
    #[arg(long = "allowed-operation")]
    allowed_operations: Vec<String>,
    #[arg(long)]
    operation: String,
    #[arg(long = "policy-ref")]
    policy_refs: Vec<String>,
    #[arg(long = "authority-ref")]
    authority_refs: Vec<String>,
    #[arg(long = "resource-ref")]
    resource_refs: Vec<String>,
    #[arg(long = "evidence-ref")]
    evidence_refs: Vec<String>,
    #[arg(long = "retention-ref")]
    retention_refs: Vec<String>,
    #[arg(long = "override-authority-ref")]
    override_authority_refs: Vec<String>,
    #[arg(long = "override-evidence-ref")]
    override_evidence_refs: Vec<String>,
    #[arg(long = "require-policy")]
    require_policy: bool,
    #[arg(long = "require-authority")]
    require_authority: bool,
    #[arg(long = "require-resource")]
    require_resource: bool,
    #[arg(long = "require-evidence")]
    require_evidence: bool,
    #[arg(long = "require-retention")]
    require_retention: bool,
    #[arg(long)]
    out: Option<FilePath>,
    #[arg(long = "summary-out")]
    summary_out: Option<FilePath>,
}
