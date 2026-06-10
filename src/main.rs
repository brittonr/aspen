use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use molten::artifacts;
use molten::catalog;
use molten::catalog_mcp;
use molten::chunk_store;
use molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
use molten::coordination;
use molten::delivery_idempotency;
use molten::error::MoltenError;
use molten::error::Result;
use molten::eval_cache;
use molten::evidence::PASS_EVIDENCE_PURPOSE;
use molten::evidence::SignReceiptInput;
use molten::evidence::SignedReceiptKey;
use molten::evidence::SignedReceiptKeyInput;
use molten::evidence::SignedReceiptKeyRevocation;
use molten::evidence::SignedReceiptKeyRevocationInput;
use molten::evidence::VerifySignedReceiptKeyringPolicy;
use molten::evidence::VerifySignedReceiptPolicy;
use molten::evidence::parse_signed_receipt_key;
use molten::evidence::parse_signed_receipt_key_revocation;
use molten::evidence::sign_receipt;
use molten::evidence::signed_receipt_key_revocation_value;
use molten::evidence::signed_receipt_key_value;
use molten::evidence::signed_receipt_summary;
use molten::evidence::verify_signed_receipt_with_keyring_policy;
use molten::evidence::verify_signed_receipt_with_policy;
use molten::evidence_chain::ChainForkPolicy;
use molten::evidence_chain::ChainScope;
use molten::harness::ReproExportProfile;
use molten::harness::failure_repro_bundle_value_with_command;
use molten::harness::failure_summary;
use molten::harness::failure_value;
use molten::harness::gate_check_value;
use molten::harness::gate_receipt_summary;
use molten::harness::gate_receipt_value;
use molten::harness::parse_failure;
use molten::harness::parse_repro_bundle;
use molten::harness::replay_report_value;
use molten::harness::report_failure_value;
use molten::harness::report_suite_value;
use molten::harness::report_summary;
use molten::harness::repro_bundle_summary;
use molten::harness::repro_bundle_value_with_export_profile;
use molten::harness::repro_verify_receipt_summary;
use molten::harness::repro_verify_receipt_value;
use molten::harness::run_suite_value;
use molten::harness::suite_failure_value;
use molten::harness::validate_report_value;
use molten::iroh_exchange::FetchBundleInput;
use molten::iroh_exchange::FetchChainSegmentInput;
use molten::iroh_exchange::PublishChainSegmentInput;
use molten::iroh_exchange::fetch_bundle;
use molten::iroh_exchange::fetch_chain_segment;
use molten::iroh_exchange::publish_bundle;
use molten::iroh_exchange::publish_chain_segment;
use molten::job_dag;
use molten::ledger;
use molten::node_daemon;
use molten::node_runtime;
use molten::octet_gate;
use molten::octet_remediation;
use molten::operator_dogfood;
use molten::plugin_host;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::preserves_rail::u64_value;
use molten::protocol_session;
use molten::provenance;
use molten::raft_control_plane;
use molten::remote_dataspace;
use molten::retention;
use molten::rewrites;
use molten::schema_identity;
use molten::secrets;
use molten::service_runtime;
use molten::service_supervision;
use molten::transcripts;
use molten::typed_storage;
use molten::upgrades;

const PROTOCOL_LIFECYCLE_INDEX_LIMIT: usize = 256;
const COORDINATION_CLI_BATCH_REF_LIMIT: usize = 4096;
const COORDINATION_CLI_BATCH_EVIDENCE_LIMIT: usize = 16384;
const JOB_WORKER_CLI_REF_LIMIT: usize = 4096;
const PROVENANCE_CLI_EVIDENCE_LIMIT: usize = 64;
const SIGNED_KEYRING_CLI_ENTRY_LIMIT: usize = 4096;
const _: () = assert!(PROTOCOL_LIFECYCLE_INDEX_LIMIT > 0);
const _: () = assert!(COORDINATION_CLI_BATCH_REF_LIMIT <= 100_000);
const _: () = assert!(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT <= 100_000);
const _: () = assert!(JOB_WORKER_CLI_REF_LIMIT <= 100_000);
const _: () = assert!(PROVENANCE_CLI_EVIDENCE_LIMIT <= 100_000);
const _: () = assert!(SIGNED_KEYRING_CLI_ENTRY_LIMIT <= 100_000);

#[derive(Debug, Parser)]
#[command(name = "molten", version, about = "Molten runtime prototype")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Command {
    Test {
        #[command(subcommand)]
        command: TestCommand,
    },
    Dogfood {
        #[command(subcommand)]
        command: DogfoodCommand,
    },
    Receipts {
        #[command(subcommand)]
        command: ReceiptsCommand,
    },
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum TestCommand {
    Run {
        suite: PathBuf,
        #[arg(long)]
        report_out: Option<PathBuf>,
    },
    Replay {
        report: PathBuf,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
    Report {
        #[command(subcommand)]
        command: ReportCommand,
    },
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
    Receipt {
        #[command(subcommand)]
        command: ReceiptCommand,
    },
    Ledger {
        #[command(subcommand)]
        command: LedgerCommand,
    },
    Chain {
        #[command(subcommand)]
        command: ChainCommand,
    },
    Chunk {
        #[command(subcommand)]
        command: ChunkCommand,
    },
    Storage {
        #[command(subcommand)]
        command: StorageCommand,
    },
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    Upgrade {
        #[command(subcommand)]
        command: UpgradeCommand,
    },
    Transcript {
        #[command(subcommand)]
        command: TranscriptCommand,
    },
    Rewrite {
        #[command(subcommand)]
        command: RewriteCommand,
    },
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
    Remote {
        #[command(subcommand)]
        command: RemoteCommand,
    },
    Delivery {
        #[command(subcommand)]
        command: DeliveryCommand,
    },
    Retention {
        #[command(subcommand)]
        command: RetentionCommand,
    },
    Provenance {
        #[command(subcommand)]
        command: ProvenanceCommand,
    },
    Protocol {
        #[command(subcommand)]
        command: ProtocolCommand,
    },
    Raft {
        #[command(subcommand)]
        command: RaftCommand,
    },
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    Coordination {
        #[command(subcommand)]
        command: CoordinationCommand,
    },
    Secrets {
        #[command(subcommand)]
        command: SecretsCommand,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    Octet {
        #[command(subcommand)]
        command: OctetCommand,
    },
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
    Repro {
        #[command(subcommand)]
        command: ReproCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProtocolCommand {
    Install {
        manifest: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunRequestResponse {
        #[arg(long)]
        out: PathBuf,
    },
    GateLifecycle {
        dir: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        receipt: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum RaftCommand {
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum CoordinationCommand {
    Manifest {
        #[arg(long, default_value = "coordination:local")]
        service_id: String,
        #[arg(long = "service")]
        services: Vec<String>,
        #[arg(long)]
        control_group_ref: Option<String>,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_QUEUE_CAPACITY)]
        queue_capacity: u64,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_SEMAPHORE_CAPACITY)]
        semaphore_capacity: u64,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_RATE_LIMIT)]
        rate_limit: u64,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_BARRIER_PARTIES)]
        barrier_parties: u64,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Request {
        #[arg(long)]
        service: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        client_session: String,
        #[arg(long)]
        operation_id_ref: String,
        #[arg(long)]
        payload: Option<PathBuf>,
        #[arg(long = "authority-ref")]
        authority_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Apply {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long = "request")]
        requests: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum SecretsCommand {
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    Install {
        manifest: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunFixture {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    Run {
        suite: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunTwoService {
        #[arg(long)]
        out: PathBuf,
    },
    Supervise {
        suite: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    RunSupervisionFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        report: PathBuf,
    },
    ShowSupervision {
        report: PathBuf,
    },
    GateSupervision {
        report: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Replay {
        report: PathBuf,
    },
    ReplaySupervision {
        report: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum OctetCommand {
    Gate {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long, default_value = "strict-ci")]
        profile: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SourceGate {
        #[command(subcommand)]
        command: OctetSourceGateCommand,
    },
    Baseline {
        #[command(subcommand)]
        command: OctetBaselineCommand,
    },
    Review {
        #[command(subcommand)]
        command: OctetReviewCommand,
    },
    Artifacts {
        #[command(subcommand)]
        command: OctetArtifactsCommand,
    },
    Remediation {
        #[command(subcommand)]
        command: OctetRemediationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum OctetRemediationCommand {
    Plan {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long = "lib-artifacts")]
        lib_artifacts: Option<PathBuf>,
        #[arg(long = "focused-object-corpus")]
        focused_object_corpus: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum OctetSourceGateCommand {
    Validate {
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        gate_receipt: PathBuf,
        #[arg(long = "source-scope")]
        source_scope: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum OctetArtifactsCommand {
    Import {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum OctetReviewCommand {
    Write {
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "quarantine-ci")]
        profile: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long = "finding-key")]
        finding_keys: Vec<String>,
        #[arg(long, default_value = "manual review")]
        rationale: String,
    },
}

#[derive(Debug, Subcommand)]
enum DogfoodCommand {
    LocalNode {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        release_gate_out: Option<PathBuf>,
    },
    NixReleaseExport {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    NixReleaseVerify {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    ReleaseBundleExport {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    ReleaseBundleVerify {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
        #[arg(long = "signed-member")]
        signed_members: Vec<PathBuf>,
        #[arg(long)]
        require_signed_members: bool,
        #[arg(long, default_value = "release-evidence")]
        signed_purpose: String,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long, default_value = "local-release-key")]
        signed_key: String,
        #[arg(long)]
        signed_key_ledger: Option<PathBuf>,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
    },
    ReleasePromote {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        bundle_verify: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
        #[arg(long)]
        signed_key_ledger: PathBuf,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
        #[arg(long)]
        source_evidence: String,
        #[arg(long)]
        octet_evidence: String,
        #[arg(long)]
        cairn_evidence: String,
    },
    ReleasePromotionSummary {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        signed_key_ledger: Option<PathBuf>,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
    },
    ReleaseExport {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        manifest_out: PathBuf,
    },
    ReleaseExportVerify {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ReceiptsCommand {
    List {
        #[arg(long)]
        ledger: PathBuf,
    },
    Show {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Validate {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Export {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Key {
        #[command(subcommand)]
        command: ReceiptKeyCommand,
    },
    Sign {
        receipt: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "local-signer")]
        signer: String,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
    },
    VerifySigned {
        signed_receipt: PathBuf,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long)]
        key_ledger: Option<PathBuf>,
        #[arg(long)]
        key_ref: Option<String>,
        #[arg(long)]
        key_id: Option<String>,
        #[arg(long)]
        signer: Option<String>,
        #[arg(long)]
        subject_ref: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ReceiptKeyCommand {
    Import {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        key_id: String,
        #[arg(long)]
        signer: String,
        #[arg(long)]
        trust_root: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        ledger: PathBuf,
    },
    Show {
        key_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Revoke {
        key_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long, default_value = "operator-revoked")]
        reason: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Rotate {
        old_key_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        new_key_id: String,
        #[arg(long)]
        new_key: String,
        #[arg(long, default_value = "rotated")]
        reason: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum NodeCommand {
    Init {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = "node:local")]
        node_id: String,
        #[arg(long)]
        config_out: Option<PathBuf>,
        #[arg(long)]
        identity_receipt_out: Option<PathBuf>,
    },
    Run {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        startup_out: Option<PathBuf>,
    },
    RunLoop {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LOOP_REQUESTS)]
        max_requests: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        heartbeat_out: Option<PathBuf>,
    },
    Serve {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_SERVICE_TICKS)]
        max_ticks: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LOOP_REQUESTS)]
        max_requests_per_tick: u64,
        #[arg(long)]
        live_iroh: bool,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_LISTENER_EVENTS)]
        live_max_events: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS)]
        live_event_timeout_ms: u64,
        #[arg(long)]
        service_receipt_out: Option<PathBuf>,
        #[arg(long)]
        live_ticket_out: Option<PathBuf>,
        #[arg(long)]
        supervisor_policy: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        health_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Stop {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        shutdown_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
    ControlRequest {
        #[arg(long)]
        operation: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        payload: Option<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
    },
    ProvenanceFixture {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityGrantFixture {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long)]
        peer: String,
        #[arg(long)]
        node: String,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long, default_value = "*")]
        target_scope: String,
        #[arg(long, default_value = "*")]
        resource_scope: String,
        #[arg(long, default_value_t = 1)]
        epoch: u64,
        #[arg(long)]
        expires_at: Option<u64>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "revocation")]
        revocation_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityGrantImport {
        #[arg(long)]
        state_root: PathBuf,
        grant: PathBuf,
        #[arg(long)]
        peer: Option<String>,
        #[arg(long)]
        node: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SupervisorPolicyFixture {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        max_restarts: u64,
        #[arg(long, default_value_t = 1)]
        restart_window_ticks: u64,
        #[arg(long, default_value_t = 1)]
        heartbeat_timeout_ticks: u64,
        #[arg(long, default_value_t = 1)]
        shutdown_drain_ticks: u64,
        #[arg(long)]
        allow_stale_lock_recovery: bool,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    LiveTicketExport {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    LiveTicketImport {
        #[arg(long)]
        state_root: PathBuf,
        ticket: PathBuf,
        #[arg(long)]
        peer_admission: Option<PathBuf>,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LivePeerAdmit {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        peer: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long)]
        expires_at: Option<u64>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        ticket: PathBuf,
    },
    ControlSubmit {
        #[arg(long)]
        state_root: PathBuf,
        request: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlDispatch {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlIngressBuild {
        request: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        to_node: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
    },
    ControlIngressLiveBuild {
        request: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        to_node: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
    },
    ControlIngressLiveLoopback {
        #[arg(long)]
        state_root: PathBuf,
        request: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        to_node: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        publish_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receive_receipt_out: Option<PathBuf>,
    },
    ControlIngressLiveSend {
        #[arg(long)]
        state_root: Option<PathBuf>,
        request: PathBuf,
        ticket: PathBuf,
        #[arg(long)]
        from_peer: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
        #[arg(long = "expected-node")]
        expected_node: Option<String>,
        #[arg(long = "expected-topic")]
        expected_topic: Option<String>,
        #[arg(long = "expected-endpoint")]
        expected_endpoint: Option<String>,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long)]
        transport_receipt_out: Option<PathBuf>,
        #[arg(long)]
        retry_receipts_dir: Option<PathBuf>,
        #[arg(long)]
        duplicate_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundle {
        #[arg(long)]
        state_root: Option<PathBuf>,
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        peer_admission: PathBuf,
        #[arg(long)]
        authority_grant: PathBuf,
        #[arg(long)]
        send_receipt: PathBuf,
        #[arg(long = "receive-receipt")]
        receive_receipts: Vec<PathBuf>,
        #[arg(long)]
        listener_receipt: Option<PathBuf>,
        #[arg(long)]
        service_receipt: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleExport {
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        peer_admission: PathBuf,
        #[arg(long)]
        authority_grant: PathBuf,
        #[arg(long = "receipt")]
        receipt_values: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleVerify {
        bundle: PathBuf,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleGate {
        bundle: PathBuf,
        #[arg(long)]
        verify_receipt: Option<PathBuf>,
        #[arg(long)]
        require_verify_receipt: bool,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleApply {
        #[arg(long)]
        state_root: PathBuf,
        bundle: PathBuf,
        #[arg(long)]
        gate_receipt: Option<PathBuf>,
        #[arg(long)]
        require_gate_receipt: bool,
        #[arg(long)]
        request: Option<PathBuf>,
        #[arg(long)]
        send: bool,
        #[arg(long)]
        from_peer: Option<String>,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long = "operation-id")]
        operation_id: Option<String>,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long = "peer-bootstrap")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long)]
        send_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleReconcile {
        apply_receipt: PathBuf,
        #[arg(long)]
        send_receipt: Option<PathBuf>,
        #[arg(long)]
        ingress_receipt: Option<PathBuf>,
        #[arg(long)]
        queue_receipt: Option<PathBuf>,
        #[arg(long)]
        control_receipt: Option<PathBuf>,
        #[arg(long)]
        expected_envelope: Option<String>,
        #[arg(long)]
        expected_operation: Option<String>,
        #[arg(long)]
        expected_request: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleAckExport {
        apply_receipt: PathBuf,
        #[arg(long)]
        send_receipt: Option<PathBuf>,
        #[arg(long)]
        ingress_receipt: Option<PathBuf>,
        #[arg(long)]
        queue_receipt: Option<PathBuf>,
        #[arg(long)]
        control_receipt: Option<PathBuf>,
        #[arg(long)]
        reconcile_receipt: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleAckImport {
        #[arg(long)]
        state_root: PathBuf,
        ack: PathBuf,
        #[arg(long)]
        expected_bundle: Option<String>,
        #[arg(long)]
        expected_envelope: Option<String>,
        #[arg(long)]
        expected_operation: Option<String>,
        #[arg(long)]
        expected_request: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleProtocolGate {
        bundle: PathBuf,
        #[arg(long)]
        gate_receipt: PathBuf,
        #[arg(long)]
        apply_receipt: PathBuf,
        #[arg(long)]
        reconcile_receipt: PathBuf,
        #[arg(long)]
        ack: PathBuf,
        #[arg(long)]
        expected_envelope: Option<String>,
        #[arg(long)]
        expected_operation: Option<String>,
        #[arg(long)]
        expected_request: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    LiveWorkflowBundleImport {
        #[arg(long)]
        state_root: PathBuf,
        bundle: PathBuf,
        #[arg(long)]
        expected_node: Option<String>,
        #[arg(long)]
        expected_topic: Option<String>,
        #[arg(long)]
        expected_endpoint: Option<String>,
        #[arg(long)]
        expected_peer: Option<String>,
        #[arg(long = "operation")]
        operations: Vec<String>,
        #[arg(long)]
        target_scope: Option<String>,
        #[arg(long)]
        resource_scope: Option<String>,
        #[arg(long, default_value_t = 1)]
        as_of_sequence: u64,
        #[arg(long, default_value_t = 1)]
        as_of_epoch: u64,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlIngressPublish {
        #[arg(long)]
        state_root: PathBuf,
        envelope: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlIngressDeliver {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        envelope_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ControlDeny {
        request: PathBuf,
        #[arg(long)]
        startup: String,
        #[arg(long)]
        diagnostic: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Shutdown {
        #[arg(long)]
        startup: String,
        #[arg(long = "adapter")]
        adapters: Vec<String>,
        #[arg(long = "drained-job")]
        drained_jobs: Vec<String>,
        #[arg(long = "index")]
        index_receipt_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Health {
        startup_receipt: PathBuf,
        #[arg(long)]
        shutdown: Option<String>,
        #[arg(long = "index")]
        index_receipt_refs: Vec<String>,
        #[arg(long = "head")]
        head_refs: Vec<String>,
        #[arg(long = "open-job")]
        open_job_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum OctetBaselineCommand {
    Write {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "manual")]
        created_at: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long)]
        target_next: Option<u64>,
    },
    Check {
        #[arg(long, default_value = "target/octet")]
        artifacts: PathBuf,
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long, default_value = "quarantine-ci")]
        profile: String,
        #[arg(long)]
        as_of: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long = "review")]
        reviews: Vec<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ReportCommand {
    Show {
        report: PathBuf,
    },
    Validate {
        report: PathBuf,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum GateCommand {
    Check {
        artifact: PathBuf,
        #[arg(long)]
        failure_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ReceiptCommand {
    Sign {
        receipt: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "local-signer")]
        signer: String,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
    },
    Verify {
        signed_receipt: PathBuf,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long)]
        key_ledger: Option<PathBuf>,
        #[arg(long)]
        key_ref: Option<String>,
        #[arg(long)]
        key_id: Option<String>,
        #[arg(long)]
        signer: Option<String>,
        #[arg(long)]
        subject_ref: Option<String>,
    },
}

#[derive(Debug, Clone, Args)]
struct RetentionEvidenceArgs {
    #[arg(long = "retention-requester")]
    requester_ref: Option<String>,
    #[arg(long = "retention-policy-ref")]
    policy_refs: Vec<String>,
    #[arg(long = "retention-authority-ref")]
    authority_refs: Vec<String>,
    #[arg(long = "retention-evidence-ref")]
    evidence_refs: Vec<String>,
    #[arg(long = "retention-retained-ref")]
    retained_refs: Vec<String>,
    #[arg(long = "retention-remote-peer-ref")]
    remote_peer_refs: Vec<String>,
    #[arg(long = "retention-remote-ref")]
    remote_refs: Vec<String>,
    #[arg(long = "retention-reference-index-ref")]
    reference_index_refs: Vec<String>,
    #[arg(long = "retention-remote-gc-ref")]
    remote_gc_refs: Vec<String>,
    #[arg(long = "retention-remote-clearance-ref")]
    remote_clearance_refs: Vec<String>,
    #[arg(long = "retention-reference-index-complete")]
    is_reference_index_complete: bool,
}

impl RetentionEvidenceArgs {
    fn into_retention_evidence(self) -> retention::DestructiveRetentionEvidence {
        retention::DestructiveRetentionEvidence {
            requester_ref: self.requester_ref,
            policy_refs: self.policy_refs,
            authority_refs: self.authority_refs,
            evidence_refs: self.evidence_refs,
            retained_refs: self.retained_refs,
            remote_peer_refs: self.remote_peer_refs,
            remote_refs: self.remote_refs,
            reference_index_refs: self.reference_index_refs,
            remote_gc_refs: self.remote_gc_refs,
            remote_clearance_refs: self.remote_clearance_refs,
            is_reference_index_complete: self.is_reference_index_complete,
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum LedgerCommand {
    Import {
        artifact: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Export {
        artifact_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        ledger: PathBuf,
    },
    Pin {
        artifact_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Gc {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ChainCommand {
    Publish {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        epoch: String,
        #[arg(long)]
        anchor: Option<String>,
        #[arg(long)]
        head: Option<String>,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long, default_value = "reject-unexpected-forks")]
        fork_policy: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Fetch {
        ticket: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long)]
        expected_bundle_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long, default_value = "reject-unexpected-forks")]
        fork_policy: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ChunkCommand {
    Put {
        input: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long, default_value = "artifact")]
        kind: String,
        #[arg(long, default_value_t = DEFAULT_FIXED_V1_CHUNK_SIZE)]
        chunk_size: u64,
        #[arg(long)]
        manifest_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Verify {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Read {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Range {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        offset: u64,
        #[arg(long)]
        length: u64,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Sync {
        manifest_ref: String,
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IrohPublish {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IrohFetch {
        ticket: String,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        expected_manifest_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Pin {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Unpin {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    PinChunk {
        chunk_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    UnpinChunk {
        chunk_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IndexStatus {
        #[arg(long)]
        store: PathBuf,
    },
    IndexRebuild {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ReceiptList {
        #[arg(long)]
        store: PathBuf,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        store: PathBuf,
    },
    Lineage {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        lineage_out: Option<PathBuf>,
    },
    Gc {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    Identity {
        shape: PathBuf,
        #[arg(long)]
        schema_ref: String,
        #[arg(long, default_value = "structural")]
        mode: String,
        #[arg(long)]
        brand_ref: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Alias {
        #[arg(long)]
        from_ref: String,
        #[arg(long)]
        to_ref: String,
        #[arg(long, default_value = "storage")]
        scope: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Compat {
        #[arg(long)]
        expected_identity: PathBuf,
        #[arg(long)]
        actual_identity: PathBuf,
        #[arg(long)]
        alias: Option<PathBuf>,
        #[arg(long)]
        migration_ref: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SearchFingerprint {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        fingerprint: String,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum CacheCommand {
    Put {
        input: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        operation: String,
        #[arg(long, default_value = "v1")]
        version: String,
        #[arg(long = "dependency")]
        dependencies: Vec<String>,
        #[arg(long = "dependency-closure-hash")]
        dependency_closure_hash: Option<String>,
        #[arg(long = "handler-profile-ref")]
        handler_profile_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "revocation-ref")]
        revocation_refs: Vec<String>,
        #[arg(long = "tool-ref")]
        tool_ref: Option<String>,
        #[arg(long, default_value = "local")]
        tool_version: String,
        #[arg(long = "assumption-ref")]
        assumption_refs: Vec<String>,
        #[arg(long, default_value = "pure")]
        tier: String,
        #[arg(long, default_value = "pass")]
        status: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        key_out: Option<PathBuf>,
        #[arg(long)]
        value_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Get {
        key_ref: String,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long = "current-policy-ref")]
        current_policy_refs: Vec<String>,
        #[arg(long = "current-capability-ref")]
        current_capability_refs: Vec<String>,
        #[arg(long = "current-revocation-ref")]
        current_revocation_refs: Vec<String>,
        #[arg(long = "semantic", default_value = "true")]
        semantic_enabled: bool,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        cache: PathBuf,
    },
    List {
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        operation: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "dependency-ref")]
        dependency_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_ref: Option<String>,
        #[arg(long = "capability-ref")]
        capability_ref: Option<String>,
        #[arg(long = "revocation-ref")]
        revocation_ref: Option<String>,
        #[arg(long = "evidence-ref")]
        evidence_ref: Option<String>,
    },
    Show {
        reference: String,
        #[arg(long)]
        cache: PathBuf,
    },
    Invalidate {
        #[arg(long)]
        cache: PathBuf,
        #[arg(long = "key-ref")]
        key_ref: Option<String>,
        #[arg(long = "dependency-ref")]
        dependency_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_ref: Option<String>,
        #[arg(long = "capability-ref")]
        capability_ref: Option<String>,
        #[arg(long = "revocation-ref")]
        revocation_ref: Option<String>,
        #[arg(long)]
        operation: Option<String>,
        #[arg(long, default_value = "manual-invalidate")]
        reason: String,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IndexRebuild {
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ArtifactCommand {
    Install {
        payload: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "artifact")]
        kind: String,
        #[arg(long = "dependency")]
        dependencies: Vec<String>,
        #[arg(long = "schema-ref")]
        schema_refs: Vec<String>,
        #[arg(long)]
        effect_manifest_ref: Option<String>,
        #[arg(long)]
        artifact_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        kind: Option<String>,
    },
    View {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        payload: bool,
    },
    NameSet {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "name")]
        kind: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    NameShow {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "name")]
        kind: String,
        #[arg(long)]
        name: String,
    },
    Deps {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
    },
    Closure {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Impact {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IndexRebuild {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum StorageCommand {
    Put {
        value: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        producer_ref: Option<String>,
        #[arg(long)]
        ref_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Get {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        migration_recipe: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Recipe {
        #[arg(long)]
        source_schema_ref: String,
        #[arg(long)]
        target_schema_ref: String,
        #[arg(long)]
        transformer_ref: String,
        #[arg(long, default_value = "schema-rename")]
        transformer_kind: String,
        #[arg(long, default_value = "explicit")]
        mode: String,
        #[arg(long)]
        out: PathBuf,
    },
    Migrate {
        recipe: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        ref_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Verify {
        storage_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum UpgradeCommand {
    PlanNameMove {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        from_ref: String,
        #[arg(long)]
        to_ref: String,
        #[arg(long = "source-gate-receipt")]
        source_gate_receipts: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    Create {
        plan: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SetName {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RunTask {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Rollback {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        plan_ref: String,
    },
    CleanupCheck {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum TranscriptCommand {
    Parse {
        markdown: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long = "dependency")]
        dependency_refs: Vec<String>,
        #[arg(long = "dependency-closure-hash")]
        dependency_closure_hash: Option<String>,
        #[arg(long = "handler-profile-ref")]
        handler_profile_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "revocation-ref")]
        revocation_refs: Vec<String>,
        #[arg(long = "seed-ref")]
        seed_ref: Option<String>,
        #[arg(long = "expected-ref")]
        expected_refs: Vec<String>,
    },
    Run {
        transcript: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long, default_value = "fresh")]
        state: String,
        #[arg(long)]
        save_root: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
    Show {
        transcript: PathBuf,
    },
    Render {
        transcript: PathBuf,
        #[arg(long)]
        receipt: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum RewriteCommand {
    Find {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "any")]
        pattern_kind: String,
        #[arg(long, default_value = "")]
        pattern: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        matches_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Preview {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Apply {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        upgrade_plan_out: Option<PathBuf>,
        #[arg(long, default_value = "rewrite-session")]
        session_id: String,
    },
    Show {
        artifact: PathBuf,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum CatalogCommand {
    List {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    View {
        reference: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long = "payload")]
        payload_inclusion_enabled: bool,
        #[arg(long = "redacted", default_value = "true")]
        redaction_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Search {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long = "kind")]
        artifact_kind: Option<String>,
        #[arg(long = "ledger-kind")]
        ledger_kind: Option<String>,
        #[arg(long = "schema-ref")]
        schema_ref: Option<String>,
        #[arg(long = "structural-fingerprint")]
        structural_fingerprint: Option<String>,
        #[arg(long = "effect-ref")]
        effect_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_ref: Option<String>,
        #[arg(long = "capability-ref")]
        capability_ref: Option<String>,
        #[arg(long = "evidence-ref")]
        evidence_ref: Option<String>,
        #[arg(long = "dependency-ref")]
        dependency_ref: Option<String>,
        #[arg(long = "dependent-ref")]
        dependent_ref: Option<String>,
        #[arg(long = "receipt-operation")]
        receipt_operation: Option<String>,
        #[arg(long = "receipt-decision")]
        receipt_decision: Option<String>,
        #[arg(long = "transcript-status")]
        transcript_status: Option<String>,
        #[arg(long = "upgrade-status")]
        upgrade_status: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "include-dependents", default_value = "true")]
        dependent_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Deps {
        reference: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        transitive: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Dependents {
        reference: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        transitive: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ShortId {
        prefix: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long, default_value_t = catalog::DEFAULT_SHORT_ID_MIN_LENGTH)]
        min_length: usize,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    McpCall {
        request: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum JobCommand {
    Install {
        dag: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        artifact_out: Option<PathBuf>,
    },
    Show {
        job: String,
        #[arg(long)]
        registry: PathBuf,
    },
    Run {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Plan {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Profile {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    FusionPreview {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SyncPlan {
        job: String,
        #[arg(long)]
        source_registry: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SyncLoopback {
        job: String,
        #[arg(long)]
        source_registry: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "provenance")]
        provenance_paths: Vec<PathBuf>,
        #[arg(long = "build-verification")]
        build_verification_paths: Vec<PathBuf>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    AdmitPlan {
        job: String,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        sync_ref: Option<String>,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    AdmitLoopback {
        job: String,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        sync_ref: Option<String>,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ExecuteLoopback {
        job: String,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        request_out: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    WorkerRequest {
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long)]
        execution_request: PathBuf,
        #[arg(long)]
        sync_ref: Option<String>,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "authority-ref")]
        authority_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long = "peer-bootstrap-ref")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "node-identity-ref")]
        node_identity_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    WorkerRunLocal {
        request: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long)]
        execution_request: PathBuf,
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long, default_value = "peer:source")]
        from_peer: String,
        #[arg(long, default_value = "source-worker")]
        from_actor: String,
        #[arg(long, default_value = "molten.job.worker")]
        topic: String,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    WorkerScheduleLocal {
        request: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long)]
        execution_request: PathBuf,
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long, default_value = "queue:job-worker")]
        queue_key: String,
        #[arg(long)]
        lease_key: Option<String>,
        #[arg(long, default_value = "scheduler")]
        scheduler_session: String,
        #[arg(long, default_value = "worker")]
        worker_session: String,
        #[arg(long)]
        lease_token: Option<u64>,
        #[arg(long, default_value = "peer:source")]
        from_peer: String,
        #[arg(long, default_value = "source-worker")]
        from_actor: String,
        #[arg(long, default_value = "molten.job.worker")]
        topic: String,
        #[arg(long = "coordination-authority-ref")]
        coordination_authority_refs: Vec<String>,
        #[arg(long = "coordination-resource-ref")]
        coordination_resource_refs: Vec<String>,
        #[arg(long = "coordination-policy-ref")]
        coordination_policy_refs: Vec<String>,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    RefSubmit {
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        operation_id: String,
        #[arg(long)]
        executable: String,
        #[arg(long = "input")]
        inputs: Vec<String>,
        #[arg(long, default_value = "chunk-manifest")]
        output_mode: String,
        #[arg(long = "input-schema-ref")]
        input_schema_refs: Vec<String>,
        #[arg(long = "output-schema-ref")]
        output_schema_refs: Vec<String>,
        #[arg(long = "effect-ref")]
        effect_manifest_refs: Vec<String>,
        #[arg(long, default_value = "local-echo-v1")]
        handler_profile: String,
        #[arg(long)]
        authority_context_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "provenance-ref")]
        provenance_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RefExecute {
        submission: PathBuf,
        #[arg(long)]
        chunks: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        job: Option<String>,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum RemoteCommand {
    Envelope {
        #[command(subcommand)]
        command: RemoteEnvelopeCommand,
    },
    PublishLocal {
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long)]
        envelope: PathBuf,
        #[arg(long)]
        node: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    DeliverLocal {
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        envelope_ref: String,
        #[arg(long)]
        receiver_peer: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RunTwoPeer {
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Gate {
        #[arg(long)]
        delivery_log: PathBuf,
        #[arg(long = "admission-receipt")]
        admission_receipts: Vec<PathBuf>,
        #[arg(long = "turn-context-ref")]
        turn_context_refs: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum DeliveryCommand {
    Scope {
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: String,
        #[arg(long = "retention-ref")]
        retention_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    OperationId {
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: Option<String>,
        #[arg(long)]
        scope_ref: Option<String>,
        #[arg(long)]
        producer: String,
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        payload_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Check {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: Option<String>,
        #[arg(long)]
        scope_ref: Option<String>,
        #[arg(long)]
        producer: String,
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        payload_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        semantic_result_ref: Option<String>,
        #[arg(long, default_value = "deny")]
        gap_policy: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        root: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum RetentionCommand {
    Class {
        #[arg(long)]
        class_name: String,
        #[arg(long, default_value_t = 0)]
        minimum_age_seconds: u64,
        #[arg(long)]
        maximum_age_seconds: Option<u64>,
        #[arg(long)]
        deletion_authority_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "secret-redaction-hook", default_value = "false")]
        has_secret_redaction_hook: bool,
        #[arg(long = "remote-gc-plan", default_value = "false")]
        has_remote_gc_plan: bool,
        #[arg(long = "compaction", default_value = "false")]
        has_compaction: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Pin {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        owner_ref: String,
        #[arg(long)]
        expiry_ref: Option<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value = "true")]
        has_authority: bool,
        #[arg(long)]
        pin_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Unpin {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        pin_ref: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value = "true")]
        has_authority: bool,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Admit {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long = "bound-ref")]
        bound_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "remote-ref")]
        remote_refs: Vec<String>,
        #[arg(long = "reference-index-complete")]
        is_reference_index_complete: bool,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearance {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceRequest {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceRespond {
        #[arg(long)]
        root: PathBuf,
        request: PathBuf,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceImport {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        response: PathBuf,
        #[arg(long)]
        expected_peer_ref: Option<String>,
        #[arg(long)]
        expected_remote_ref: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RemoteClearanceLiveRequestSend {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        requester_node_root: Option<PathBuf>,
        #[arg(long)]
        peer_ticket: PathBuf,
        #[arg(long)]
        requester_node_id: String,
        #[arg(long)]
        peer_node_id: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "retention-evidence-ref")]
        retention_evidence_refs: Vec<String>,
        #[arg(long = "peer-bootstrap-ref")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "transport-evidence-ref")]
        transport_evidence_refs: Vec<String>,
        #[arg(long)]
        request_out: Option<PathBuf>,
        #[arg(long)]
        control_out: Option<PathBuf>,
        #[arg(long)]
        transport_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RemoteClearanceLiveResponseSend {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        peer_node_root: Option<PathBuf>,
        #[arg(long)]
        requester_ticket: PathBuf,
        request: PathBuf,
        #[arg(long)]
        peer_node_id: String,
        #[arg(long)]
        requester_node_id: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long, default_value_t = node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS)]
        max_attempts: u64,
        #[arg(long, default_value_t = 10_000)]
        join_timeout_ms: u64,
        #[arg(long = "response-evidence-ref")]
        response_evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "peer-bootstrap-ref")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "authority")]
        authority_refs: Vec<String>,
        #[arg(long = "policy")]
        policy_refs: Vec<String>,
        #[arg(long = "resource")]
        resource_refs: Vec<String>,
        #[arg(long = "transport-evidence-ref")]
        transport_evidence_refs: Vec<String>,
        #[arg(long)]
        response_out: Option<PathBuf>,
        #[arg(long)]
        control_out: Option<PathBuf>,
        #[arg(long)]
        transport_receipt_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RemoteClearanceLiveImportWorkflow {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        response: PathBuf,
        #[arg(long)]
        request_control: PathBuf,
        #[arg(long)]
        request_send_receipt: PathBuf,
        #[arg(long)]
        request_receive_receipt: PathBuf,
        #[arg(long)]
        request_ingress_ref: String,
        #[arg(long)]
        response_control: PathBuf,
        #[arg(long)]
        response_send_receipt: PathBuf,
        #[arg(long)]
        response_receive_receipt: PathBuf,
        #[arg(long)]
        response_ingress_ref: String,
        #[arg(long)]
        expected_peer_ref: Option<String>,
        #[arg(long)]
        expected_remote_ref: Option<String>,
        #[arg(long)]
        import_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RemoteClearanceLiveLoopback {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        requester_node_root: PathBuf,
        #[arg(long)]
        peer_node_root: PathBuf,
        #[arg(long)]
        requester_node_id: String,
        #[arg(long)]
        peer_node_id: String,
        #[arg(long, default_value = node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
        topic: String,
        #[arg(long, default_value_t = 1)]
        request_sequence: u64,
        #[arg(long, default_value_t = 1)]
        response_sequence: u64,
        #[arg(long)]
        requester_ref: String,
        #[arg(long)]
        peer_ref: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        remote_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        authority_ref: String,
        #[arg(long = "retention-evidence-ref")]
        retention_evidence_refs: Vec<String>,
        #[arg(long = "response-evidence-ref")]
        response_evidence_refs: Vec<String>,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "stale")]
        is_stale: bool,
        #[arg(long = "revoked-ref")]
        revoked_refs: Vec<String>,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "request-peer-bootstrap-ref")]
        request_peer_bootstrap_refs: Vec<String>,
        #[arg(long = "request-authority-ref")]
        request_authority_refs: Vec<String>,
        #[arg(long = "request-policy-ref")]
        request_policy_refs: Vec<String>,
        #[arg(long = "request-resource-ref")]
        request_resource_refs: Vec<String>,
        #[arg(long = "request-transport-evidence-ref")]
        request_transport_evidence_refs: Vec<String>,
        #[arg(long = "response-peer-bootstrap-ref")]
        response_peer_bootstrap_refs: Vec<String>,
        #[arg(long = "response-authority-ref")]
        response_authority_refs: Vec<String>,
        #[arg(long = "response-policy-ref")]
        response_policy_refs: Vec<String>,
        #[arg(long = "response-resource-ref")]
        response_resource_refs: Vec<String>,
        #[arg(long = "response-transport-evidence-ref")]
        response_transport_evidence_refs: Vec<String>,
        #[arg(long)]
        request_out: Option<PathBuf>,
        #[arg(long)]
        response_out: Option<PathBuf>,
        #[arg(long)]
        import_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Explain {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: Option<String>,
        #[arg(long)]
        retention_class: Option<String>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long)]
        subsystem: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    BundleExport {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        explain: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "internal")]
        profile: String,
    },
    BundleVerify {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    GcPlan {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = "generic")]
        subsystem: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long, default_value = "delete")]
        action: String,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    GcApplyPlan {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        plan_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    GcAudit {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        execution_ref: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Check {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: String,
        #[arg(long)]
        retention_class: String,
        #[arg(long, default_value = "eligibility")]
        action: String,
        #[arg(long)]
        requester_ref: String,
        #[arg(long = "reference-index-complete", default_value = "true")]
        is_reference_index_complete: bool,
        #[arg(long = "retained-ref")]
        retained_refs: Vec<String>,
        #[arg(long = "remote-ref")]
        remote_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long, default_value = "false")]
        has_delete_authority: bool,
        #[arg(long = "remote-gc-clearance")]
        has_remote_gc_clearance: bool,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ProvenanceCommand {
    BuildRecord {
        #[arg(long)]
        expected_artifact_ref: String,
        #[arg(long = "source-ref")]
        source_refs: Vec<String>,
        #[arg(long)]
        dependency_closure_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long = "build-param")]
        build_params: Vec<String>,
        #[arg(long)]
        builder_ref: String,
        #[arg(long = "nix-derivation-ref")]
        nix_derivation_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    VerifyBuild {
        build_record: PathBuf,
        #[arg(long)]
        actual_artifact_ref: String,
        #[arg(long = "diagnostic")]
        prior_diagnostics: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Record {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        trust_state: String,
        #[arg(long = "source-ref")]
        source_refs: Vec<String>,
        #[arg(long)]
        dependency_closure_ref: String,
        #[arg(long = "toolchain-ref")]
        toolchain_refs: Vec<String>,
        #[arg(long)]
        builder_ref: String,
        #[arg(long = "review-ref")]
        review_refs: Vec<String>,
        #[arg(long = "test-ref")]
        test_refs: Vec<String>,
        #[arg(long = "source-gate-ref")]
        source_gate_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "build-record-ref")]
        build_record_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Fixture {
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Evaluate {
        #[arg(long)]
        operation: String,
        #[arg(long, default_value = "node-control")]
        profile: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long = "provenance")]
        provenance_paths: Vec<PathBuf>,
        #[arg(long = "build-verification")]
        build_verification_paths: Vec<PathBuf>,
        #[arg(long = "diagnostic")]
        prior_diagnostics: Vec<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Show {
        artifact: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum RemoteEnvelopeCommand {
    Build {
        #[arg(long)]
        from_peer: String,
        #[arg(long)]
        from_actor: String,
        #[arg(long)]
        to_peer: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        payload: PathBuf,
        #[arg(long = "content-ref")]
        content_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ReproCommand {
    Export {
        report: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "deny-sensitive")]
        profile: String,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
    Verify {
        bundle: PathBuf,
        #[arg(long)]
        failure_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Unpack {
        bundle: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long = "reveal-receipt")]
        reveal_receipts: Vec<PathBuf>,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
    Publish {
        bundle: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
    Fetch {
        ticket: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        expected_bundle_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        failure_out: Option<PathBuf>,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("{}", molten::greeting());
            Ok(())
        }
        Some(Command::Test { command }) => run_test_command(command),
        Some(Command::Dogfood { command }) => run_dogfood_command(command),
        Some(Command::Receipts { command }) => run_receipts_command(command),
        Some(Command::Node { command }) => run_node_command(command),
    }
}

fn run_receipts_command(command: ReceiptsCommand) -> Result<()> {
    match command {
        ReceiptsCommand::List { ledger } => {
            for entry in ledger::list_artifacts(&ledger)? {
                if is_operator_receipt_kind(&entry.artifact_kind) {
                    println!("{} {}", entry.artifact_ref, entry.artifact_kind);
                }
            }
            Ok(())
        }
        ReceiptsCommand::Show { receipt_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            let summary = validate_operator_receipt_value(&value)?;
            println!("{summary}");
            Ok(())
        }
        ReceiptsCommand::Validate { receipt_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            let summary = validate_operator_receipt_value(&value)?;
            println!(
                "receipts validate ok artifact={} kind={} summary={}",
                receipt_ref,
                ledger::artifact_kind(&value),
                summary
            );
            Ok(())
        }
        ReceiptsCommand::Export {
            receipt_ref,
            ledger,
            out,
            receipt_out,
        } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            validate_operator_receipt_value(&value)?;
            let exported = ledger::export_artifact(&ledger, &receipt_ref, &out)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts export receipt", &exported.receipt_value)?;
            println!(
                "receipts export ok artifact={} kind={} out={} redaction=pass logs=auxiliary",
                exported.artifact_ref,
                exported.artifact_kind,
                out.display()
            );
            Ok(())
        }
        ReceiptsCommand::Key { command } => run_receipt_key_command(command),
        ReceiptsCommand::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        } => {
            let receipt_value = read_preserves_file(&receipt)?;
            let signed = sign_receipt(&SignReceiptInput {
                receipt: &receipt_value,
                signer: &signer,
                purpose: &purpose,
                trust_root: &trust_root,
                key: &key,
                parents: &parents,
            })?;
            let signed_ref = molten::preserves_rail::canonical_hash(&signed)?;
            let subject_ref = molten::preserves_rail::canonical_hash(&receipt_value)?;
            write_file(&out, &to_text(&signed)?)?;
            println!(
                "receipts sign ok signed={} subject={} signer={} purpose={} out={} evidence-only=pass",
                signed_ref,
                subject_ref,
                signer,
                purpose,
                out.display()
            );
            Ok(())
        }
        ReceiptsCommand::VerifySigned {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        } => {
            let signed_value = read_preserves_file(&signed_receipt)?;
            ensure_keyring_selector_has_ledger(key_ledger.as_deref(), key_ref.as_deref(), key_id.as_deref())?;
            if let Some(ledger) = key_ledger {
                let keyring = load_signed_receipt_keyring(&ledger)?;
                let verified =
                    verify_signed_receipt_with_keyring_policy(&signed_value, &VerifySignedReceiptKeyringPolicy {
                        required_purpose: &purpose,
                        trust_root: &trust_root,
                        expected_signer: signer.as_deref(),
                        expected_subject_ref: subject_ref.as_deref(),
                        required_key_ref: key_ref.as_deref(),
                        required_key_id: key_id.as_deref(),
                        keys: &keyring.keys,
                        revocations: &keyring.revocations,
                    })?;
                println!(
                    "receipts verify-signed ok envelope={} subject={} signer={} purpose={} key={} key-id={} keyring=current evidence-only=pass",
                    verified.receipt.envelope_ref,
                    verified.receipt.subject_ref,
                    verified.receipt.signer,
                    verified.receipt.purpose,
                    verified.key_ref,
                    verified.key_id
                );
            } else {
                let verified = verify_signed_receipt_with_policy(&signed_value, &VerifySignedReceiptPolicy {
                    required_purpose: &purpose,
                    trust_root: &trust_root,
                    key: &key,
                    expected_signer: signer.as_deref(),
                    expected_subject_ref: subject_ref.as_deref(),
                })?;
                println!(
                    "receipts verify-signed ok envelope={} subject={} signer={} purpose={} evidence-only=pass",
                    verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
                );
            }
            Ok(())
        }
    }
}

struct SignedReceiptKeyring {
    keys: Vec<SignedReceiptKey>,
    revocations: Vec<SignedReceiptKeyRevocation>,
}

fn run_receipt_key_command(command: ReceiptKeyCommand) -> Result<()> {
    match command {
        ReceiptKeyCommand::Import {
            ledger,
            key_id,
            signer,
            trust_root,
            key,
            receipt_out,
        } => {
            let key_value = signed_receipt_key_value(&SignedReceiptKeyInput {
                key_id: &key_id,
                signer: &signer,
                trust_root: &trust_root,
                key: &key,
                generation: 1,
                predecessor_ref: None,
            })?;
            let imported = ledger::import_artifact(&ledger, &key_value)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts key import receipt", &imported.receipt_value)?;
            println!(
                "receipts key import ok key={} key-id={} signer={} trust-root={} status=current evidence-only=pass",
                imported.artifact_ref, key_id, signer, trust_root
            );
            Ok(())
        }
        ReceiptKeyCommand::List { ledger } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            for key in &keyring.keys {
                let is_revoked = signed_key_revocation(&keyring, &key.key_ref).is_some();
                println!(
                    "{} signed-receipt-key key-id={} signer={} trust-root={} status={} generation={} revoked={} predecessor={}",
                    key.key_ref,
                    key.key_id,
                    key.signer,
                    key.trust_root,
                    key.status,
                    key.generation,
                    is_revoked,
                    key.predecessor_ref.as_deref().unwrap_or("none")
                );
            }
            for revocation in &keyring.revocations {
                println!(
                    "{} signed-receipt-key-revocation key={} key-id={} signer={} trust-root={} reason={} superseded-by={}",
                    revocation.revocation_ref,
                    revocation.key_ref,
                    revocation.key_id,
                    revocation.signer,
                    revocation.trust_root,
                    revocation.reason,
                    revocation.superseded_by.as_deref().unwrap_or("none")
                );
            }
            Ok(())
        }
        ReceiptKeyCommand::Show { key_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &key_ref)?;
            println!("{}", signed_key_summary(&value)?);
            Ok(())
        }
        ReceiptKeyCommand::Revoke {
            key_ref,
            ledger,
            reason,
            receipt_out,
        } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            if signed_key_revocation(&keyring, &key_ref).is_some() {
                return Err(MoltenError::invalid_harness(format!("signed receipt key {key_ref} is already revoked")));
            }
            let key_value = ledger::read_artifact(&ledger, &key_ref)?;
            let key = parse_signed_receipt_key(&key_value)?;
            let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
                key: &key,
                reason: &reason,
                superseded_by: None,
            })?;
            let imported = ledger::import_artifact(&ledger, &revocation_value)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts key revoke receipt", &imported.receipt_value)?;
            println!(
                "receipts key revoke ok revocation={} key={} key-id={} signer={} reason={} evidence-only=pass",
                imported.artifact_ref, key.key_ref, key.key_id, key.signer, reason
            );
            Ok(())
        }
        ReceiptKeyCommand::Rotate {
            old_key_ref,
            ledger,
            new_key_id,
            new_key,
            reason,
            receipt_out,
        } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            if signed_key_revocation(&keyring, &old_key_ref).is_some() {
                return Err(MoltenError::invalid_harness(format!(
                    "signed receipt key {old_key_ref} is already revoked and cannot be rotated"
                )));
            }
            let old_value = ledger::read_artifact(&ledger, &old_key_ref)?;
            let old_key = parse_signed_receipt_key(&old_value)?;
            let generation = old_key
                .generation
                .checked_add(1)
                .ok_or_else(|| MoltenError::invalid_harness("signed receipt key generation overflow"))?;
            let new_value = signed_receipt_key_value(&SignedReceiptKeyInput {
                key_id: &new_key_id,
                signer: &old_key.signer,
                trust_root: &old_key.trust_root,
                key: &new_key,
                generation,
                predecessor_ref: Some(&old_key.key_ref),
            })?;
            let new_import = ledger::import_artifact(&ledger, &new_value)?;
            let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
                key: &old_key,
                reason: &reason,
                superseded_by: Some(&new_import.artifact_ref),
            })?;
            let revocation_import = ledger::import_artifact(&ledger, &revocation_value)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts key rotate receipt", &revocation_import.receipt_value)?;
            println!(
                "receipts key rotate ok old-key={} new-key={} new-key-id={} signer={} trust-root={} revocation={} evidence-only=pass",
                old_key.key_ref,
                new_import.artifact_ref,
                new_key_id,
                old_key.signer,
                old_key.trust_root,
                revocation_import.artifact_ref
            );
            Ok(())
        }
    }
}

fn ensure_keyring_selector_has_ledger(
    ledger: Option<&Path>,
    key_ref: Option<&str>,
    key_id: Option<&str>,
) -> Result<()> {
    if ledger.is_none() && (key_ref.is_some() || key_id.is_some()) {
        Err(MoltenError::invalid_harness(
            "signed receipt key selectors require --key-ledger or --signed-key-ledger",
        ))
    } else {
        Ok(())
    }
}

fn load_signed_receipt_keyring(ledger: &Path) -> Result<SignedReceiptKeyring> {
    let mut keys = Vec::with_capacity(SIGNED_KEYRING_CLI_ENTRY_LIMIT);
    let mut revocations = Vec::with_capacity(SIGNED_KEYRING_CLI_ENTRY_LIMIT);
    for entry in ledger::list_artifacts(ledger)? {
        match entry.artifact_kind.as_str() {
            "signed-receipt-key" => {
                ensure_signed_keyring_entry_count(keys.len().saturating_add(1), "signed receipt key records")?;
                keys.push(parse_signed_receipt_key(&ledger::read_artifact(ledger, &entry.artifact_ref)?)?);
            }
            "signed-receipt-key-revocation" => {
                ensure_signed_keyring_entry_count(
                    revocations.len().saturating_add(1),
                    "signed receipt key revocation records",
                )?;
                revocations
                    .push(parse_signed_receipt_key_revocation(&ledger::read_artifact(ledger, &entry.artifact_ref)?)?);
            }
            _ => {}
        }
    }
    Ok(SignedReceiptKeyring { keys, revocations })
}

fn ensure_signed_keyring_entry_count(count: usize, label: &str) -> Result<()> {
    if count > SIGNED_KEYRING_CLI_ENTRY_LIMIT {
        return Err(MoltenError::invalid_harness(format!(
            "{label} count {count} exceeds {SIGNED_KEYRING_CLI_ENTRY_LIMIT}"
        )));
    }
    Ok(())
}

fn signed_key_revocation<'a>(
    keyring: &'a SignedReceiptKeyring,
    key_ref: &str,
) -> Option<&'a SignedReceiptKeyRevocation> {
    keyring.revocations.iter().find(|revocation| revocation.key_ref == key_ref)
}

fn signed_key_summary(value: &preserves::IOValue) -> Result<String> {
    match ledger::artifact_kind(value) {
        "signed-receipt-key" => {
            let key = parse_signed_receipt_key(value)?;
            Ok(format!(
                "signed receipt key {}\nkey-id={}\nsigner={}\ntrust-root={}\nstatus={}\ngeneration={}\npredecessor={}\nevidence-only=pass",
                key.key_ref,
                key.key_id,
                key.signer,
                key.trust_root,
                key.status,
                key.generation,
                key.predecessor_ref.as_deref().unwrap_or("none")
            ))
        }
        "signed-receipt-key-revocation" => {
            let revocation = parse_signed_receipt_key_revocation(value)?;
            Ok(format!(
                "signed receipt key revocation {}\nkey={}\nkey-id={}\nsigner={}\ntrust-root={}\nreason={}\nsuperseded-by={}\nevidence-only=pass",
                revocation.revocation_ref,
                revocation.key_ref,
                revocation.key_id,
                revocation.signer,
                revocation.trust_root,
                revocation.reason,
                revocation.superseded_by.as_deref().unwrap_or("none")
            ))
        }
        kind => Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt keyring artifact kind {kind}; expected signed-receipt-key or signed-receipt-key-revocation"
        ))),
    }
}

fn validate_operator_receipt_value(value: &preserves::IOValue) -> Result<String> {
    match ledger::artifact_kind(value) {
        "dogfood-report"
        | "operator-workflow"
        | "operator-checkpoint"
        | "release-gate-receipt"
        | "nix-dogfood-release-evidence"
        | "nix-dogfood-release-verify-receipt"
        | "release-evidence-bundle"
        | "release-evidence-bundle-verify-receipt"
        | "release-promotion-gate-receipt" => operator_dogfood::operator_dogfood_summary(value),
        "signed-receipt" => signed_receipt_summary(value),
        "operator-step" => {
            let step = operator_dogfood::parse_operator_step(value)?;
            Ok(format!(
                "operator step ref={} name={} decision={} receipt={} (summary is non-normative)",
                step.step_ref,
                step.name,
                step.decision,
                step.receipt_ref.as_deref().unwrap_or("none")
            ))
        }
        kind => Err(MoltenError::invalid_harness(format!(
            "unsupported operator receipt kind {kind}; expected dogfood/operator receipt artifact"
        ))),
    }
}

fn is_operator_receipt_kind(kind: &str) -> bool {
    matches!(
        kind,
        "dogfood-report"
            | "operator-workflow"
            | "operator-step"
            | "operator-checkpoint"
            | "release-gate-receipt"
            | "nix-dogfood-release-evidence"
            | "nix-dogfood-release-verify-receipt"
            | "release-evidence-bundle"
            | "release-evidence-bundle-verify-receipt"
            | "release-promotion-gate-receipt"
            | "signed-receipt"
    )
}

fn run_test_command(command: TestCommand) -> Result<()> {
    match command {
        TestCommand::Run { suite, report_out } => {
            let suite_text = match fs::read_to_string(&suite).map_err(MoltenError::from) {
                Ok(suite_text) => suite_text,
                Err(error) => {
                    write_optional_failure(report_out.as_ref(), "preflight", &error, None)?;
                    return Err(error);
                }
            };
            let suite_value = match parse_text(&suite_text) {
                Ok(suite_value) => suite_value,
                Err(error) => {
                    write_optional_failure(report_out.as_ref(), "preflight", &error, None)?;
                    return Err(error);
                }
            };
            let run = match run_suite_value(&suite_value) {
                Ok(run) => run,
                Err(error) => {
                    let phase = run_failure_phase(&error);
                    write_optional_suite_failure(report_out.as_ref(), phase, &error, &suite_value)?;
                    return Err(error);
                }
            };
            let report_text = to_text(&run.report_value)?;
            if let Some(path) = report_out {
                write_file(&path, &report_text)?;
                println!("report {} written to {}", run.report_ref, path.display());
            } else {
                println!("{report_text}");
                eprintln!("report {}", run.report_ref);
            }
            Ok(())
        }
        TestCommand::Replay { report, failure_out } => {
            let report_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "replay")?;
            let replay = match replay_report_value(&report_value) {
                Ok(replay) => replay,
                Err(error) => {
                    write_optional_report_failure(failure_out.as_ref(), "replay", &error, &report_value)?;
                    return Err(error);
                }
            };
            println!(
                "replay ok expected={} actual={} final_state={}",
                replay.expected_report_ref, replay.actual_report_ref, replay.final_state_hash
            );
            Ok(())
        }
        TestCommand::Report { command } => run_report_command(command),
        TestCommand::Gate { command } => run_gate_command(command),
        TestCommand::Receipt { command } => run_receipt_command(command),
        TestCommand::Ledger { command } => run_ledger_command(command),
        TestCommand::Chain { command } => run_chain_command(command),
        TestCommand::Chunk { command } => run_chunk_command(command),
        TestCommand::Storage { command } => run_storage_command(command),
        TestCommand::Artifact { command } => run_artifact_command(command),
        TestCommand::Schema { command } => run_schema_command(command),
        TestCommand::Cache { command } => run_cache_command(command),
        TestCommand::Upgrade { command } => run_upgrade_command(command),
        TestCommand::Transcript { command } => run_transcript_command(command),
        TestCommand::Rewrite { command } => run_rewrite_command(command),
        TestCommand::Catalog { command } => run_catalog_command(command),
        TestCommand::Job { command } => run_job_command(command),
        TestCommand::Remote { command } => run_remote_command(command),
        TestCommand::Delivery { command } => run_delivery_command(command),
        TestCommand::Retention { command } => run_retention_command(command),
        TestCommand::Provenance { command } => run_provenance_command(command),
        TestCommand::Protocol { command } => run_protocol_command(command),
        TestCommand::Raft { command } => run_raft_command(command),
        TestCommand::Plugin { command } => run_plugin_command(command),
        TestCommand::Coordination { command } => run_coordination_command(command),
        TestCommand::Secrets { command } => run_secrets_command(command),
        TestCommand::Service { command } => run_service_command(command),
        TestCommand::Octet { command } => run_octet_command(command),
        TestCommand::Node { command } => run_node_command(command),
        TestCommand::Repro { command } => run_repro_command(command),
    }
}

fn run_report_command(command: ReportCommand) -> Result<()> {
    match command {
        ReportCommand::Show { report } => {
            let report_value = read_preserves_file(&report)?;
            println!("{}", report_show_summary(&report_value)?);
            Ok(())
        }
        ReportCommand::Validate { report, failure_out } => {
            let report_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "validate")?;
            let validation = match validate_report_value(&report_value) {
                Ok(validation) => validation,
                Err(error) => {
                    write_optional_report_failure(failure_out.as_ref(), "validate", &error, &report_value)?;
                    return Err(error);
                }
            };
            let replay = match replay_report_value(&report_value) {
                Ok(replay) => replay,
                Err(error) => {
                    write_optional_report_failure(failure_out.as_ref(), "validate", &error, &report_value)?;
                    return Err(error);
                }
            };
            println!(
                "report validate ok report={} suite={} observations={} final_state={} replay_actual={}",
                validation.report_ref,
                validation.suite_ref,
                validation.observations,
                validation.final_state_hash,
                replay.actual_report_ref
            );
            Ok(())
        }
    }
}

fn report_show_summary(report_value: &preserves::IOValue) -> Result<String> {
    let report_error = match report_summary(report_value) {
        Ok(summary) => return Ok(summary),
        Err(error) => error,
    };
    if let Ok(summary) = failure_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = repro_bundle_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = gate_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = repro_verify_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = signed_receipt_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = secrets::fixture_report_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = secrets::secrets_summary(report_value) {
        return Ok(summary);
    }
    if let Ok(summary) = remote_dataspace_gate_summary(report_value) {
        return Ok(summary);
    }
    Err(report_error)
}

fn run_gate_command(command: GateCommand) -> Result<()> {
    match command {
        GateCommand::Check {
            artifact,
            failure_out,
            receipt_out,
        } => {
            let artifact_value = read_preserves_file_with_failure(&artifact, failure_out.as_ref(), "validate")?;
            let check = match gate_check_value(&artifact_value) {
                Ok(check) => check,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "validate", &error, &artifact_value)?;
                    return Err(error);
                }
            };
            let receipt = gate_receipt_value(&check);
            if let Err(error) = emit_gate_receipt(receipt_out.as_ref(), &receipt) {
                write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
                return Err(error);
            }
            Ok(())
        }
    }
}

fn run_receipt_command(command: ReceiptCommand) -> Result<()> {
    match command {
        ReceiptCommand::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        } => {
            let receipt_value = read_preserves_file(&receipt)?;
            let signed = sign_receipt(&SignReceiptInput {
                receipt: &receipt_value,
                signer: &signer,
                purpose: &purpose,
                trust_root: &trust_root,
                key: &key,
                parents: &parents,
            })?;
            write_file(&out, &to_text(&signed)?)?;
            println!("signed receipt written to {}", out.display());
            Ok(())
        }
        ReceiptCommand::Verify {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        } => {
            let signed_value = read_preserves_file(&signed_receipt)?;
            ensure_keyring_selector_has_ledger(key_ledger.as_deref(), key_ref.as_deref(), key_id.as_deref())?;
            if let Some(ledger) = key_ledger {
                let keyring = load_signed_receipt_keyring(&ledger)?;
                let verified =
                    verify_signed_receipt_with_keyring_policy(&signed_value, &VerifySignedReceiptKeyringPolicy {
                        required_purpose: &purpose,
                        trust_root: &trust_root,
                        expected_signer: signer.as_deref(),
                        expected_subject_ref: subject_ref.as_deref(),
                        required_key_ref: key_ref.as_deref(),
                        required_key_id: key_id.as_deref(),
                        keys: &keyring.keys,
                        revocations: &keyring.revocations,
                    })?;
                println!(
                    "signed receipt verify ok envelope={} subject={} signer={} purpose={} key={} key-id={}",
                    verified.receipt.envelope_ref,
                    verified.receipt.subject_ref,
                    verified.receipt.signer,
                    verified.receipt.purpose,
                    verified.key_ref,
                    verified.key_id
                );
            } else {
                let verified = verify_signed_receipt_with_policy(&signed_value, &VerifySignedReceiptPolicy {
                    required_purpose: &purpose,
                    trust_root: &trust_root,
                    key: &key,
                    expected_signer: signer.as_deref(),
                    expected_subject_ref: subject_ref.as_deref(),
                })?;
                println!(
                    "signed receipt verify ok envelope={} subject={} signer={} purpose={}",
                    verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
                );
            }
            Ok(())
        }
    }
}

fn run_ledger_command(command: LedgerCommand) -> Result<()> {
    match command {
        LedgerCommand::Import {
            artifact,
            ledger,
            receipt_out,
        } => {
            let artifact_value = read_preserves_file(&artifact)?;
            let imported = ledger::import_artifact(&ledger, &artifact_value)?;
            emit_named_receipt(receipt_out.as_ref(), "ledger import receipt", &imported.receipt_value)?;
            println!(
                "ledger import ok artifact={} kind={} ledger={}",
                imported.artifact_ref,
                imported.artifact_kind,
                ledger.display()
            );
            Ok(())
        }
        LedgerCommand::Export {
            artifact_ref,
            ledger,
            out,
            receipt_out,
        } => {
            let exported = ledger::export_artifact(&ledger, &artifact_ref, &out)?;
            emit_named_receipt(receipt_out.as_ref(), "ledger export receipt", &exported.receipt_value)?;
            println!(
                "ledger export ok artifact={} kind={} out={}",
                exported.artifact_ref,
                exported.artifact_kind,
                out.display()
            );
            Ok(())
        }
        LedgerCommand::List { ledger } => {
            for entry in ledger::list_artifacts(&ledger)? {
                println!("{} {}", entry.artifact_ref, entry.artifact_kind);
            }
            Ok(())
        }
        LedgerCommand::Pin { artifact_ref, ledger } => {
            ledger::pin_artifact(&ledger, &artifact_ref)?;
            println!("ledger pin ok artifact={} ledger={}", artifact_ref, ledger.display());
            Ok(())
        }
        LedgerCommand::Gc {
            ledger,
            dry_run,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let retention_evidence = retention.into_retention_evidence();
            let gc = ledger::gc(&ledger, ledger::LedgerGcInput {
                dry_run,
                retention_evidence: &retention_evidence,
                apply_refs: &apply_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "ledger gc receipt", &gc.receipt_value)?;
            println!(
                "ledger gc ok decision={} dry_run={} removed={} retention_receipts={}",
                gc.decision,
                gc.dry_run,
                gc.removed_refs.len(),
                gc.retention_receipt_refs.len()
            );
            Ok(())
        }
    }
}

fn run_chain_command(command: ChainCommand) -> Result<()> {
    match command {
        ChainCommand::Publish {
            ledger,
            iroh_store,
            scope,
            id,
            epoch,
            anchor,
            head,
            node,
            fork_policy,
            receipt_out,
        } => {
            let chain = ChainScope::new(scope, id, epoch);
            let policy = parse_chain_fork_policy(&fork_policy)?;
            let published = publish_chain_segment(&PublishChainSegmentInput {
                iroh_root: &iroh_store,
                ledger_root: &ledger,
                chain: &chain,
                anchor_ref: anchor.as_deref(),
                expected_head: head.as_deref(),
                node: &node,
                fork_policy: policy,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "iroh chain exchange receipt", &published.receipt_value)?;
            println!(
                "chain publish ok ticket={} bundle={} chain={}/{}/{}",
                published.ticket,
                published.bundle_ref,
                published.chain.scope,
                published.chain.id,
                published.chain.epoch
            );
            Ok(())
        }
        ChainCommand::Fetch {
            ticket,
            ledger,
            iroh_store,
            expected_bundle_ref,
            peer,
            fork_policy,
            receipt_out,
        } => {
            let policy = parse_chain_fork_policy(&fork_policy)?;
            let fetched = fetch_chain_segment(&FetchChainSegmentInput {
                iroh_root: &iroh_store,
                ticket: &ticket,
                expected_bundle_ref: expected_bundle_ref.as_deref(),
                peer: &peer,
                ledger_root: &ledger,
                fork_policy: policy,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "iroh chain exchange receipt", &fetched.receipt_value)?;
            println!(
                "chain fetch ok ticket={} bundle={} chain={}/{}/{}",
                fetched.ticket, fetched.bundle_ref, fetched.chain.scope, fetched.chain.id, fetched.chain.epoch
            );
            Ok(())
        }
    }
}

fn parse_chain_fork_policy(value: &str) -> Result<ChainForkPolicy> {
    match value {
        "reject-unexpected-forks" | "production" | "reject" => Ok(ChainForkPolicy::RejectUnexpectedForks),
        "retain-fork-evidence" | "diagnostic" | "retain" => Ok(ChainForkPolicy::RetainForkEvidence),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported chain fork policy {other}; expected reject-unexpected-forks or retain-fork-evidence"
        ))),
    }
}

fn run_chunk_command(command: ChunkCommand) -> Result<()> {
    match command {
        ChunkCommand::Put {
            input,
            store,
            kind,
            chunk_size,
            manifest_out,
            receipt_out,
        } => {
            let bytes = fs::read(&input).map_err(MoltenError::from)?;
            let put = chunk_store::put_bytes(&store, &kind, &bytes, chunk_size)?;
            if let Some(path) = manifest_out.as_ref() {
                write_file(path, &to_text(&put.manifest_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &put.receipt_value)?;
            println!(
                "chunk put ok manifest={} chunks={} bytes={} store={}",
                put.manifest_ref,
                put.chunk_refs.len(),
                put.total_len,
                store.display()
            );
            Ok(())
        }
        ChunkCommand::Verify {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let verified = chunk_store::verify_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &verified.receipt_value)?;
            println!(
                "chunk verify ok manifest={} chunks={} bytes={}",
                verified.manifest_ref,
                verified.chunk_refs.len(),
                verified.total_len
            );
            Ok(())
        }
        ChunkCommand::Read {
            manifest_ref,
            store,
            out,
            receipt_out,
        } => {
            let read = chunk_store::read_object(&store, &manifest_ref)?;
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(MoltenError::from)?;
            }
            fs::write(&out, &read.bytes).map_err(MoltenError::from)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
            println!("chunk read ok manifest={} bytes={} out={}", read.manifest_ref, read.bytes.len(), out.display());
            Ok(())
        }
        ChunkCommand::Range {
            manifest_ref,
            store,
            offset,
            length,
            out,
            receipt_out,
        } => {
            let read = chunk_store::range_read(&store, &manifest_ref, offset, length)?;
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(MoltenError::from)?;
            }
            fs::write(&out, &read.bytes).map_err(MoltenError::from)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
            println!(
                "chunk range ok manifest={} offset={} length={} out={}",
                read.manifest_ref,
                read.offset,
                read.length,
                out.display()
            );
            Ok(())
        }
        ChunkCommand::Sync {
            manifest_ref,
            from,
            store,
            receipt_out,
        } => {
            let sync = chunk_store::sync_missing_chunks(&from, &store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &sync.receipt_value)?;
            println!(
                "chunk sync ok manifest={} missing_before={} fetched={}",
                sync.manifest_ref,
                sync.missing_before.len(),
                sync.fetched_chunks.len()
            );
            Ok(())
        }
        ChunkCommand::IrohPublish {
            manifest_ref,
            store,
            iroh_store,
            node,
            receipt_out,
        } => {
            let published = chunk_store::publish_iroh_blobs(&store, &iroh_store, &manifest_ref, &node)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &published.receipt_value)?;
            println!(
                "chunk iroh-publish ok manifest={} chunks={} ticket={} iroh_store={}",
                published.manifest_ref,
                published.chunk_blob_refs.len(),
                published.ticket,
                iroh_store.display()
            );
            Ok(())
        }
        ChunkCommand::IrohFetch {
            ticket,
            iroh_store,
            store,
            expected_manifest_ref,
            peer,
            receipt_out,
        } => {
            let fetched =
                chunk_store::fetch_iroh_blobs(&iroh_store, &store, &ticket, expected_manifest_ref.as_deref(), &peer)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &fetched.receipt_value)?;
            println!(
                "chunk iroh-fetch ok manifest={} missing_before={} fetched={} store={}",
                fetched.manifest_ref,
                fetched.missing_before.len(),
                fetched.fetched_chunks.len(),
                store.display()
            );
            Ok(())
        }
        ChunkCommand::Pin {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::pin_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk pin ok manifest={} store={}", manifest_ref, store.display());
            Ok(())
        }
        ChunkCommand::Unpin {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::unpin_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk unpin ok manifest={} store={}", manifest_ref, store.display());
            Ok(())
        }
        ChunkCommand::PinChunk {
            chunk_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::pin_chunk(&store, &chunk_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk pin-chunk ok chunk={} store={}", chunk_ref, store.display());
            Ok(())
        }
        ChunkCommand::UnpinChunk {
            chunk_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::unpin_chunk(&store, &chunk_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk unpin-chunk ok chunk={} store={}", chunk_ref, store.display());
            Ok(())
        }
        ChunkCommand::IndexStatus { store } => {
            let status = chunk_store::index_status(&store)?;
            println!(
                "chunk index status manifests={} chunks={} available={} missing={} manifest_pins={} chunk_pins={} partial_fetches={} receipts={} store={}",
                status.manifests,
                status.chunks,
                status.available_chunks,
                status.missing_chunks,
                status.manifest_pins,
                status.chunk_pins,
                status.partial_fetches,
                status.receipts,
                store.display()
            );
            Ok(())
        }
        ChunkCommand::IndexRebuild { store, receipt_out } => {
            let rebuild = chunk_store::rebuild_index(&store)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &rebuild.receipt_value)?;
            println!(
                "chunk index rebuild ok manifests={} chunks={} available={} missing={} receipts={} store={}",
                rebuild.status.manifests,
                rebuild.status.chunks,
                rebuild.status.available_chunks,
                rebuild.status.missing_chunks,
                rebuild.status.receipts,
                store.display()
            );
            Ok(())
        }
        ChunkCommand::ReceiptList { store } => {
            let refs = chunk_store::list_receipt_refs(&store)?;
            for receipt_ref in &refs {
                println!("{receipt_ref}");
            }
            println!("chunk receipt-list ok receipts={} store={}", refs.len(), store.display());
            Ok(())
        }
        ChunkCommand::ReceiptShow { receipt_ref, store } => {
            let receipt = chunk_store::read_receipt(&store, &receipt_ref)?;
            println!("{}", to_text(&receipt.value)?);
            eprintln!(
                "chunk receipt-show ok receipt={} operation={} decision={} chunks={} store={}",
                receipt.receipt_ref,
                receipt.operation,
                receipt.decision,
                receipt.chunk_refs.len(),
                store.display()
            );
            Ok(())
        }
        ChunkCommand::Lineage {
            manifest_ref,
            store,
            lineage_out,
        } => {
            let lineage = chunk_store::build_chunk_lineage(&store, &manifest_ref)?;
            emit_named_receipt(lineage_out.as_ref(), "chunk lineage", &lineage.value)?;
            println!(
                "chunk lineage ok lineage={} manifest={} links={} receipts={} predicates={}",
                lineage.lineage_ref,
                lineage.manifest_ref,
                lineage.link_refs.len(),
                lineage.receipt_refs.len(),
                lineage.predicate_receipt_refs.len()
            );
            Ok(())
        }
        ChunkCommand::Gc {
            store,
            dry_run,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let retention_evidence = retention.into_retention_evidence();
            let gc = chunk_store::gc(&store, chunk_store::ChunkStoreGcInput {
                dry_run,
                retention_evidence: &retention_evidence,
                apply_refs: &apply_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &gc.receipt_value)?;
            println!(
                "chunk gc ok decision={} dry_run={} removed_manifests={} removed_chunks={} retention_receipts={}",
                gc.decision,
                gc.dry_run,
                gc.removed_manifests.len(),
                gc.removed_chunks.len(),
                gc.retention_receipt_refs.len()
            );
            Ok(())
        }
    }
}

fn run_schema_command(command: SchemaCommand) -> Result<()> {
    match command {
        SchemaCommand::Identity {
            shape,
            schema_ref,
            mode,
            brand_ref,
            out,
            receipt_out,
        } => {
            let shape = read_preserves_file(&shape)?;
            let value = schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
                mode,
                schema_ref,
                shape,
                brand_ref,
                metadata_refs: vec![cli_schema_ref("metadata", "identity")?],
                policy_refs: vec![cli_schema_ref("policy", "identity")?],
                evidence_refs: vec![cli_schema_ref("evidence", "identity")?],
            })?;
            let identity = schema_identity::parse_schema_identity(&value)?;
            let receipt = schema_identity::compatibility_receipt_value(
                "fingerprint",
                &schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                    expected: identity.clone(),
                    actual: identity.clone(),
                    alias: None,
                    migration_ref: None,
                    policy_refs: identity.policy_refs.clone(),
                    evidence_refs: identity.evidence_refs.clone(),
                    deny_by_policy: false,
                })?,
            )?;
            write_file(&out, &to_text(&value)?)?;
            emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
            println!(
                "schema identity ok identity={} schema={} fingerprint={} out={}",
                identity.identity_ref,
                identity.schema_ref,
                identity.structural_fingerprint,
                out.display()
            );
            Ok(())
        }
        SchemaCommand::Alias {
            from_ref,
            to_ref,
            scope,
            out,
            receipt_out,
        } => {
            let value = schema_identity::schema_alias_value(&schema_identity::SchemaAliasInput {
                from_schema_ref: from_ref,
                to_schema_ref: to_ref,
                scope,
                policy_refs: vec![cli_schema_ref("policy", "alias")?],
                evidence_refs: vec![cli_schema_ref("evidence", "alias")?],
            })?;
            let alias = schema_identity::parse_schema_alias(&value)?;
            let expected = local_unique_schema_identity(&alias.to_schema_ref)?;
            let actual = local_unique_schema_identity(&alias.from_schema_ref)?;
            let compatibility =
                schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                    expected,
                    actual,
                    alias: Some(alias.clone()),
                    migration_ref: None,
                    policy_refs: alias.policy_refs.clone(),
                    evidence_refs: alias.evidence_refs.clone(),
                    deny_by_policy: false,
                })?;
            let receipt = schema_identity::compatibility_receipt_value("alias-admit", &compatibility)?;
            write_file(&out, &to_text(&value)?)?;
            emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
            println!(
                "schema alias ok alias={} from={} to={} out={}",
                alias.alias_ref,
                alias.from_schema_ref,
                alias.to_schema_ref,
                out.display()
            );
            Ok(())
        }
        SchemaCommand::Compat {
            expected_identity,
            actual_identity,
            alias,
            migration_ref,
            out,
            receipt_out,
        } => {
            let expected = schema_identity::parse_schema_identity(&read_preserves_file(&expected_identity)?)?;
            let actual = schema_identity::parse_schema_identity(&read_preserves_file(&actual_identity)?)?;
            let alias = alias
                .as_ref()
                .map(|path| read_preserves_file(path).and_then(|value| schema_identity::parse_schema_alias(&value)))
                .transpose()?;
            let compatibility =
                schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                    expected,
                    actual,
                    alias,
                    migration_ref,
                    policy_refs: vec![cli_schema_ref("policy", "compat")?],
                    evidence_refs: vec![cli_schema_ref("evidence", "compat")?],
                    deny_by_policy: false,
                })?;
            let parsed = schema_identity::parse_schema_compatibility(&compatibility)?;
            let receipt = schema_identity::compatibility_receipt_value("compatibility", &compatibility)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &to_text(&compatibility)?)?;
            } else {
                println!("{}", to_text(&compatibility)?);
            }
            emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
            eprintln!("schema compat ok decision={} compatibility={}", parsed.decision, parsed.compatibility_ref);
            Ok(())
        }
        SchemaCommand::SearchFingerprint { registry, fingerprint } => {
            for identity in schema_identity::search_registry_by_fingerprint(&registry, &fingerprint)? {
                println!("{} {} {}", identity.identity_ref, identity.schema_ref, identity.mode);
            }
            Ok(())
        }
    }
}

fn local_unique_schema_identity(schema_ref: &str) -> Result<schema_identity::SchemaIdentity> {
    let shape = record("shape", vec![string("any-preserves")]);
    let value = schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
        mode: schema_identity::MODE_UNIQUE.to_string(),
        schema_ref: schema_ref.to_string(),
        shape,
        brand_ref: None,
        metadata_refs: vec![cli_schema_ref("metadata", schema_ref)?],
        policy_refs: vec![cli_schema_ref("policy", schema_ref)?],
        evidence_refs: vec![cli_schema_ref("evidence", schema_ref)?],
    })?;
    schema_identity::parse_schema_identity(&value)
}

fn cli_schema_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("schema-cli-ref", vec![string(kind), string(label)]))
}

fn run_cache_command(command: CacheCommand) -> Result<()> {
    match command {
        CacheCommand::Put {
            input,
            cache,
            output,
            operation,
            version,
            dependencies,
            dependency_closure_hash,
            handler_profile_ref,
            policy_refs,
            capability_refs,
            revocation_refs,
            tool_ref,
            tool_version,
            mut assumption_refs,
            tier,
            status,
            evidence_refs,
            diagnostics,
            key_out,
            value_out,
            receipt_out,
        } => {
            let input_value = read_preserves_file(&input)?;
            let output_value = output.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let tool_ref = match tool_ref {
                Some(tool_ref) => tool_ref,
                None => cli_cache_ref("tool", &operation)?,
            };
            if matches!(status.as_str(), eval_cache::STATUS_DENY | eval_cache::STATUS_ERROR) {
                for evidence_ref in &evidence_refs {
                    if !assumption_refs.contains(evidence_ref)
                        && !policy_refs.contains(evidence_ref)
                        && !capability_refs.contains(evidence_ref)
                        && !revocation_refs.contains(evidence_ref)
                    {
                        assumption_refs.push(evidence_ref.clone());
                    }
                }
            }
            let closure_hash = match dependency_closure_hash {
                Some(hash) => hash,
                None => canonical_hash(&record("eval-cache-cli-closure", vec![
                    string(&operation),
                    preserves_sequence_strings(&dependencies),
                ]))?,
            };
            let key_input = eval_cache::EvalCacheKeyInput {
                operation: operation.clone(),
                version,
                input_ref: canonical_hash(&input_value)?,
                dependency_closure_hash: closure_hash,
                dependency_refs: dependencies,
                handler_profile_ref,
                policy_refs: policy_refs.clone(),
                capability_refs,
                revocation_refs,
                tool_ref,
                tool_version,
                assumption_refs,
            };
            let value_input = eval_cache::EvalCacheValueInput {
                tier,
                status,
                output: output_value,
                dependency_refs: key_input.dependency_refs.clone(),
                policy_refs,
                evidence_refs,
                diagnostics,
            };
            let put = eval_cache::put(&cache, &key_input, &value_input)?;
            if let Some(path) = key_out.as_ref() {
                write_file(path, &to_text(&put.key.value)?)?;
            }
            if let Some(path) = value_out.as_ref() {
                write_file(path, &to_text(&put.value.value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &put.receipt_value)?;
            println!(
                "cache put ok key={} value={} operation={} tier={} status={} cache={}",
                put.key.key_ref,
                put.value.value_ref,
                put.key.operation,
                put.value.tier,
                put.value.status,
                cache.display()
            );
            Ok(())
        }
        CacheCommand::Get {
            key_ref,
            cache,
            current_policy_refs,
            current_capability_refs,
            current_revocation_refs,
            semantic_enabled,
            out,
            receipt_out,
        } => {
            let get = eval_cache::get(&cache, &key_ref, &eval_cache::EvalCacheGetInput {
                current_policy_refs,
                current_capability_refs,
                current_revocation_refs,
                semantic: semantic_enabled,
            })?;
            if let Some(output) = get.output.as_ref() {
                let text = to_text(output)?;
                if let Some(path) = out.as_ref() {
                    write_file(path, &text)?;
                } else {
                    println!("{text}");
                }
            } else if out.is_none() {
                println!("<none>");
            }
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &get.receipt_value)?;
            eprintln!(
                "cache get ok key={} value={} status={} tier={} cache={}",
                get.key.key_ref,
                get.value.value_ref,
                get.value.status,
                get.value.tier,
                cache.display()
            );
            Ok(())
        }
        CacheCommand::Status { cache } => {
            let status = eval_cache::status(&cache)?;
            println!(
                "keys={} values={} tombstones={} receipts={} tiers[pure={},simulated={},policy-current={},trace-only={}] statuses[pass={},deny={},error={},trace-only={}]",
                status.keys,
                status.values,
                status.tombstones,
                status.receipts,
                status.pure,
                status.simulated,
                status.policy_current,
                status.trace_only_tier,
                status.pass,
                status.deny,
                status.error,
                status.trace_only_status
            );
            Ok(())
        }
        CacheCommand::List {
            cache,
            operation,
            tier,
            status,
            dependency_ref,
            policy_ref,
            capability_ref,
            revocation_ref,
            evidence_ref,
        } => {
            for entry in eval_cache::list(&cache, &eval_cache::EvalCacheListFilter {
                operation,
                tier,
                status,
                dependency_ref,
                policy_ref,
                capability_ref,
                revocation_ref,
                evidence_ref,
            })? {
                println!(
                    "{} {} {} {} tombstoned={}",
                    entry.key_ref, entry.value_ref, entry.operation, entry.status, entry.tombstoned
                );
            }
            Ok(())
        }
        CacheCommand::Show { reference, cache } => {
            if let Ok(key) = eval_cache::read_key(&cache, &reference) {
                println!("{}", to_text(&key.value)?);
                return Ok(());
            }
            for entry in eval_cache::list(&cache, &eval_cache::EvalCacheListFilter {
                operation: None,
                tier: None,
                status: None,
                dependency_ref: None,
                policy_ref: None,
                capability_ref: None,
                revocation_ref: None,
                evidence_ref: None,
            })? {
                if entry.value_ref == reference {
                    let value = eval_cache::read_value(&cache, &entry.key_ref)?;
                    println!("{}", to_text(&value.value)?);
                    return Ok(());
                }
            }
            let receipt = eval_cache::read_receipt(&cache, &reference)?;
            println!("{}", to_text(&receipt.value)?);
            Ok(())
        }
        CacheCommand::Invalidate {
            cache,
            key_ref,
            dependency_ref,
            policy_ref,
            capability_ref,
            revocation_ref,
            operation,
            reason,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let invalidated = eval_cache::invalidate(&cache, &eval_cache::EvalCacheInvalidateInput {
                key_ref,
                dependency_ref,
                policy_ref,
                capability_ref,
                revocation_ref,
                operation,
                reason,
                retention_evidence: retention.into_retention_evidence(),
                apply_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &invalidated.receipt_value)?;
            for key_ref in &invalidated.invalidated_key_refs {
                println!("{key_ref}");
            }
            eprintln!(
                "cache invalidate ok decision={} keys={} retention_receipts={} cache={}",
                invalidated.decision,
                invalidated.invalidated_key_refs.len(),
                invalidated.retention_receipt_refs.len(),
                cache.display()
            );
            Ok(())
        }
        CacheCommand::IndexRebuild { cache, receipt_out } => {
            let receipt = eval_cache::rebuild_index(&cache)?;
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &receipt)?;
            println!("cache index-rebuild ok cache={}", cache.display());
            Ok(())
        }
    }
}

fn cli_cache_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("eval-cache-cli-ref", vec![string(kind), string(label)]))
}

fn preserves_sequence_strings(values: &[String]) -> preserves::IOValue {
    molten::preserves_rail::sequence(values.iter().map(string).collect())
}

fn run_artifact_command(command: ArtifactCommand) -> Result<()> {
    match command {
        ArtifactCommand::Install {
            payload,
            registry,
            kind,
            dependencies,
            schema_refs,
            effect_manifest_ref,
            artifact_out,
            receipt_out,
        } => {
            let payload = read_preserves_file(&payload)?;
            let schemas = if schema_refs.is_empty() {
                vec![cli_artifact_ref("schema", &kind)?]
            } else {
                schema_refs
            };
            let install = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
                kind: kind.clone(),
                payload,
                schema_refs: schemas,
                dependency_refs: dependencies,
                effect_manifest_ref,
                policy_refs: vec![cli_artifact_ref("policy", &kind)?],
                evidence_refs: vec![cli_artifact_ref("evidence", &kind)?],
                installer_ref: cli_artifact_ref("installer", &kind)?,
                capability_refs: vec![cli_artifact_ref("capability", &kind)?],
            })?;
            if let Some(path) = artifact_out.as_ref() {
                write_file(path, &to_text(&install.artifact.value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &install.receipt_value)?;
            println!(
                "artifact install {} artifact={} kind={} registry={}",
                install.decision,
                install.artifact_ref,
                install.artifact.kind,
                registry.display()
            );
            Ok(())
        }
        ArtifactCommand::List { registry, kind } => {
            for artifact in artifacts::list_artifacts(&registry, kind.as_deref())? {
                println!("{} {}", artifact.artifact_ref, artifact.kind);
            }
            Ok(())
        }
        ArtifactCommand::View {
            artifact_ref,
            registry,
            payload,
        } => {
            if payload {
                println!("{}", to_text(&artifacts::read_payload(&registry, &artifact_ref)?)?);
            } else {
                let artifact = artifacts::read_artifact(&registry, &artifact_ref)?;
                println!("{}", to_text(&artifact.value)?);
            }
            Ok(())
        }
        ArtifactCommand::NameSet {
            registry,
            kind,
            name,
            artifact_ref,
            receipt_out,
        } => {
            let policy_refs = [cli_artifact_ref("policy", &name)?];
            let evidence_refs = [cli_artifact_ref("evidence", &name)?];
            let pointer = artifacts::set_name_pointer(&registry, &artifacts::SetNamePointerInput {
                pointer_kind: &kind,
                name: &name,
                artifact_ref: &artifact_ref,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })?;
            let receipt = artifacts::read_receipt(&registry, &pointer.receipt_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &receipt.value)?;
            println!(
                "artifact name-set ok kind={} name={} artifact={} pointer={}",
                pointer.pointer_kind, pointer.name, pointer.artifact_ref, pointer.pointer_ref
            );
            Ok(())
        }
        ArtifactCommand::NameShow { registry, kind, name } => {
            let pointer = artifacts::read_name_pointer(&registry, &kind, &name)?
                .ok_or_else(|| MoltenError::invalid_harness(format!("artifact pointer {kind}:{name} not found")))?;
            println!("{} {} {}", pointer.pointer_kind, pointer.name, pointer.artifact_ref);
            Ok(())
        }
        ArtifactCommand::Deps { artifact_ref, registry } => {
            for dependency in artifacts::direct_dependencies(&registry, &artifact_ref)? {
                println!("{dependency}");
            }
            Ok(())
        }
        ArtifactCommand::Closure {
            artifact_ref,
            registry,
            receipt_out,
        } => {
            let closure = artifacts::dependency_closure(&registry, &[artifact_ref])?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &closure.receipt_value)?;
            for reference in &closure.closure_refs {
                println!("{reference}");
            }
            if !closure.missing_refs.is_empty() {
                eprintln!("missing dependencies: {}", closure.missing_refs.join(","));
            }
            eprintln!("artifact closure {} refs={}", closure.closure_hash, closure.closure_refs.len());
            Ok(())
        }
        ArtifactCommand::Impact {
            artifact_ref,
            registry,
            receipt_out,
        } => {
            let impact = artifacts::impact(&registry, &[artifact_ref])?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &impact.receipt_value)?;
            for reference in &impact.impacted_refs {
                println!("{reference}");
            }
            eprintln!("artifact impact {} refs={}", impact.impact_hash, impact.impacted_refs.len());
            Ok(())
        }
        ArtifactCommand::IndexRebuild { registry, receipt_out } => {
            let rebuild = artifacts::rebuild_index(&registry)?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &rebuild.receipt_value)?;
            println!(
                "artifact index-rebuild ok artifacts={} names={} registry={}",
                rebuild.artifacts,
                rebuild.names,
                registry.display()
            );
            Ok(())
        }
    }
}

fn cli_artifact_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("artifact-cli-ref", vec![string(kind), string(label)]))
}

fn run_storage_command(command: StorageCommand) -> Result<()> {
    match command {
        StorageCommand::Put {
            value,
            store,
            namespace,
            key,
            schema_ref,
            producer_ref,
            ref_out,
            receipt_out,
        } => {
            let value = read_preserves_file(&value)?;
            let producer_ref = match producer_ref {
                Some(producer_ref) => producer_ref,
                None => cli_storage_ref("producer", &namespace, &key)?,
            };
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
            let put = typed_storage::put_value(&store, &typed_storage::TypedStoragePutInput {
                namespace: namespace.clone(),
                key: key.clone(),
                schema_ref,
                value,
                producer_ref,
                policy_refs: vec![admission.policy_ref.clone()],
                evidence_refs: admission.evidence_refs.clone(),
                admission,
            })?;
            if let Some(path) = ref_out.as_ref() {
                write_file(path, &to_text(&put.typed_ref_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &put.receipt_value)?;
            println!(
                "storage put ok namespace={} key={} storage_ref={} schema_ref={} value_ref={} store={}",
                namespace,
                key,
                put.storage_ref,
                put.schema_ref,
                put.value_ref,
                store.display()
            );
            Ok(())
        }
        StorageCommand::Get {
            store,
            namespace,
            key,
            schema_ref,
            migration_recipe,
            out,
            receipt_out,
        } => {
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
            let get = if let Some(migration_recipe) = migration_recipe.as_ref() {
                let expected_schema_ref = schema_ref.as_deref().ok_or_else(|| {
                    MoltenError::invalid_harness("storage get --migration-recipe requires --schema-ref target")
                })?;
                let recipe_value = read_preserves_file(migration_recipe)?;
                typed_storage::get_value_with_migration(typed_storage::MigrationGetInput {
                    root: &store,
                    namespace: &namespace,
                    key: &key,
                    expected_schema_ref,
                    migration_recipe_value: &recipe_value,
                    admission: &admission,
                })?
            } else {
                typed_storage::get_value(&store, &namespace, &key, schema_ref.as_deref(), &admission)?
            };
            let text = to_text(&get.value)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &text)?;
            } else {
                println!("{text}");
            }
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &get.receipt_value)?;
            eprintln!(
                "storage get ok namespace={} key={} storage_ref={} schema_ref={} store={}",
                namespace,
                key,
                get.storage_ref,
                get.typed_ref.schema_ref,
                store.display()
            );
            Ok(())
        }
        StorageCommand::Recipe {
            source_schema_ref,
            target_schema_ref,
            transformer_ref,
            transformer_kind,
            mode,
            out,
        } => {
            let recipe = typed_storage::migration_recipe_value(&typed_storage::StorageMigrationRecipeInput {
                source_schema_ref,
                target_schema_ref,
                transformer_ref,
                transformer_kind,
                mode,
                policy_refs: vec![cli_storage_ref("migration-policy", "recipe", "policy")?],
                evidence_refs: vec![cli_storage_ref("migration-evidence", "recipe", "evidence")?],
            })?;
            write_file(&out, &to_text(&recipe)?)?;
            println!("storage recipe ok recipe_ref={} out={}", canonical_hash(&recipe)?, out.display());
            Ok(())
        }
        StorageCommand::Migrate {
            recipe,
            store,
            namespace,
            key,
            ref_out,
            receipt_out,
        } => {
            let recipe = read_preserves_file(&recipe)?;
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
            let migrated = typed_storage::migrate_value(&store, &namespace, &key, &recipe, &admission)?;
            if let Some(path) = ref_out.as_ref() {
                write_file(path, &to_text(&migrated.typed_ref_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &migrated.receipt_value)?;
            println!(
                "storage migrate ok namespace={} key={} old_ref={} new_ref={} recipe={} store={}",
                namespace,
                key,
                migrated.old_storage_ref,
                migrated.new_storage_ref,
                migrated.recipe_ref,
                store.display()
            );
            Ok(())
        }
        StorageCommand::Verify {
            storage_ref,
            store,
            schema_ref,
            receipt_out,
        } => {
            let verified = typed_storage::verify_ref(&store, &storage_ref, schema_ref.as_deref())?;
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &verified.receipt_value)?;
            println!(
                "storage verify ok storage_ref={} namespace={} key={} schema_ref={} store={}",
                verified.storage_ref,
                verified.typed_ref.namespace,
                verified.typed_ref.key,
                verified.typed_ref.schema_ref,
                store.display()
            );
            Ok(())
        }
    }
}

fn cli_storage_ref(kind: &str, namespace: &str, key: &str) -> Result<String> {
    canonical_hash(&record("typed-storage-cli-ref", vec![string(kind), string(namespace), string(key)]))
}

fn run_upgrade_command(command: UpgradeCommand) -> Result<()> {
    match command {
        UpgradeCommand::PlanNameMove {
            ledger,
            registry,
            session_id,
            name,
            from_ref,
            to_ref,
            source_gate_receipts,
            out,
        } => {
            if source_gate_receipts.is_empty() {
                return Err(MoltenError::invalid_harness(
                    "upgrade plan-name-move requires --source-gate-receipt for strict Octet source-gate validation",
                ));
            }
            let source_gate_receipt_values =
                source_gate_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let plan = upgrades::name_move_plan_value_with_registry(
                registry.as_deref(),
                &ledger,
                &upgrades::NameMovePlanInput {
                    session_id,
                    name: name.clone(),
                    from_ref,
                    to_ref,
                    initiator_ref: cli_upgrade_ref("initiator", &name)?,
                    capability_refs: vec![cli_upgrade_ref("capability", &name)?],
                    policy_refs: vec![cli_upgrade_ref("policy", &name)?],
                    evidence_refs: vec![cli_upgrade_ref("transcript", &name)?],
                    source_gate_receipt_values,
                },
            )?;
            let plan_ref = canonical_hash(&plan)?;
            write_file(&out, &to_text(&plan)?)?;
            println!("upgrade plan-name-move ok plan={} out={}", plan_ref, out.display());
            Ok(())
        }
        UpgradeCommand::Create {
            plan,
            store,
            receipt_out,
        } => {
            let plan_value = read_preserves_file(&plan)?;
            let created = upgrades::create_session(&store, &plan_value)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &created.receipt.value)?;
            println!(
                "upgrade create ok session={} plan={} tasks={} store={}",
                created.plan.session_id,
                created.plan.plan_ref,
                created.plan.tasks.len(),
                store.display()
            );
            Ok(())
        }
        UpgradeCommand::SetName {
            store,
            name,
            artifact_ref,
            receipt_out,
        } => {
            let receipt = upgrades::set_name_pointer(&store, &name, &artifact_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!("upgrade set-name ok name={} artifact={} store={}", name, artifact_ref, store.display());
            Ok(())
        }
        UpgradeCommand::RunTask {
            store,
            ledger,
            plan_ref,
            task_id,
            receipt_out,
        } => {
            let executed = upgrades::execute_task(&store, &ledger, &plan_ref, &task_id)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &executed.receipt.value)?;
            println!(
                "upgrade run-task ok plan={} task={} kind={} decision={}",
                executed.plan_ref, executed.task_id, executed.task_kind, executed.receipt.decision
            );
            Ok(())
        }
        UpgradeCommand::Rollback {
            store,
            plan_ref,
            task_id,
            receipt_out,
        } => {
            let receipt = upgrades::rollback_task(&store, &plan_ref, &task_id)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!(
                "upgrade rollback {} plan={} task={} receipt={}",
                receipt.decision, plan_ref, task_id, receipt.receipt_ref
            );
            Ok(())
        }
        UpgradeCommand::Status { store, plan_ref } => {
            let status = upgrades::status(&store, &plan_ref)?;
            println!(
                "upgrade status session={} plan={} remaining={}",
                status.session_id,
                status.plan_ref,
                status.remaining_task_ids.len()
            );
            for task in status.tasks {
                println!(
                    "{} {} {} {}",
                    task.task_id,
                    task.kind,
                    if task.done { "done" } else { "todo" },
                    task.receipt_ref.unwrap_or_else(|| "-".to_string())
                );
            }
            Ok(())
        }
        UpgradeCommand::CleanupCheck {
            store,
            ledger,
            registry,
            artifact_ref,
            receipt_out,
        } => {
            let receipt =
                upgrades::cleanup_admission_with_registry(&store, &ledger, registry.as_deref(), &artifact_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "upgrade receipt", &receipt.value)?;
            println!(
                "upgrade cleanup-check {} artifact={} receipt={}",
                receipt.decision, artifact_ref, receipt.receipt_ref
            );
            Ok(())
        }
    }
}

fn cli_upgrade_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("upgrade-cli-ref", vec![string(kind), string(label)]))
}

fn run_transcript_command(command: TranscriptCommand) -> Result<()> {
    match command {
        TranscriptCommand::Parse {
            markdown,
            out,
            dependency_refs,
            dependency_closure_hash,
            handler_profile_ref,
            policy_refs,
            capability_refs,
            revocation_refs,
            seed_ref,
            expected_refs,
        } => {
            let source = fs::read_to_string(&markdown).map_err(MoltenError::from)?;
            let transcript = transcripts::parse_markdown(&source, &transcripts::TranscriptParseInput {
                dependency_refs,
                dependency_closure_hash,
                handler_profile_ref,
                policy_refs,
                capability_refs,
                revocation_refs,
                seed_ref,
                expected_refs,
            })?;
            write_file(&out, &to_text(&transcript.value)?)?;
            println!(
                "transcript parse ok transcript={} stanzas={} out={}",
                transcript.transcript_ref,
                transcript.stanzas.len(),
                out.display()
            );
            Ok(())
        }
        TranscriptCommand::Run {
            transcript,
            cache,
            state,
            save_root,
            out,
            receipt_out,
            failure_out,
        } => {
            let artifact = match read_transcript_input(&transcript) {
                Ok(artifact) => artifact,
                Err(error) => {
                    write_optional_failure(failure_out.as_ref(), "parse", &error, None)?;
                    return Err(error);
                }
            };
            let mode = transcripts::TranscriptRunMode::parse(&state)?;
            let run = transcripts::run_transcript(&artifact, &transcripts::TranscriptRunInput {
                mode,
                cache_root: cache,
                save_root,
            })?;
            if let Some(path) = out.as_ref() {
                write_file(path, &transcripts::render_transcript(&artifact, Some(&run))?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "transcript run receipt", &run.receipt_value)?;
            eprintln!(
                "transcript run decision={} transcript={} receipt={}",
                run.decision, run.transcript_ref, run.receipt_ref
            );
            if run.decision == "deny" || run.decision == "error" {
                let error = MoltenError::invalid_harness(format!("transcript run decision {}", run.decision));
                write_optional_failure(failure_out.as_ref(), "run", &error, Some(vec![run.receipt_value]))?;
                return Err(error);
            }
            Ok(())
        }
        TranscriptCommand::Show { transcript } => {
            let artifact = read_transcript_input(&transcript)?;
            println!("{}", to_text(&artifact.value)?);
            Ok(())
        }
        TranscriptCommand::Render {
            transcript,
            receipt,
            out,
        } => {
            let artifact = read_transcript_input(&transcript)?;
            let run = receipt
                .as_ref()
                .map(|path| {
                    let receipt_value = read_preserves_file(path)?;
                    let receipt = transcripts::parse_transcript_run_receipt(&receipt_value)?;
                    Ok::<transcripts::TranscriptRun, MoltenError>(transcripts::TranscriptRun {
                        transcript_ref: receipt.transcript_ref,
                        decision: receipt.decision,
                        stanza_outcomes: Vec::new(),
                        receipt_ref: receipt.receipt_ref,
                        receipt_value,
                        cache_receipt_value: None,
                        state_root: None,
                    })
                })
                .transpose()?;
            write_file(&out, &transcripts::render_transcript(&artifact, run.as_ref())?)?;
            println!("transcript render ok transcript={} out={}", artifact.transcript_ref, out.display());
            Ok(())
        }
    }
}

fn run_rewrite_command(command: RewriteCommand) -> Result<()> {
    match command {
        RewriteCommand::Find {
            registry,
            pattern_kind,
            pattern,
            artifact_kinds,
            root_refs,
            dependency_inclusion_enabled,
            hidden_refs,
            matches_out,
            receipt_out,
        } => {
            let query = rewrite_query(RewriteQueryCliInput {
                pattern_kind,
                pattern,
                artifact_kinds,
                root_refs,
                dependency_inclusion_enabled,
                hidden_refs,
            })?;
            let found = rewrites::find(&registry, &query)?;
            if let Some(path) = matches_out.as_ref() {
                let value = record("rewrite-matches", vec![molten::preserves_rail::sequence(
                    found.matches.iter().map(|rewrite_match| rewrite_match.value.clone()).collect(),
                )]);
                write_file(path, &to_text(&value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &found.receipt_value)?;
            for rewrite_match in &found.matches {
                println!("{} {} {}", rewrite_match.artifact_ref, rewrite_match.kind, rewrite_match.bindings.len());
            }
            eprintln!("rewrite find matches={} query={}", found.matches.len(), found.query_ref);
            Ok(())
        }
        RewriteCommand::Preview {
            registry,
            from,
            to,
            artifact_kinds,
            root_refs,
            dependency_inclusion_enabled,
            hidden_refs,
            plan_out,
            receipt_out,
        } => {
            let input = rewrite_plan_input(RewritePlanCliInput {
                from,
                to,
                artifact_kinds,
                root_refs,
                dependency_inclusion_enabled,
                hidden_refs,
            })?;
            let preview = rewrites::preview(&registry, &input)?;
            if let Some(path) = plan_out.as_ref() {
                write_file(path, &to_text(&preview.plan_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &preview.receipt_value)?;
            for diff in &preview.diffs {
                println!("{} {} {}", diff.artifact_ref, diff.old_payload_ref, diff.new_payload_ref);
            }
            eprintln!(
                "rewrite preview decision={} diffs={} plan={}",
                if preview.diffs.is_empty() { "deny" } else { "pass" },
                preview.diffs.len(),
                preview.plan_ref
            );
            Ok(())
        }
        RewriteCommand::Apply {
            registry,
            from,
            to,
            artifact_kinds,
            root_refs,
            dependency_inclusion_enabled,
            hidden_refs,
            plan_out,
            receipt_out,
            upgrade_plan_out,
            session_id,
        } => {
            let input = rewrite_plan_input(RewritePlanCliInput {
                from,
                to,
                artifact_kinds,
                root_refs,
                dependency_inclusion_enabled,
                hidden_refs,
            })?;
            let applied = rewrites::apply(&registry, &input)?;
            if let Some(path) = plan_out.as_ref() {
                write_file(path, &to_text(&applied.preview.plan_value)?)?;
            }
            if let Some(path) = upgrade_plan_out.as_ref() {
                let upgrade_plan = rewrites::upgrade_plan_from_apply(
                    &applied,
                    &session_id,
                    &cli_rewrite_ref("initiator", &session_id)?,
                    &[cli_rewrite_ref("capability", &session_id)?],
                    &[cli_rewrite_ref("policy", &session_id)?],
                )?;
                write_file(path, &to_text(&upgrade_plan)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &applied.receipt_value)?;
            for installed in &applied.installed {
                println!(
                    "{} {} {}",
                    installed.old_artifact_ref, installed.new_artifact_ref, installed.install_receipt_ref
                );
            }
            eprintln!("rewrite apply installed={} plan={}", applied.installed.len(), applied.preview.plan_ref);
            Ok(())
        }
        RewriteCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", rewrites::rewrite_summary(&value)?);
            Ok(())
        }
    }
}

struct RewriteQueryCliInput {
    pattern_kind: String,
    pattern: String,
    artifact_kinds: Vec<String>,
    root_refs: Vec<String>,
    dependency_inclusion_enabled: bool,
    hidden_refs: Vec<String>,
}

struct RewritePlanCliInput {
    from: String,
    to: String,
    artifact_kinds: Vec<String>,
    root_refs: Vec<String>,
    dependency_inclusion_enabled: bool,
    hidden_refs: Vec<String>,
}

fn rewrite_query(input: RewriteQueryCliInput) -> Result<rewrites::RewriteQueryInput> {
    Ok(rewrites::RewriteQueryInput {
        artifact_kinds: input.artifact_kinds,
        root_refs: input.root_refs,
        include_dependencies: input.dependency_inclusion_enabled,
        pattern: cli_rewrite_pattern(&input.pattern_kind, &input.pattern)?,
        policy_refs: vec![cli_rewrite_ref("query-policy", &input.pattern_kind)?],
        capability_refs: vec![cli_rewrite_ref("query-capability", &input.pattern_kind)?],
        hidden_refs: input.hidden_refs,
    })
}

fn rewrite_plan_input(input: RewritePlanCliInput) -> Result<rewrites::RewritePlanInput> {
    Ok(rewrites::RewritePlanInput {
        query: rewrite_query(RewriteQueryCliInput {
            pattern_kind: "string-equals".to_string(),
            pattern: input.from.clone(),
            artifact_kinds: input.artifact_kinds,
            root_refs: input.root_refs,
            dependency_inclusion_enabled: input.dependency_inclusion_enabled,
            hidden_refs: input.hidden_refs,
        })?,
        replacement: rewrites::RewriteReplacement::StringValue {
            from: input.from.clone(),
            to: input.to,
        },
        planner_ref: cli_rewrite_ref("planner", &input.from)?,
        policy_refs: vec![cli_rewrite_ref("plan-policy", &input.from)?],
        capability_refs: vec![cli_rewrite_ref("plan-capability", &input.from)?],
        transcript_refs: vec![cli_rewrite_ref("transcript", &input.from)?],
        schema_migration_recipe_refs: vec![cli_rewrite_ref("schema-migration", &input.from)?],
    })
}

fn cli_rewrite_pattern(kind: &str, pattern: &str) -> Result<rewrites::RewritePattern> {
    match kind {
        "any" => Ok(rewrites::RewritePattern::Any),
        "artifact-kind" => Ok(rewrites::RewritePattern::ArtifactKind(pattern.to_string())),
        "record-label" => Ok(rewrites::RewritePattern::RecordLabel(pattern.to_string())),
        "string-equals" => Ok(rewrites::RewritePattern::StringEquals(pattern.to_string())),
        "string-contains" => Ok(rewrites::RewritePattern::StringContains(pattern.to_string())),
        "schema-shape-kind" => Ok(rewrites::RewritePattern::SchemaShapeKind(pattern.to_string())),
        "ref-contains" => Ok(rewrites::RewritePattern::RefContains(pattern.to_string())),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported rewrite pattern kind {other}; expected any, artifact-kind, record-label, string-equals, \
             string-contains, schema-shape-kind, or ref-contains"
        ))),
    }
}

fn cli_rewrite_ref(kind: &str, label: &str) -> Result<String> {
    rewrites::default_local_ref(kind, label)
}

fn run_catalog_command(command: CatalogCommand) -> Result<()> {
    match command {
        CatalogCommand::List {
            registry,
            ledger,
            kind,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::list(&registry, ledger.as_deref(), &catalog::CatalogListInput {
                kind,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            eprintln!("catalog list items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        CatalogCommand::View {
            reference,
            registry,
            ledger,
            payload_inclusion_enabled,
            redaction_enabled,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::view(&registry, ledger.as_deref(), &catalog::CatalogViewInput {
                reference,
                include_payload: payload_inclusion_enabled,
                redacted: redaction_enabled,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            Ok(())
        }
        CatalogCommand::Search {
            registry,
            ledger,
            artifact_kind,
            ledger_kind,
            schema_ref,
            structural_fingerprint,
            effect_ref,
            policy_ref,
            capability_ref,
            evidence_ref,
            dependency_ref,
            dependent_ref,
            receipt_operation,
            receipt_decision,
            transcript_status,
            upgrade_status,
            text,
            root_refs,
            dependency_inclusion_enabled,
            dependent_inclusion_enabled,
            hidden_refs,
            receipt_out,
        } => {
            let filters = catalog_filters(CatalogFilterCliInput {
                artifact_kind,
                ledger_kind,
                schema_ref,
                structural_fingerprint,
                effect_ref,
                policy_ref,
                capability_ref,
                evidence_ref,
                dependency_ref,
                dependent_ref,
                receipt_operation,
                receipt_decision,
                transcript_status,
                upgrade_status,
                text,
            });
            let result = catalog::search(&registry, ledger.as_deref(), &catalog::CatalogSearchInput {
                root_refs,
                include_dependencies: dependency_inclusion_enabled,
                include_dependents: dependent_inclusion_enabled,
                filters,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            eprintln!("catalog search items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        CatalogCommand::Deps {
            reference,
            registry,
            ledger,
            transitive,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::dependencies(&registry, ledger.as_deref(), &catalog::CatalogGraphInput {
                reference,
                transitive,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            Ok(())
        }
        CatalogCommand::Dependents {
            reference,
            registry,
            ledger,
            transitive,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::dependents(&registry, ledger.as_deref(), &catalog::CatalogGraphInput {
                reference,
                transitive,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            Ok(())
        }
        CatalogCommand::ShortId {
            prefix,
            registry,
            ledger,
            min_length,
            hidden_refs,
            receipt_out,
        } => {
            let resolution = catalog::resolve_short_id(&registry, ledger.as_deref(), &catalog::CatalogShortIdInput {
                prefix,
                min_length,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &resolution.receipt_value)?;
            println!("{}", to_text(&resolution.value)?);
            if let Some(full_ref) = resolution.full_ref.as_ref() {
                eprintln!("catalog short-id {} -> {}", resolution.prefix, full_ref);
            } else {
                eprintln!("catalog short-id {} decision={}", resolution.prefix, resolution.decision);
            }
            Ok(())
        }
        CatalogCommand::McpCall {
            request,
            registry,
            ledger,
            out,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let call = catalog_mcp::call(&registry, ledger.as_deref(), &request_value)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &to_text(&call.response_value)?)?;
            } else {
                println!("{}", to_text(&call.response_value)?);
            }
            emit_named_receipt(receipt_out.as_ref(), "catalog MCP receipt", &call.receipt_value)?;
            eprintln!("catalog MCP call decision={} response={}", call.decision, call.response_ref);
            Ok(())
        }
        CatalogCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            match catalog::catalog_summary(&value) {
                Ok(summary) => println!("{summary}"),
                Err(_) => println!("{}", catalog_mcp::catalog_mcp_summary(&value)?),
            }
            Ok(())
        }
    }
}

fn catalog_visibility(hidden_refs: Vec<String>) -> catalog::CatalogVisibilityInput {
    catalog::CatalogVisibilityInput {
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        hidden_refs,
        redaction_profile_ref: None,
    }
}

struct CatalogFilterCliInput {
    artifact_kind: Option<String>,
    ledger_kind: Option<String>,
    schema_ref: Option<String>,
    structural_fingerprint: Option<String>,
    effect_ref: Option<String>,
    policy_ref: Option<String>,
    capability_ref: Option<String>,
    evidence_ref: Option<String>,
    dependency_ref: Option<String>,
    dependent_ref: Option<String>,
    receipt_operation: Option<String>,
    receipt_decision: Option<String>,
    transcript_status: Option<String>,
    upgrade_status: Option<String>,
    text: Option<String>,
}

fn catalog_filters(input: CatalogFilterCliInput) -> Vec<catalog::CatalogFilter> {
    let mut filters = Vec::new();
    if let Some(value) = input.artifact_kind {
        filters.push(catalog::CatalogFilter::ArtifactKind(value));
    }
    if let Some(value) = input.ledger_kind {
        filters.push(catalog::CatalogFilter::LedgerKind(value));
    }
    if let Some(value) = input.schema_ref {
        filters.push(catalog::CatalogFilter::SchemaRef(value));
    }
    if let Some(value) = input.structural_fingerprint {
        filters.push(catalog::CatalogFilter::StructuralFingerprint(value));
    }
    if let Some(value) = input.effect_ref {
        filters.push(catalog::CatalogFilter::EffectRef(value));
    }
    if let Some(value) = input.policy_ref {
        filters.push(catalog::CatalogFilter::PolicyRef(value));
    }
    if let Some(value) = input.capability_ref {
        filters.push(catalog::CatalogFilter::CapabilityRef(value));
    }
    if let Some(value) = input.evidence_ref {
        filters.push(catalog::CatalogFilter::EvidenceRef(value));
    }
    if let Some(value) = input.dependency_ref {
        filters.push(catalog::CatalogFilter::DependencyRef(value));
    }
    if let Some(value) = input.dependent_ref {
        filters.push(catalog::CatalogFilter::DependentRef(value));
    }
    if let Some(value) = input.receipt_operation {
        filters.push(catalog::CatalogFilter::ReceiptOperation(value));
    }
    if let Some(value) = input.receipt_decision {
        filters.push(catalog::CatalogFilter::ReceiptDecision(value));
    }
    if let Some(value) = input.transcript_status {
        filters.push(catalog::CatalogFilter::TranscriptStatus(value));
    }
    if let Some(value) = input.upgrade_status {
        filters.push(catalog::CatalogFilter::UpgradeStatus(value));
    }
    if let Some(value) = input.text {
        filters.push(catalog::CatalogFilter::Text(value));
    }
    filters
}

fn print_catalog_items(items: &[preserves::IOValue]) -> Result<()> {
    for item in items {
        println!("{}", to_text(item)?);
    }
    Ok(())
}

fn run_job_command(command: JobCommand) -> Result<()> {
    match command {
        JobCommand::Install {
            dag,
            registry,
            receipt_out,
            artifact_out,
        } => {
            let value = read_preserves_file(&dag)?;
            let installed = job_dag::install_job_dag(&registry, &value)?;
            if let Some(path) = artifact_out.as_ref() {
                write_file(path, &to_text(&value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "job receipt", &installed.receipt_value)?;
            println!(
                "job install {} job={} artifact={} registry={}",
                installed.decision,
                installed.job_ref,
                installed.artifact_ref,
                registry.display()
            );
            Ok(())
        }
        JobCommand::Show { job, registry } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            println!("{}", job_dag::dag_summary(&dag));
            println!("{}", to_text(&dag.value)?);
            Ok(())
        }
        JobCommand::Run {
            job,
            registry,
            storage,
            cache,
            chunks,
            ledger,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let chunk_root = chunks.unwrap_or_else(|| registry.join("job-chunks"));
            let run = job_dag::run_job_dag(&dag, &job_dag::JobRunOptions {
                registry_root: &registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                ledger_root: ledger.as_deref(),
                output_request: request,
            })?;
            let output_text = to_text(&run.output_value)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &output_text)?;
            } else {
                println!("{output_text}");
            }
            emit_named_receipt(receipt_out.as_ref(), "job receipt", &run.receipt_value)?;
            eprintln!(
                "job run ok job={} request={} outputs={} stages={}",
                run.job_ref,
                run.request_ref,
                run.output_refs.len(),
                run.stage_receipt_refs.len()
            );
            Ok(())
        }
        JobCommand::Plan {
            job,
            registry,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let plan = job_dag::plan_job_dag(&dag, request.as_ref())?;
            emit_job_analysis(&plan.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job plan receipt", &plan.receipt_value)?;
            eprintln!("job plan ok job={} plan={} stages={}", plan.job_ref, plan.plan_ref, plan.stage_order.len());
            Ok(())
        }
        JobCommand::Profile {
            job,
            registry,
            cache,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let profile = job_dag::profile_job_dag(&dag, request.as_ref(), cache.as_deref())?;
            emit_job_analysis(&profile.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job profile receipt", &profile.receipt_value)?;
            eprintln!(
                "job profile ok job={} profile={} stages={} edges={}",
                profile.job_ref, profile.profile_ref, profile.stage_count, profile.edge_count
            );
            Ok(())
        }
        JobCommand::FusionPreview {
            job,
            registry,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let fusion = job_dag::fusion_preview_job_dag(&dag, request.as_ref())?;
            emit_job_analysis(&fusion.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job fusion receipt", &fusion.receipt_value)?;
            eprintln!(
                "job fusion-preview ok job={} fusion={} chains={}",
                fusion.job_ref,
                fusion.fusion_ref,
                fusion.chains.len()
            );
            Ok(())
        }
        JobCommand::SyncPlan {
            job,
            source_registry,
            target_registry,
            target_peer,
            stages,
            out,
            receipt_out,
        } => {
            let request = job_sync_cli_request(&source_registry, &job, &stages, &target_peer, &[])?;
            let plan = job_dag::sync_plan_value(&source_registry, &target_registry, &request)?;
            emit_job_analysis(&plan.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job sync receipt", &plan.receipt_value)?;
            eprintln!(
                "job sync-plan ok job={} plan={} missing={}",
                plan.request.job_ref,
                plan.plan_ref,
                plan.missing_refs.len()
            );
            Ok(())
        }
        JobCommand::SyncLoopback {
            job,
            source_registry,
            target_registry,
            target_peer,
            stages,
            provenance_paths,
            build_verification_paths,
            plan_out,
            receipt_out,
        } => {
            let provenance_values = read_preserves_files(&provenance_paths)?;
            let build_verification_values = read_preserves_files(&build_verification_paths)?;
            let mut evidence_refs = values_canonical_refs(&provenance_values)?;
            evidence_refs.extend(values_canonical_refs(&build_verification_values)?);
            let request = job_sync_cli_request(&source_registry, &job, &stages, &target_peer, &evidence_refs)?;
            let synced = job_dag::sync_loopback(job_dag::SyncLoopbackInput {
                source_registry: &source_registry,
                target_registry: &target_registry,
                request_value: &request,
                provenance_values: &provenance_values,
                build_verification_values: &build_verification_values,
            })?;
            emit_job_analysis(&synced.plan.value, plan_out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job sync receipt", &synced.receipt_value)?;
            eprintln!(
                "job sync-loopback decision={} job={} installed={} already_present={}",
                synced.decision,
                synced.plan.request.job_ref,
                synced.installed_refs.len(),
                synced.already_present_refs.len()
            );
            Ok(())
        }
        JobCommand::AdmitPlan {
            job,
            target_registry,
            sync_ref,
            target_peer,
            stages,
            policy_refs,
            capability_refs,
            evidence_refs,
            resource_refs,
            out,
            receipt_out,
        } => {
            let request = job_admission_cli_request(JobAdmissionCliInput {
                target_registry: &target_registry,
                job: &job,
                sync_ref: sync_ref.as_deref(),
                stages: &stages,
                target_peer: &target_peer,
                policy_refs,
                capability_refs,
                evidence_refs,
                resource_refs,
            })?;
            let plan = job_dag::admission_plan_value(&target_registry, &request)?;
            emit_job_analysis(&plan.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job admission receipt", &plan.receipt_value)?;
            eprintln!(
                "job admit-plan {} job={} plan={} stages={}",
                plan.decision,
                plan.request.job_ref,
                plan.plan_ref,
                plan.stage_order.len()
            );
            Ok(())
        }
        JobCommand::AdmitLoopback {
            job,
            target_registry,
            sync_ref,
            target_peer,
            stages,
            policy_refs,
            capability_refs,
            evidence_refs,
            resource_refs,
            plan_out,
            receipt_out,
        } => {
            let request = job_admission_cli_request(JobAdmissionCliInput {
                target_registry: &target_registry,
                job: &job,
                sync_ref: sync_ref.as_deref(),
                stages: &stages,
                target_peer: &target_peer,
                policy_refs,
                capability_refs,
                evidence_refs,
                resource_refs,
            })?;
            let admitted = job_dag::admission_loopback(&target_registry, &request)?;
            emit_job_analysis(&admitted.plan.value, plan_out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job admission receipt", &admitted.receipt_value)?;
            eprintln!(
                "job admit-loopback {} job={} receipt={} stages={}",
                admitted.plan.decision,
                admitted.plan.request.job_ref,
                admitted.receipt_ref,
                admitted.plan.stage_order.len()
            );
            Ok(())
        }
        JobCommand::ExecuteLoopback {
            job,
            target_registry,
            storage,
            cache,
            chunks,
            admission_receipt,
            target_peer,
            stages,
            policy_refs,
            capability_refs,
            resource_refs,
            request_out,
            out,
            receipt_out,
        } => {
            let admission_value = match read_preserves_file(&admission_receipt) {
                Ok(value) => value,
                Err(error) => {
                    let request = job_execution_cli_request_from_admission_ref(JobExecutionFromAdmissionCliInput {
                        target_registry: &target_registry,
                        job: &job,
                        admission_ref: None,
                        target_peer: &target_peer,
                        stages: &stages,
                        policy_refs,
                        capability_refs,
                        resource_refs,
                    })?;
                    if let Some(path) = request_out.as_ref() {
                        write_file(path, &to_text(&request)?)?;
                    }
                    let receipt = job_dag::missing_admission_execution_receipt_value(&request, &error.to_string())?;
                    emit_named_receipt(receipt_out.as_ref(), "job execution receipt", &receipt)?;
                    return Err(error);
                }
            };
            let request = job_execution_cli_request(JobExecutionCliInput {
                target_registry: &target_registry,
                job: &job,
                admission_value: &admission_value,
                target_peer: &target_peer,
                stages: &stages,
                policy_refs,
                capability_refs,
                resource_refs,
            })?;
            if let Some(path) = request_out.as_ref() {
                write_file(path, &to_text(&request)?)?;
            }
            let chunk_root = chunks.unwrap_or_else(|| target_registry.join("job-chunks"));
            let executed = job_dag::execution_loopback(job_dag::ExecutionLoopbackInput {
                target_registry: &target_registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                admission_receipt_value: &admission_value,
                request_value: &request,
            })?;
            if let Some(run) = executed.run.as_ref() {
                let output_text = to_text(&run.output_value)?;
                if let Some(path) = out.as_ref() {
                    write_file(path, &output_text)?;
                } else {
                    println!("{output_text}");
                }
            }
            emit_named_receipt(receipt_out.as_ref(), "job execution receipt", &executed.receipt_value)?;
            if executed.decision == "pass" {
                eprintln!(
                    "job execute-loopback pass job={} receipt={} outputs={}",
                    executed.request.job_ref,
                    executed.receipt_ref,
                    executed.run.as_ref().map(|run| run.output_refs.len()).unwrap_or_default()
                );
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job execute-loopback denied: {}",
                    executed.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::WorkerRequest {
            admission_receipt,
            execution_request,
            sync_ref,
            target_peer,
            stages,
            authority_refs,
            resource_refs,
            peer_bootstrap_refs,
            node_identity_refs,
            evidence_refs,
            out,
        } => {
            let admission_value = read_preserves_file(&admission_receipt)?;
            let execution_request_value = read_preserves_file(&execution_request)?;
            let request_value = job_worker_cli_request(JobWorkerRequestCliInput {
                admission_value: &admission_value,
                execution_request_value: &execution_request_value,
                sync_ref: sync_ref.as_deref(),
                target_peer: &target_peer,
                stages: &stages,
                authority_refs,
                resource_refs,
                peer_bootstrap_refs,
                node_identity_refs,
                evidence_refs,
            })?;
            let parsed = job_dag::parse_job_worker_request_value(&request_value)?;
            emit_job_analysis(&request_value, out.as_ref())?;
            eprintln!(
                "job worker-request ok job={} request={} target={} stages={}",
                parsed.job_ref,
                parsed.request_ref,
                parsed.target_peer,
                parsed.stage_ids.len()
            );
            Ok(())
        }
        JobCommand::WorkerRunLocal {
            request,
            target_registry,
            storage,
            cache,
            chunks,
            admission_receipt,
            execution_request,
            transport_root,
            from_peer,
            from_actor,
            topic,
            ledger,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let admission_value = read_preserves_file(&admission_receipt)?;
            let execution_request_value = read_preserves_file(&execution_request)?;
            let chunk_root = chunks.unwrap_or_else(|| target_registry.join("job-chunks"));
            let executed = job_worker_run_local(JobWorkerRunLocalInput {
                request_value: &request_value,
                target_registry: &target_registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                admission_value: &admission_value,
                execution_request_value: &execution_request_value,
                transport_root: &transport_root,
                from_peer: &from_peer,
                from_actor: &from_actor,
                topic: &topic,
                ledger_root: ledger.as_deref(),
                out: &out,
            })?;
            eprintln!(
                "job worker-run-local {} job={} receipt={} result={} out={}",
                executed.result.decision,
                executed.result.job_ref,
                executed.receipt_ref,
                executed.result.result_ref,
                out.display()
            );
            if executed.result.decision == "pass" {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job worker-run-local denied: {}",
                    executed.result.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::WorkerScheduleLocal {
            request,
            target_registry,
            storage,
            cache,
            chunks,
            admission_receipt,
            execution_request,
            transport_root,
            queue_key,
            lease_key,
            scheduler_session,
            worker_session,
            lease_token,
            from_peer,
            from_actor,
            topic,
            coordination_authority_refs,
            coordination_resource_refs,
            coordination_policy_refs,
            ledger,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let admission_value = read_preserves_file(&admission_receipt)?;
            let execution_request_value = read_preserves_file(&execution_request)?;
            let chunk_root = chunks.unwrap_or_else(|| target_registry.join("job-chunks"));
            let scheduled = job_worker_schedule_local(JobWorkerScheduleLocalInput {
                request_value: &request_value,
                target_registry: &target_registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                admission_value: &admission_value,
                execution_request_value: &execution_request_value,
                transport_root: &transport_root,
                queue_key: &queue_key,
                lease_key: lease_key.as_deref(),
                scheduler_session: &scheduler_session,
                worker_session: &worker_session,
                lease_token,
                from_peer: &from_peer,
                from_actor: &from_actor,
                topic: &topic,
                coordination_authority_refs,
                coordination_resource_refs,
                coordination_policy_refs,
                ledger_root: ledger.as_deref(),
                out: &out,
            })?;
            eprintln!(
                "job worker-schedule-local {} receipt={} worker={} out={}",
                scheduled.decision,
                scheduled.receipt_ref,
                scheduled.worker.as_ref().map(|worker| worker.receipt_ref.as_str()).unwrap_or("-"),
                out.display()
            );
            if scheduled.decision == "pass" {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job worker-schedule-local denied: {}",
                    job_dag::parse_job_worker_schedule_receipt_value(&scheduled.receipt_value)?.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::RefSubmit {
            job_id,
            operation_id,
            executable,
            inputs,
            output_mode,
            input_schema_refs,
            output_schema_refs,
            effect_manifest_refs,
            handler_profile,
            authority_context_ref,
            policy_refs,
            provenance_refs,
            evidence_refs,
            out,
        } => {
            let executable = parse_job_content_arg(&executable, "executable")?;
            let inputs =
                inputs.iter().map(|input| parse_job_content_arg(input, "input")).collect::<Result<Vec<_>>>()?;
            let value = job_dag::job_ref_submission_value(job_dag::BlobRefJobSubmissionValueInput {
                job_id: &job_id,
                operation_id: &operation_id,
                executable,
                inputs,
                output_mode: &output_mode,
                input_schema_refs: &input_schema_refs,
                output_schema_refs: &output_schema_refs,
                effect_manifest_refs: &effect_manifest_refs,
                handler_profile: &handler_profile,
                authority_context_ref: &authority_context_ref,
                policy_refs: &policy_refs,
                provenance_refs: &provenance_refs,
                evidence_refs: &evidence_refs,
            })?;
            let submission = job_dag::parse_job_ref_submission_value(&value)?;
            emit_job_analysis(&value, out.as_ref())?;
            eprintln!(
                "job ref-submit ok job={} submission={} inputs={}",
                submission.job_id,
                submission.submission_ref,
                submission.inputs.len()
            );
            Ok(())
        }
        JobCommand::RefExecute {
            submission,
            chunks,
            ledger,
            receipt_out,
        } => {
            let submission_value = read_preserves_file(&submission)?;
            let executed = job_dag::execute_blob_ref_job(job_dag::BlobRefJobExecuteInput {
                chunk_root: &chunks,
                submission_value: &submission_value,
                ledger_root: ledger.as_deref(),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "job ref receipt", &executed.receipt_value)?;
            eprintln!(
                "job ref-execute {} job={} receipt={} output={}",
                executed.decision,
                executed.submission.job_id,
                executed.receipt_ref,
                executed.output_manifest_ref.as_deref().unwrap_or("none")
            );
            if executed.decision == "pass" {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job ref-execute denied: {}",
                    executed.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::Status { ledger, job } => {
            for entry in ledger::list_artifacts(&ledger)? {
                let value = match entry.artifact_kind.as_str() {
                    "job-dag-receipt" | "job-ref-receipt" | "job-worker-receipt" | "job-worker-schedule-receipt" => {
                        ledger::read_artifact(&ledger, &entry.artifact_ref)?
                    }
                    _ => continue,
                };
                if let Ok(schedule) = job_dag::parse_job_worker_schedule_receipt_value(&value) {
                    if job.as_ref().is_none_or(|job_ref| schedule.job_ref == *job_ref) {
                        println!(
                            "{} worker-schedule {} {} {}",
                            entry.artifact_ref,
                            schedule.decision,
                            schedule.job_ref,
                            schedule.result_ref.unwrap_or_else(|| "-".to_string())
                        );
                    }
                    continue;
                }
                if let Ok(worker) = job_dag::parse_job_worker_receipt_value(&value) {
                    if job.as_ref().is_none_or(|job_ref| worker.job_ref.as_ref() == Some(job_ref)) {
                        println!(
                            "{} worker-execute {} {} {}",
                            entry.artifact_ref,
                            worker.decision,
                            worker.job_ref.unwrap_or_else(|| "-".to_string()),
                            worker.result_ref
                        );
                    }
                    continue;
                }
                let receipt = job_dag::parse_job_receipt(&value)
                    .or_else(|_| job_dag::parse_blob_ref_job_receipt_value(&value))?;
                if job.as_ref().is_none_or(|job_ref| receipt.job_ref.as_ref() == Some(job_ref)) {
                    println!(
                        "{} {} {} {} {}",
                        entry.artifact_ref,
                        receipt.operation,
                        receipt.decision,
                        receipt.job_ref.unwrap_or_else(|| "-".to_string()),
                        receipt.stage_id.unwrap_or_else(|| "-".to_string())
                    );
                }
            }
            Ok(())
        }
        JobCommand::ReceiptShow { receipt_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            println!("{}", job_dag::receipt_summary(&value)?);
            println!("{}", to_text(&value)?);
            Ok(())
        }
    }
}

fn job_sync_cli_request(
    source_registry: &Path,
    job: &str,
    stages: &[String],
    target_peer: &str,
    extra_evidence_refs: &[String],
) -> Result<preserves::IOValue> {
    let dag = job_dag::read_job_dag_file_or_registry(source_registry, job)?;
    let mut evidence_refs = vec![cli_job_ref("sync-evidence", &dag.job_ref)?];
    evidence_refs.extend(extra_evidence_refs.iter().cloned());
    job_dag::job_sync_request_value(job_dag::SyncRequestValueInput {
        job_ref: &dag.job_ref,
        stage_ids: stages,
        target_peer,
        policy_refs: &[cli_job_ref("sync-policy", &dag.job_ref)?],
        capability_refs: &[cli_job_ref("sync-capability", &dag.job_ref)?],
        evidence_refs: &evidence_refs,
    })
}

struct JobAdmissionCliInput<'a> {
    target_registry: &'a Path,
    job: &'a str,
    sync_ref: Option<&'a str>,
    stages: &'a [String],
    target_peer: &'a str,
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    evidence_refs: Vec<String>,
    resource_refs: Vec<String>,
}

struct JobExecutionCliInput<'a> {
    target_registry: &'a Path,
    job: &'a str,
    admission_value: &'a preserves::IOValue,
    target_peer: &'a str,
    stages: &'a [String],
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    resource_refs: Vec<String>,
}

struct JobExecutionFromAdmissionCliInput<'a> {
    target_registry: &'a Path,
    job: &'a str,
    admission_ref: Option<&'a str>,
    target_peer: &'a str,
    stages: &'a [String],
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    resource_refs: Vec<String>,
}

struct JobWorkerRequestCliInput<'a> {
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
    sync_ref: Option<&'a str>,
    target_peer: &'a str,
    stages: &'a [String],
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    peer_bootstrap_refs: Vec<String>,
    node_identity_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

struct JobWorkerRunLocalInput<'a> {
    request_value: &'a preserves::IOValue,
    target_registry: &'a Path,
    storage_root: &'a Path,
    cache_root: &'a Path,
    chunk_root: &'a Path,
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
    transport_root: &'a Path,
    from_peer: &'a str,
    from_actor: &'a str,
    topic: &'a str,
    ledger_root: Option<&'a Path>,
    out: &'a Path,
}

struct JobWorkerScheduleLocalInput<'a> {
    request_value: &'a preserves::IOValue,
    target_registry: &'a Path,
    storage_root: &'a Path,
    cache_root: &'a Path,
    chunk_root: &'a Path,
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
    transport_root: &'a Path,
    queue_key: &'a str,
    lease_key: Option<&'a str>,
    scheduler_session: &'a str,
    worker_session: &'a str,
    lease_token: Option<u64>,
    from_peer: &'a str,
    from_actor: &'a str,
    topic: &'a str,
    coordination_authority_refs: Vec<String>,
    coordination_resource_refs: Vec<String>,
    coordination_policy_refs: Vec<String>,
    ledger_root: Option<&'a Path>,
    out: &'a Path,
}

struct JobWorkerScheduleLocalResult {
    decision: String,
    receipt_ref: String,
    receipt_value: preserves::IOValue,
    worker: Option<job_dag::JobWorkerExecution>,
}

fn job_worker_cli_request(input: JobWorkerRequestCliInput<'_>) -> Result<preserves::IOValue> {
    let admission = job_dag::parse_job_admission_receipt_value(input.admission_value)?;
    let execution_request = job_dag::parse_job_execution_request_value(input.execution_request_value)?;
    let admission_ref = canonical_hash(input.admission_value)?;
    let execution_request_ref = canonical_hash(input.execution_request_value)?;
    if execution_request.admission_ref != admission_ref {
        return Err(MoltenError::invalid_harness("job worker execution request does not bind admission receipt"));
    }
    if execution_request.job_ref != admission.job_ref {
        return Err(MoltenError::invalid_harness("job worker execution request job ref mismatches admission"));
    }
    let sync_ref = input.sync_ref.map(str::to_string).unwrap_or_else(|| admission.sync_ref.clone());
    let stage_ids = if input.stages.is_empty() {
        execution_request.stage_ids.clone()
    } else {
        input.stages.to_vec()
    };
    let authority_refs = if input.authority_refs.is_empty() {
        admission.authority_receipt_refs.clone()
    } else {
        input.authority_refs
    };
    let resource_refs = if input.resource_refs.is_empty() {
        execution_request.resource_refs.clone()
    } else {
        input.resource_refs
    };
    let mut evidence_refs = CliBoundedItems::new(JOB_WORKER_CLI_REF_LIMIT, "job worker evidence refs");
    for reference in input.evidence_refs {
        evidence_refs.push_unique(reference)?;
    }
    for reference in [sync_ref.clone(), admission_ref.clone(), execution_request_ref.clone()] {
        evidence_refs.push_unique(reference)?;
    }
    for reference in &input.peer_bootstrap_refs {
        evidence_refs.push_unique(reference.clone())?;
    }
    for reference in &input.node_identity_refs {
        evidence_refs.push_unique(reference.clone())?;
    }
    job_dag::job_worker_request_value(job_dag::JobWorkerRequestValueInput {
        job_ref: &admission.job_ref,
        target_peer: input.target_peer,
        stage_ids: &stage_ids,
        sync_ref: &sync_ref,
        admission_ref: &admission_ref,
        execution_request_ref: &execution_request_ref,
        authority_refs: &authority_refs,
        resource_refs: &resource_refs,
        peer_bootstrap_refs: &input.peer_bootstrap_refs,
        node_identity_refs: &input.node_identity_refs,
        evidence_refs: &evidence_refs.into_vec(),
    })
}

fn job_worker_run_local(input: JobWorkerRunLocalInput<'_>) -> Result<job_dag::JobWorkerExecution> {
    let request = job_dag::parse_job_worker_request_value(input.request_value)?;
    let envelope = job_dag::job_worker_envelope(job_dag::JobWorkerEnvelopeInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: &request.target_peer,
        topic: input.topic,
        request_value: input.request_value,
    })?;
    let published = remote_dataspace::publish_local_gossip(input.transport_root, &envelope, input.from_peer)?;
    let delivery = remote_dataspace::deliver_local_gossip(
        input.transport_root,
        input.topic,
        &envelope.envelope_ref,
        &request.target_peer,
    )?;
    let delivery_log = remote_dataspace::delivery_log(std::slice::from_ref(&delivery), true)?;
    let executed = job_dag::execute_worker_delivery(job_dag::JobWorkerExecuteInput {
        target_registry: input.target_registry,
        storage_root: input.storage_root,
        cache_root: input.cache_root,
        chunk_root: input.chunk_root,
        delivery: &delivery,
        delivery_log: Some(&delivery_log),
        admission_receipt_value: input.admission_value,
        execution_request_value: input.execution_request_value,
        ledger_root: input.ledger_root,
    })?;
    fs::create_dir_all(input.out).map_err(MoltenError::from)?;
    write_file(&input.out.join("request.preserves"), &to_text(input.request_value)?)?;
    write_file(&input.out.join("envelope.preserves"), &to_text(&envelope.value)?)?;
    write_file(&input.out.join("publish-receipt.preserves"), &to_text(&published.receipt_value)?)?;
    write_file(&input.out.join("delivery-receipt.preserves"), &to_text(&delivery.receipt_value)?)?;
    write_file(&input.out.join("delivery-log.preserves"), &to_text(&delivery_log.value)?)?;
    write_file(&input.out.join("assignment.preserves"), &to_text(&executed.assignment_value)?)?;
    write_indexed_values(input.out, "status", &executed.status_values)?;
    write_file(&input.out.join("result.preserves"), &to_text(&executed.result.value)?)?;
    write_file(&input.out.join("worker-receipt.preserves"), &to_text(&executed.receipt_value)?)?;
    if let Some(execution) = executed.execution.as_ref() {
        write_file(&input.out.join("execution-receipt.preserves"), &to_text(&execution.receipt_value)?)?;
        if let Some(run) = execution.run.as_ref() {
            write_file(&input.out.join("output.preserves"), &to_text(&run.output_value)?)?;
        }
    }
    Ok(executed)
}

struct ScheduleCoordinationRefs {
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    policy_refs: Vec<String>,
}

struct ScheduleCoordinationRequestInput<'a> {
    service: &'a str,
    operation: &'a str,
    key: &'a str,
    client_session: &'a str,
    operation_label: &'a str,
    request_ref: &'a str,
    payload: Option<preserves::IOValue>,
    refs: &'a ScheduleCoordinationRefs,
}

fn job_schedule_coordination_request(input: ScheduleCoordinationRequestInput<'_>) -> Result<preserves::IOValue> {
    coordination::coordination_request_value(&coordination::CoordinationRequestInput {
        service: input.service.to_string(),
        operation: input.operation.to_string(),
        key: input.key.to_string(),
        client_session: input.client_session.to_string(),
        operation_id_ref: cli_job_ref(input.operation_label, input.request_ref)?,
        payload: input.payload,
        authority_refs: input.refs.authority_refs.clone(),
        resource_refs: input.refs.resource_refs.clone(),
        policy_refs: input.refs.policy_refs.clone(),
    })
}

fn push_schedule_coordination_result(
    result: &coordination::CoordinationApplyResult,
    evidence_values: &mut CliBoundedItems<preserves::IOValue>,
    receipt_refs: &mut CliBoundedItems<String>,
    assertion_refs: &mut CliBoundedItems<String>,
) -> Result<()> {
    receipt_refs.push(result.receipt.receipt_ref.clone())?;
    for assertion in &result.assertions {
        assertion_refs.push(assertion.assertion_ref.clone())?;
    }
    for value in &result.evidence_values {
        evidence_values.push(value.clone())?;
    }
    Ok(())
}

fn job_worker_schedule_local(input: JobWorkerScheduleLocalInput<'_>) -> Result<JobWorkerScheduleLocalResult> {
    let request = job_dag::parse_job_worker_request_value(input.request_value)?;
    let request_ref = request.request_ref.clone();
    let lease_key = input
        .lease_key
        .map(str::to_string)
        .unwrap_or_else(|| format!("lock:job-worker:{}", request.request_ref));
    let coordination_refs = ScheduleCoordinationRefs {
        authority_refs: if input.coordination_authority_refs.is_empty() {
            request.authority_refs.clone()
        } else {
            input.coordination_authority_refs.clone()
        },
        resource_refs: if input.coordination_resource_refs.is_empty() {
            request.resource_refs.clone()
        } else {
            input.coordination_resource_refs.clone()
        },
        policy_refs: if input.coordination_policy_refs.is_empty() {
            vec![cli_job_ref("worker-schedule-policy", &request_ref)?]
        } else {
            input.coordination_policy_refs.clone()
        },
    };
    let manifest_value = coordination::coordination_fixture_manifest_value()?;
    let mut runtime = coordination::new_coordination_runtime(&manifest_value)?;
    let manifest_ref = runtime.manifest.manifest_ref.clone();
    let mut evidence_values =
        CliBoundedItems::new(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "job worker schedule evidence");
    let mut receipt_refs = CliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule receipts");
    let mut assertion_refs = CliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule assertions");
    evidence_values.push(manifest_value.clone())?;

    let enqueue_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
        service: coordination::SERVICE_QUEUE,
        operation: coordination::OP_ENQUEUE,
        key: input.queue_key,
        client_session: input.scheduler_session,
        operation_label: "worker-schedule-enqueue",
        request_ref: &request_ref,
        payload: Some(record("item", vec![string(&request_ref)])),
        refs: &coordination_refs,
    })?;
    let enqueue = coordination::apply_coordination_request(&mut runtime, &enqueue_request)?;
    push_schedule_coordination_result(&enqueue, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
    let enqueue_duplicate = coordination::apply_coordination_request(&mut runtime, &enqueue_request)?;
    push_schedule_coordination_result(
        &enqueue_duplicate,
        &mut evidence_values,
        &mut receipt_refs,
        &mut assertion_refs,
    )?;

    let mut diagnostics = Vec::new();
    let mut dequeue: Option<coordination::CoordinationApplyResult> = None;
    let mut lease: Option<coordination::CoordinationApplyResult> = None;
    let mut release: Option<coordination::CoordinationApplyResult> = None;
    let mut worker: Option<job_dag::JobWorkerExecution> = None;
    if enqueue.receipt.decision != "pass" {
        diagnostics.extend(enqueue.receipt.diagnostics.clone());
    } else if enqueue_duplicate.receipt.receipt_ref != enqueue.receipt.receipt_ref {
        diagnostics.push("coordination duplicate enqueue did not replay prior receipt".to_string());
    } else {
        let dequeue_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
            service: coordination::SERVICE_QUEUE,
            operation: coordination::OP_DEQUEUE,
            key: input.queue_key,
            client_session: input.worker_session,
            operation_label: "worker-schedule-dequeue",
            request_ref: &request_ref,
            payload: None,
            refs: &coordination_refs,
        })?;
        let result = coordination::apply_coordination_request(&mut runtime, &dequeue_request)?;
        push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
        if result.receipt.decision != "pass" {
            diagnostics.extend(result.receipt.diagnostics.clone());
        }
        dequeue = Some(result);
    }
    if diagnostics.is_empty() {
        let lease_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
            service: coordination::SERVICE_LOCK,
            operation: coordination::OP_ACQUIRE,
            key: &lease_key,
            client_session: input.worker_session,
            operation_label: "worker-schedule-lease",
            request_ref: &request_ref,
            payload: None,
            refs: &coordination_refs,
        })?;
        let result = coordination::apply_coordination_request(&mut runtime, &lease_request)?;
        push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
        if result.receipt.decision != "pass" {
            diagnostics.extend(result.receipt.diagnostics.clone());
        }
        lease = Some(result);
    }
    let token = lease.as_ref().and_then(|result| result.token.as_ref());
    if diagnostics.is_empty() {
        let Some(token) = token else {
            diagnostics.push("coordination lease did not emit fencing token".to_string());
            return job_worker_schedule_finalize(JobWorkerScheduleFinalizeInput {
                input,
                request: &request,
                manifest_ref: &manifest_ref,
                runtime: &runtime,
                evidence_values,
                receipt_refs,
                assertion_refs,
                enqueue: Some(&enqueue),
                enqueue_duplicate: Some(&enqueue_duplicate),
                dequeue: dequeue.as_ref(),
                lease: lease.as_ref(),
                release: None,
                worker: None,
                diagnostics,
                lease_key: &lease_key,
            });
        };
        let effective_token = input.lease_token.unwrap_or(token.token);
        if effective_token != token.token {
            let release_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
                service: coordination::SERVICE_LOCK,
                operation: coordination::OP_RELEASE,
                key: &lease_key,
                client_session: input.worker_session,
                operation_label: "worker-schedule-release",
                request_ref: &request_ref,
                payload: Some(record("token", vec![u64_value(effective_token)])),
                refs: &coordination_refs,
            })?;
            let result = coordination::apply_coordination_request(&mut runtime, &release_request)?;
            push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
            diagnostics.extend(result.receipt.diagnostics.clone());
            if diagnostics.is_empty() {
                diagnostics.push(format!("stale fencing token {effective_token}; current token is {}", token.token));
            }
            release = Some(result);
        } else {
            let worker_out = input.out.join("worker");
            let executed = job_worker_run_local(JobWorkerRunLocalInput {
                request_value: input.request_value,
                target_registry: input.target_registry,
                storage_root: input.storage_root,
                cache_root: input.cache_root,
                chunk_root: input.chunk_root,
                admission_value: input.admission_value,
                execution_request_value: input.execution_request_value,
                transport_root: input.transport_root,
                from_peer: input.from_peer,
                from_actor: input.from_actor,
                topic: input.topic,
                ledger_root: input.ledger_root,
                out: &worker_out,
            })?;
            if executed.result.decision != "pass" {
                diagnostics.extend(executed.result.diagnostics.clone());
            }
            worker = Some(executed);
            let release_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
                service: coordination::SERVICE_LOCK,
                operation: coordination::OP_RELEASE,
                key: &lease_key,
                client_session: input.worker_session,
                operation_label: "worker-schedule-release",
                request_ref: &request_ref,
                payload: Some(record("token", vec![u64_value(effective_token)])),
                refs: &coordination_refs,
            })?;
            let result = coordination::apply_coordination_request(&mut runtime, &release_request)?;
            push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
            if result.receipt.decision != "pass" {
                diagnostics.extend(result.receipt.diagnostics.clone());
            }
            release = Some(result);
        }
    }
    job_worker_schedule_finalize(JobWorkerScheduleFinalizeInput {
        input,
        request: &request,
        manifest_ref: &manifest_ref,
        runtime: &runtime,
        evidence_values,
        receipt_refs,
        assertion_refs,
        enqueue: Some(&enqueue),
        enqueue_duplicate: Some(&enqueue_duplicate),
        dequeue: dequeue.as_ref(),
        lease: lease.as_ref(),
        release: release.as_ref(),
        worker: worker.as_ref(),
        diagnostics,
        lease_key: &lease_key,
    })
}

struct JobWorkerScheduleFinalizeInput<'a> {
    input: JobWorkerScheduleLocalInput<'a>,
    request: &'a job_dag::JobWorkerRequest,
    manifest_ref: &'a str,
    runtime: &'a coordination::CoordinationRuntime,
    evidence_values: CliBoundedItems<preserves::IOValue>,
    receipt_refs: CliBoundedItems<String>,
    assertion_refs: CliBoundedItems<String>,
    enqueue: Option<&'a coordination::CoordinationApplyResult>,
    enqueue_duplicate: Option<&'a coordination::CoordinationApplyResult>,
    dequeue: Option<&'a coordination::CoordinationApplyResult>,
    lease: Option<&'a coordination::CoordinationApplyResult>,
    release: Option<&'a coordination::CoordinationApplyResult>,
    worker: Option<&'a job_dag::JobWorkerExecution>,
    diagnostics: Vec<String>,
    lease_key: &'a str,
}

fn pass_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn job_worker_schedule_finalize(input: JobWorkerScheduleFinalizeInput<'_>) -> Result<JobWorkerScheduleLocalResult> {
    let mut evidence_values = input.evidence_values;
    let receipt_refs = input.receipt_refs.into_vec();
    let assertion_refs = input.assertion_refs.into_vec();
    let final_state_value = coordination::coordination_state_snapshot_value(&input.runtime.state)?;
    let final_state_ref = canonical_hash(&final_state_value)?;
    evidence_values.push(final_state_value)?;
    let evidence_values = evidence_values.into_vec();
    let evidence_refs = evidence_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let report_value = coordination::coordination_apply_report_value(coordination::ApplyReportValueInput {
        decision,
        manifest_ref: input.manifest_ref,
        final_state_ref: &final_state_ref,
        receipt_refs: &receipt_refs,
        assertion_refs: &assertion_refs,
        evidence_refs: &evidence_refs,
    })?;
    let report_ref = canonical_hash(&report_value)?;
    let worker_receipt_ref = input.worker.map(|worker| worker.receipt_ref.as_str());
    let result_ref = input.worker.map(|worker| worker.result.result_ref.as_str());
    let token_ref = input.lease.and_then(|lease| lease.token.as_ref()).map(|token| token.token_ref.as_str());
    let mut refs = evidence_refs.clone();
    if let Some(worker) = input.worker {
        refs.push(worker.receipt_ref.clone());
        refs.push(worker.result.result_ref.clone());
    }
    let receipt_value = job_dag::job_worker_schedule_receipt_value(job_dag::JobWorkerScheduleReceiptValueInput {
        operation: "worker-schedule-local",
        decision,
        job_ref: &input.request.job_ref,
        request_ref: &input.request.request_ref,
        queue_key: input.input.queue_key,
        lease_key: input.lease_key,
        worker_session: input.input.worker_session,
        coordination_report_ref: &report_ref,
        enqueue_receipt_ref: input.enqueue.map(|result| result.receipt.receipt_ref.as_str()),
        enqueue_duplicate_receipt_ref: input.enqueue_duplicate.map(|result| result.receipt.receipt_ref.as_str()),
        dequeue_receipt_ref: input.dequeue.map(|result| result.receipt.receipt_ref.as_str()),
        lease_receipt_ref: input.lease.map(|result| result.receipt.receipt_ref.as_str()),
        release_receipt_ref: input.release.map(|result| result.receipt.receipt_ref.as_str()),
        token_ref,
        worker_receipt_ref,
        result_ref,
        diagnostics: &input.diagnostics,
        refs: &refs,
        checks: &[
            (
                "duplicate-operation-replay",
                pass_fail(input.enqueue_duplicate.is_some_and(|duplicate| {
                    input.enqueue.is_some_and(|enqueue| duplicate.receipt.receipt_ref == enqueue.receipt.receipt_ref)
                })),
            ),
            ("lease-checked-before-worker", pass_fail(input.worker.is_some() || !input.diagnostics.is_empty())),
            (
                "worker-result-bound",
                pass_fail(input.worker.is_some_and(|worker| worker.result.decision == "pass")),
            ),
        ],
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    fs::create_dir_all(input.input.out).map_err(MoltenError::from)?;
    write_file(&input.input.out.join("schedule-receipt.preserves"), &to_text(&receipt_value)?)?;
    let coordination_out = input.input.out.join("coordination");
    fs::create_dir_all(&coordination_out).map_err(MoltenError::from)?;
    write_file(&coordination_out.join("manifest.preserves"), &to_text(&evidence_values[0])?)?;
    write_file(&coordination_out.join("report.preserves"), &to_text(&report_value)?)?;
    write_indexed_values(&coordination_out, "evidence", &evidence_values)?;
    if let Some(result) = input.enqueue {
        write_file(&coordination_out.join("enqueue-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(result) = input.enqueue_duplicate {
        write_file(&coordination_out.join("enqueue-duplicate-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(result) = input.dequeue {
        write_file(&coordination_out.join("dequeue-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(result) = input.lease {
        write_file(&coordination_out.join("lease-receipt.preserves"), &to_text(&result.receipt.value)?)?;
        if let Some(token) = &result.token {
            write_file(&coordination_out.join("fencing-token.preserves"), &to_text(&token.value)?)?;
        }
    }
    if let Some(result) = input.release {
        write_file(&coordination_out.join("release-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(ledger_root) = input.input.ledger_root {
        ledger::import_artifact(ledger_root, &report_value)?;
        ledger::import_artifact(ledger_root, &receipt_value)?;
    }
    Ok(JobWorkerScheduleLocalResult {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        worker: input.worker.cloned(),
    })
}

fn job_admission_cli_request(input: JobAdmissionCliInput<'_>) -> Result<preserves::IOValue> {
    let mut policy_refs = input.policy_refs;
    let mut capability_refs = input.capability_refs;
    let mut evidence_refs = input.evidence_refs;
    let mut resource_refs = input.resource_refs;
    let dag = job_dag::read_job_dag_file_or_registry(input.target_registry, input.job)?;
    let sync_ref = input.sync_ref.map(str::to_string).unwrap_or(cli_job_ref("sync-evidence", &dag.job_ref)?);
    if policy_refs.is_empty() {
        policy_refs.push(cli_job_ref("admission-policy", &dag.job_ref)?);
    }
    if capability_refs.is_empty() {
        capability_refs.push(cli_job_ref("admission-capability", &dag.job_ref)?);
    }
    if !evidence_refs.iter().any(|reference| reference == &sync_ref) {
        evidence_refs.push(sync_ref.clone());
    }
    if !evidence_refs.iter().any(|reference| reference != &sync_ref) {
        evidence_refs.push(cli_job_ref("strict-octet-gate", &dag.job_ref)?);
    }
    if resource_refs.is_empty() {
        let selected = if input.stages.is_empty() {
            dag.nodes.len()
        } else {
            input.stages.len()
        };
        for index in 0..selected.max(1) {
            resource_refs.push(cli_job_ref("admission-resource", &format!("{}:{index}", dag.job_ref))?);
        }
    }
    job_dag::job_admission_request_value(job_dag::AdmissionRequestValueInput {
        job_ref: &dag.job_ref,
        sync_ref: &sync_ref,
        stage_ids: input.stages,
        target_peer: input.target_peer,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        evidence_refs: &evidence_refs,
        resource_refs: &resource_refs,
    })
}

fn job_execution_cli_request(input: JobExecutionCliInput<'_>) -> Result<preserves::IOValue> {
    let admission = job_dag::parse_job_admission_receipt_value(input.admission_value)?;
    let selected_stages = if input.stages.is_empty() {
        admission.stage_order.clone()
    } else {
        input.stages.to_vec()
    };
    let mut capability_refs = input.capability_refs;
    if capability_refs.is_empty() {
        capability_refs.extend(admission.authority_receipt_refs.iter().cloned());
    }
    job_execution_cli_request_from_admission_ref(JobExecutionFromAdmissionCliInput {
        target_registry: input.target_registry,
        job: input.job,
        admission_ref: Some(&admission.receipt_ref),
        target_peer: input.target_peer,
        stages: &selected_stages,
        policy_refs: input.policy_refs,
        capability_refs,
        resource_refs: input.resource_refs,
    })
}

fn job_execution_cli_request_from_admission_ref(
    input: JobExecutionFromAdmissionCliInput<'_>,
) -> Result<preserves::IOValue> {
    let dag = job_dag::read_job_dag_file_or_registry(input.target_registry, input.job)?;
    let admission_ref = input
        .admission_ref
        .map(str::to_string)
        .unwrap_or(cli_job_ref("missing-admission-receipt", &dag.job_ref)?);
    let storage_profile = cli_job_ref("target-storage-profile", &dag.job_ref)?;
    let cache_profile = cli_job_ref("target-cache-profile", &dag.job_ref)?;
    let chunk_profile = cli_job_ref("target-chunk-profile", &dag.job_ref)?;
    job_dag::job_execution_request_value(job_dag::ExecutionRequestValueInput {
        job_ref: &dag.job_ref,
        admission_ref: &admission_ref,
        stage_ids: input.stages,
        target_peer: input.target_peer,
        storage_profile_ref: &storage_profile,
        cache_profile_ref: &cache_profile,
        chunk_profile_ref: &chunk_profile,
        policy_refs: &input.policy_refs,
        capability_refs: &input.capability_refs,
        resource_refs: &input.resource_refs,
    })
}

fn parse_job_content_arg(value: &str, label: &str) -> Result<job_dag::JobContentRef> {
    let parts = value.split('@').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(MoltenError::invalid_harness(format!(
            "job {label} must be formatted as <content-ref>@<size>@<format>[@<schema-ref>]"
        )));
    }
    let size = parts[1]
        .parse::<u64>()
        .map_err(|error| MoltenError::invalid_harness(format!("job {label} size is invalid: {error}")))?;
    let schema_ref = if parts.len() == 4 {
        Some(parts[3].to_string())
    } else {
        None
    };
    Ok(job_dag::JobContentRef {
        content_ref: parts[0].to_string(),
        size,
        format: parts[2].to_string(),
        schema_ref,
    })
}

fn cli_job_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("job-cli-ref", vec![string(kind), string(label)]))
}

fn emit_job_analysis(value: &preserves::IOValue, out: Option<&PathBuf>) -> Result<()> {
    let text = to_text(value)?;
    if let Some(path) = out {
        write_file(path, &text)?;
    } else {
        println!("{text}");
    }
    Ok(())
}

fn read_transcript_input(path: &Path) -> Result<transcripts::TranscriptArtifact> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    if let Ok(value) = parse_text(&text)
        && let Ok(transcript) = transcripts::parse_transcript_artifact(&value)
    {
        return Ok(transcript);
    }
    transcripts::parse_markdown(&text, &transcripts::TranscriptParseInput {
        dependency_refs: Vec::new(),
        dependency_closure_hash: None,
        handler_profile_ref: None,
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        seed_ref: None,
        expected_refs: Vec::new(),
    })
}

fn write_optional_preserves(out: Option<&PathBuf>, value: &preserves::IOValue) -> Result<bool> {
    if let Some(path) = out {
        write_file(path, &to_text(value)?)?;
        Ok(true)
    } else {
        println!("{}", to_text(value)?);
        Ok(false)
    }
}

fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn run_delivery_command(command: DeliveryCommand) -> Result<()> {
    match command {
        DeliveryCommand::Scope {
            scope_profile,
            scope_name,
            retention_refs,
            out,
        } => {
            let value = delivery_idempotency::scope_profile_value(&scope_profile, &scope_name, &retention_refs)?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("delivery scope ref={reference} profile={scope_profile} name={scope_name}"),
            );
            Ok(())
        }
        DeliveryCommand::OperationId {
            scope_profile,
            scope_name,
            scope_ref,
            producer,
            consumer,
            sequence,
            intent,
            payload_ref,
            policy_refs,
            out,
        } => {
            let resolved_scope_ref =
                resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
            let operation = delivery_idempotency::derive_operation_id(delivery_idempotency::OperationIdInput {
                scope_ref: resolved_scope_ref,
                producer,
                consumer,
                sequence,
                intent,
                payload_ref,
                policy_refs,
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &operation.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "delivery operation ref={} scope={} sequence={} intent={}",
                    operation.operation_ref, operation.scope_ref, operation.sequence, operation.intent
                ),
            );
            Ok(())
        }
        DeliveryCommand::Check {
            root,
            scope_profile,
            scope_name,
            scope_ref,
            producer,
            consumer,
            sequence,
            intent,
            payload_ref,
            policy_refs,
            evidence_refs,
            semantic_result_ref,
            gap_policy,
            receipt_out,
        } => {
            let resolved_scope_ref =
                resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
            let delivery = delivery_idempotency::check_delivery(delivery_idempotency::DeliveryCheckInput {
                root: &root,
                scope_profile: &scope_profile,
                scope_ref: &resolved_scope_ref,
                producer: &producer,
                consumer: &consumer,
                sequence,
                intent: &intent,
                payload_ref: &payload_ref,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: semantic_result_ref.as_deref(),
                gap_policy: parse_delivery_gap_policy(&gap_policy)?,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &delivery.receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "delivery idempotency decision={} operation={} receipt={} side_effect={} prior={}",
                    delivery.receipt.decision,
                    delivery.operation.operation_ref,
                    delivery.receipt.receipt_ref,
                    delivery.receipt.side_effect,
                    delivery.prior_semantic_result_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        DeliveryCommand::ReceiptShow { receipt_ref, root } => {
            let value = delivery_idempotency::read_idempotency_receipt(&root, &receipt_ref)?;
            println!("{}", delivery_idempotency::delivery_summary(&value)?);
            Ok(())
        }
        DeliveryCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", delivery_idempotency::delivery_summary(&value)?);
            Ok(())
        }
    }
}

fn run_retention_command(command: RetentionCommand) -> Result<()> {
    match command {
        RetentionCommand::Class {
            class_name,
            minimum_age_seconds,
            maximum_age_seconds,
            deletion_authority_ref,
            policy_refs,
            has_secret_redaction_hook,
            has_remote_gc_plan,
            has_compaction,
            out,
        } => {
            let value = retention::retention_class_profile_value(&retention::RetentionClassProfileInput {
                class_name: class_name.clone(),
                minimum_age_seconds,
                maximum_age_seconds,
                deletion_authority_ref,
                policy_refs,
                has_secret_redaction_hook,
                has_remote_gc_plan,
                can_compact: has_compaction,
            })?;
            let profile = retention::parse_retention_class_profile(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("retention class ref={} class={}", profile.profile_ref, profile.class_name),
            );
            Ok(())
        }
        RetentionCommand::Pin {
            root,
            object_ref,
            object_kind,
            retention_class,
            source,
            reason,
            owner_ref,
            expiry_ref,
            policy_refs,
            evidence_refs,
            has_authority,
            pin_out,
            receipt_out,
        } => {
            let operation = retention::pin_object(&root, retention::RetentionPinInput {
                object_ref,
                object_kind,
                retention_class,
                source,
                reason,
                owner_ref,
                expiry_ref,
                policy_refs,
                evidence_refs,
                has_authority,
            })?;
            write_optional_preserves(pin_out.as_ref(), &operation.pin.value)?;
            let is_receipt_written = write_optional_preserves(receipt_out.as_ref(), &operation.receipt.value)?;
            print_or_log_summary(
                is_receipt_written,
                &format!(
                    "retention pin decision={} pin={} receipt={}",
                    operation.receipt.decision, operation.pin.pin_ref, operation.receipt.receipt_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::Unpin {
            root,
            pin_ref,
            requester_ref,
            policy_refs,
            evidence_refs,
            has_authority,
            receipt_out,
        } => {
            let receipt = retention::unpin_object(retention::UnpinObjectInput {
                root: &root,
                pin_ref: &pin_ref,
                requester_ref: &requester_ref,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                has_authority,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention unpin decision={} pin={} receipt={}",
                    receipt.decision, pin_ref, receipt.receipt_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::Admit {
            root,
            kind,
            decision,
            requester_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            bound_refs,
            retained_refs,
            remote_refs,
            is_reference_index_complete,
            is_stale,
            revoked_refs,
            diagnostics,
            out,
        } => {
            let admission =
                retention::store_retention_evidence_admission(&root, &retention::RetentionEvidenceAdmissionInput {
                    kind: &kind,
                    decision: &decision,
                    requester_ref: &requester_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    bound_refs: &bound_refs,
                    retained_refs: &retained_refs,
                    remote_refs: &remote_refs,
                    is_reference_index_complete,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    diagnostics: &diagnostics,
                })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &admission.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention admission ref={} kind={} decision={}",
                    admission.admission_ref, admission.kind, admission.decision
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearance {
            root,
            decision,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            out,
        } => {
            let clearance =
                retention::store_retention_remote_gc_clearance(&root, &retention::RetentionRemoteGcClearanceInput {
                    decision: &decision,
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    evidence_refs: &evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    diagnostics: &diagnostics,
                })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &clearance.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance ref={} peer={} remote={} decision={}",
                    clearance.clearance_ref, clearance.peer_ref, clearance.remote_ref, clearance.decision
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceRequest {
            root,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            evidence_refs,
            out,
        } => {
            let request = retention::store_retention_remote_gc_clearance_request(
                &root,
                &retention::RetentionRemoteGcClearanceRequestInput {
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    evidence_refs: &evidence_refs,
                },
            )?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &request.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance request ref={} peer={} remote={} object={}",
                    request.request_ref, request.peer_ref, request.remote_ref, request.object_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceRespond {
            root,
            request,
            evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let response = retention::store_retention_remote_gc_clearance_response(
                retention::RetentionRemoteGcClearanceResponseInput {
                    root: &root,
                    request_value: &request_value,
                    evidence_refs: &evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    diagnostics: &diagnostics,
                },
            )?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &response.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance response ref={} decision={} request={} clearance={}",
                    response.response_ref, response.decision, response.request_ref, response.clearance_ref
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceImport {
            root,
            request,
            response,
            expected_peer_ref,
            expected_remote_ref,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let response_value = read_preserves_file(&response)?;
            let import = retention::import_retention_remote_gc_clearance_response(
                retention::RetentionRemoteGcClearanceImportInput {
                    root: &root,
                    request_value: &request_value,
                    response_value: &response_value,
                    expected_peer_ref: expected_peer_ref.as_deref(),
                    expected_remote_ref: expected_remote_ref.as_deref(),
                },
            )?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &import.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance import ref={} decision={} clearance={}",
                    import.import_ref,
                    import.decision,
                    import.clearance_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveRequestSend {
            root,
            requester_node_root,
            peer_ticket,
            requester_node_id,
            peer_node_id,
            topic,
            sequence,
            max_attempts,
            join_timeout_ms,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            retention_evidence_refs,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            transport_evidence_refs,
            request_out,
            control_out,
            transport_receipt_out,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&peer_ticket)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let sent = runtime.block_on(retention::send_retention_remote_gc_clearance_live_request(
                retention::RetentionRemoteGcClearanceLiveRequestSendInput {
                    root: &root,
                    requester_node_root: requester_node_root.as_deref(),
                    peer_ticket_value: &ticket_value,
                    requester_node_id: &requester_node_id,
                    peer_node_id: &peer_node_id,
                    topic: &topic,
                    sequence,
                    max_attempts,
                    join_timeout_ms,
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    retention_evidence_refs: &retention_evidence_refs,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    transport_evidence_refs: &transport_evidence_refs,
                },
            ))?;
            write_optional_preserves(request_out.as_ref(), &sent.request.value)?;
            write_optional_preserves(control_out.as_ref(), &sent.control_value)?;
            if let Some(path) = transport_receipt_out.as_ref()
                && let Some(value) = sent.send.transport_receipt_value.as_ref()
            {
                write_file(path, &to_text(value)?)?;
            }
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &sent.send.send_receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live request-send request={} control={} send={} transport={} diagnostics={}",
                    sent.request.request_ref,
                    sent.control_ref,
                    sent.send.send_receipt_ref,
                    sent.send.transport_receipt_ref.as_deref().unwrap_or("none"),
                    node_daemon::parse_node_control_live_send_receipt(&sent.send.send_receipt_value)?.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveResponseSend {
            root,
            peer_node_root,
            requester_ticket,
            request,
            peer_node_id,
            requester_node_id,
            topic,
            sequence,
            max_attempts,
            join_timeout_ms,
            response_evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            transport_evidence_refs,
            response_out,
            control_out,
            transport_receipt_out,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&requester_ticket)?;
            let request_value = read_preserves_file(&request)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let sent = runtime.block_on(retention::send_retention_remote_gc_clearance_live_response(
                retention::RetentionRemoteGcClearanceLiveResponseSendInput {
                    root: &root,
                    peer_node_root: peer_node_root.as_deref(),
                    requester_ticket_value: &ticket_value,
                    request_value: &request_value,
                    peer_node_id: &peer_node_id,
                    requester_node_id: &requester_node_id,
                    topic: &topic,
                    sequence,
                    max_attempts,
                    join_timeout_ms,
                    response_evidence_refs: &response_evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    response_diagnostics: &diagnostics,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    transport_evidence_refs: &transport_evidence_refs,
                },
            ))?;
            write_optional_preserves(response_out.as_ref(), &sent.response.value)?;
            write_optional_preserves(control_out.as_ref(), &sent.control_value)?;
            if let Some(path) = transport_receipt_out.as_ref()
                && let Some(value) = sent.send.transport_receipt_value.as_ref()
            {
                write_file(path, &to_text(value)?)?;
            }
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &sent.send.send_receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live response-send response={} control={} send={} transport={} diagnostics={}",
                    sent.response.response_ref,
                    sent.control_ref,
                    sent.send.send_receipt_ref,
                    sent.send.transport_receipt_ref.as_deref().unwrap_or("none"),
                    node_daemon::parse_node_control_live_send_receipt(&sent.send.send_receipt_value)?.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveImportWorkflow {
            root,
            request,
            response,
            request_control,
            request_send_receipt,
            request_receive_receipt,
            request_ingress_ref,
            response_control,
            response_send_receipt,
            response_receive_receipt,
            response_ingress_ref,
            expected_peer_ref,
            expected_remote_ref,
            import_out,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let response_value = read_preserves_file(&response)?;
            let request_control_value = read_preserves_file(&request_control)?;
            let request_send_receipt_value = read_preserves_file(&request_send_receipt)?;
            let request_receive_receipt_value = read_preserves_file(&request_receive_receipt)?;
            let response_control_value = read_preserves_file(&response_control)?;
            let response_send_receipt_value = read_preserves_file(&response_send_receipt)?;
            let response_receive_receipt_value = read_preserves_file(&response_receive_receipt)?;
            let imported = retention::import_retention_remote_gc_clearance_live_workflow(
                retention::RetentionRemoteGcClearanceLiveImportWorkflowInput {
                    root: &root,
                    request_value: &request_value,
                    response_value: &response_value,
                    request_control_value: &request_control_value,
                    request_send_receipt_value: &request_send_receipt_value,
                    request_receive_receipt_value: &request_receive_receipt_value,
                    request_ingress_ref: &request_ingress_ref,
                    response_control_value: &response_control_value,
                    response_send_receipt_value: &response_send_receipt_value,
                    response_receive_receipt_value: &response_receive_receipt_value,
                    response_ingress_ref: &response_ingress_ref,
                    expected_peer_ref: expected_peer_ref.as_deref(),
                    expected_remote_ref: expected_remote_ref.as_deref(),
                },
            )?;
            write_optional_preserves(import_out.as_ref(), &imported.import.value)?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &imported.workflow.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live import-workflow ref={} decision={} import={} clearance={} request-send={} response-send={} diagnostics={}",
                    imported.workflow.workflow_ref,
                    imported.workflow.decision,
                    imported.import.import_ref,
                    imported.import.clearance_ref.as_deref().unwrap_or("none"),
                    imported.request_send_receipt_ref,
                    imported.response_send_receipt_ref,
                    imported.workflow.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::RemoteClearanceLiveLoopback {
            root,
            requester_node_root,
            peer_node_root,
            requester_node_id,
            peer_node_id,
            topic,
            request_sequence,
            response_sequence,
            requester_ref,
            peer_ref,
            object_ref,
            object_kind,
            retention_class,
            action,
            remote_ref,
            policy_ref,
            authority_ref,
            retention_evidence_refs,
            response_evidence_refs,
            retained_refs,
            is_stale,
            revoked_refs,
            diagnostics,
            request_peer_bootstrap_refs,
            request_authority_refs,
            request_policy_refs,
            request_resource_refs,
            request_transport_evidence_refs,
            response_peer_bootstrap_refs,
            response_authority_refs,
            response_policy_refs,
            response_resource_refs,
            response_transport_evidence_refs,
            request_out,
            response_out,
            import_out,
            receipt_out,
        } => {
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let live = runtime.block_on(retention::run_retention_remote_gc_clearance_live_loopback(
                retention::RetentionRemoteGcClearanceLiveLoopbackInput {
                    root: &root,
                    requester_node_root: &requester_node_root,
                    peer_node_root: &peer_node_root,
                    requester_node_id: &requester_node_id,
                    peer_node_id: &peer_node_id,
                    topic: &topic,
                    request_sequence,
                    response_sequence,
                    requester_ref: &requester_ref,
                    peer_ref: &peer_ref,
                    object_ref: &object_ref,
                    object_kind: &object_kind,
                    retention_class: &retention_class,
                    action: &action,
                    remote_ref: &remote_ref,
                    policy_ref: &policy_ref,
                    authority_ref: &authority_ref,
                    retention_evidence_refs: &retention_evidence_refs,
                    response_evidence_refs: &response_evidence_refs,
                    retained_refs: &retained_refs,
                    is_current: !is_stale,
                    revoked_refs: &revoked_refs,
                    response_diagnostics: &diagnostics,
                    request_peer_bootstrap_refs: &request_peer_bootstrap_refs,
                    request_authority_refs: &request_authority_refs,
                    request_policy_refs: &request_policy_refs,
                    request_resource_refs: &request_resource_refs,
                    request_transport_evidence_refs: &request_transport_evidence_refs,
                    response_peer_bootstrap_refs: &response_peer_bootstrap_refs,
                    response_authority_refs: &response_authority_refs,
                    response_policy_refs: &response_policy_refs,
                    response_resource_refs: &response_resource_refs,
                    response_transport_evidence_refs: &response_transport_evidence_refs,
                },
            ))?;
            if let Some(path) = request_out.as_ref() {
                write_file(path, &to_text(&live.request.value)?)?;
            }
            if let Some(path) = response_out.as_ref() {
                write_file(path, &to_text(&live.response.value)?)?;
            }
            if let Some(path) = import_out.as_ref() {
                write_file(path, &to_text(&live.import.value)?)?;
            }
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &live.workflow.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention remote clearance live workflow ref={} decision={} request={} response={} import={} clearance={} diagnostics={}",
                    live.workflow.workflow_ref,
                    live.workflow.decision,
                    live.request.request_ref,
                    live.response.response_ref,
                    live.import.import_ref,
                    live.import.clearance_ref.as_deref().unwrap_or("none"),
                    live.workflow.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::Explain {
            root,
            object_ref,
            object_kind,
            retention_class,
            action,
            subsystem,
            out,
        } => {
            let explain = retention::explain_retention_candidate(retention::RetentionCandidateExplainInput {
                root: &root,
                object_ref: &object_ref,
                object_kind: object_kind.as_deref(),
                retention_class: retention_class.as_deref(),
                action: action.as_deref(),
                subsystem: subsystem.as_deref(),
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &explain.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention explain ref={} object={} pins={} admissions={} clearances={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
                    explain.explain_ref,
                    explain.object_ref,
                    explain.pin_refs.len(),
                    explain.admission_refs.len(),
                    explain.remote_clearance_refs.len(),
                    explain.gc_plan_refs.len(),
                    explain.gc_apply_refs.len(),
                    explain.gc_execution_refs.len(),
                    explain.gc_audit_refs.len(),
                    explain.retention_receipt_refs.len(),
                    explain.tombstone_refs.len(),
                    explain.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::BundleExport {
            root,
            explain,
            out,
            profile,
        } => {
            let explain_value = read_preserves_file(&explain)?;
            let profile = retention::RetentionCandidateBundleExportProfile::parse(&profile)?;
            let bundle =
                retention::export_retention_candidate_bundle(retention::RetentionCandidateBundleExportInput {
                    root: &root,
                    explain_value: &explain_value,
                    out: &out,
                    profile,
                })?;
            eprintln!(
                "retention bundle ref={} explain={} profile={} artifacts={} diagnostics={} out={}",
                bundle.bundle_ref,
                bundle.explain_ref,
                profile.as_str(),
                bundle.artifact_refs.len(),
                bundle.diagnostics.len(),
                out.display()
            );
            Ok(())
        }
        RetentionCommand::BundleVerify { bundle, receipt_out } => {
            let verify =
                retention::verify_retention_candidate_bundle(retention::RetentionCandidateBundleVerifyInput {
                    bundle_dir: &bundle,
                })?;
            let text = to_text(&verify.value)?;
            if let Some(path) = receipt_out {
                write_file(&path, &text)?;
                eprintln!("retention bundle verify receipt {} written to {}", verify.verify_ref, path.display());
            } else {
                println!("{text}");
            }
            eprintln!(
                "retention bundle verify ref={} decision={} bundle={} files={} diagnostics={}",
                verify.verify_ref,
                verify.decision,
                verify.bundle_ref,
                verify.file_refs.len(),
                verify.diagnostics.len()
            );
            Ok(())
        }
        RetentionCommand::GcPlan {
            root,
            subsystem,
            object_ref,
            object_kind,
            retention_class,
            action,
            retention,
            out,
        } => {
            let evidence = retention.into_retention_evidence();
            let plan = retention::store_retention_gc_plan(retention::RetentionGcPlanInput {
                root: &root,
                subsystem: &subsystem,
                object_ref: &object_ref,
                object_kind: &object_kind,
                retention_class: &retention_class,
                action: &action,
                evidence: &evidence,
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &plan.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention gc plan ref={} decision={} subsystem={} action={} object={} gates={} diagnostics={}",
                    plan.plan_ref,
                    plan.decision,
                    plan.subsystem,
                    plan.action,
                    plan.object_ref,
                    plan.gates.len(),
                    plan.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::GcApplyPlan {
            root,
            plan_ref,
            receipt_out,
        } => {
            let apply = retention::apply_retention_gc_plan(retention::RetentionGcApplyFromPlanInput {
                root: &root,
                plan_ref: &plan_ref,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &apply.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention gc apply ref={} decision={} plan={} recomputed={} receipt={} tombstone={} diagnostics={}",
                    apply.apply_ref,
                    apply.decision,
                    apply.plan_ref,
                    apply.recomputed_plan_ref,
                    apply.retention_receipt_ref.as_deref().unwrap_or("none"),
                    apply.tombstone_ref.as_deref().unwrap_or("none"),
                    apply.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::GcAudit {
            root,
            execution_ref,
            out,
        } => {
            let audit = retention::audit_retention_gc_execution(retention::RetentionGcAuditInput {
                root: &root,
                execution_ref: &execution_ref,
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &audit.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention gc audit ref={} decision={} plan={} apply={} execution={} receipt={} tombstone={} diagnostics={}",
                    audit.audit_ref,
                    audit.decision,
                    audit.plan_ref.as_deref().unwrap_or("none"),
                    audit.apply_ref.as_deref().unwrap_or("none"),
                    audit.execution_ref,
                    audit.retention_receipt_ref.as_deref().unwrap_or("none"),
                    audit.tombstone_ref.as_deref().unwrap_or("none"),
                    audit.diagnostics.len()
                ),
            );
            Ok(())
        }
        RetentionCommand::Check {
            root,
            object_ref,
            object_kind,
            retention_class,
            action,
            requester_ref,
            is_reference_index_complete,
            retained_refs,
            remote_refs,
            policy_refs,
            evidence_refs,
            has_delete_authority,
            has_remote_gc_clearance,
            receipt_out,
        } => {
            let evaluation = retention::evaluate_retention(retention::RetentionEvaluationInput {
                root: &root,
                object_ref: &object_ref,
                object_kind: &object_kind,
                retention_class: &retention_class,
                action: &action,
                requester_ref: &requester_ref,
                is_reference_index_complete,
                retained_refs: &retained_refs,
                remote_refs: &remote_refs,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                has_delete_authority,
                has_remote_gc_clearance,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "retention decision={} action={} object={} receipt={} tombstone={}",
                    evaluation.receipt.decision,
                    evaluation.receipt.action,
                    evaluation.receipt.object_ref,
                    evaluation.receipt.receipt_ref,
                    evaluation.receipt.tombstone_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        RetentionCommand::RunFixture { out } => {
            let artifacts = retention::run_fixture(&out)?;
            println!("retention fixture artifacts={} out={}", artifacts.len(), out.display());
            Ok(())
        }
        RetentionCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", retention::retention_summary(&value)?);
            Ok(())
        }
    }
}

fn resolve_delivery_scope_ref(
    scope_profile: &str,
    scope_name: Option<&str>,
    scope_ref: Option<&str>,
) -> Result<String> {
    match (scope_name, scope_ref) {
        (_, Some(reference)) => Ok(reference.to_string()),
        (Some(name), None) => delivery_idempotency::scope_ref(scope_profile, name),
        (None, None) => Err(MoltenError::invalid_harness("delivery command requires --scope-ref or --scope-name")),
    }
}

fn parse_delivery_gap_policy(value: &str) -> Result<delivery_idempotency::GapPolicy> {
    match value {
        "deny" => Ok(delivery_idempotency::GapPolicy::Deny),
        "retry" => Ok(delivery_idempotency::GapPolicy::Retry),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported delivery gap policy {other}; expected deny or retry"
        ))),
    }
}

fn parse_provenance_build_params(values: &[String]) -> Result<Vec<provenance::BuildParam>> {
    let mut params = CliBoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance build params");
    for value in values {
        let Some((key, param_value)) = value.split_once('=') else {
            return Err(MoltenError::invalid_harness(format!("provenance build param `{value}` must use key=value")));
        };
        params.push(provenance::BuildParam {
            key: key.to_string(),
            value: param_value.to_string(),
        })?;
    }
    Ok(params.into_vec())
}

fn run_provenance_command(command: ProvenanceCommand) -> Result<()> {
    match command {
        ProvenanceCommand::BuildRecord {
            expected_artifact_ref,
            source_refs,
            dependency_closure_ref,
            toolchain_refs,
            build_params,
            builder_ref,
            nix_derivation_refs,
            policy_refs,
            evidence_refs,
            out,
        } => {
            let build_params = parse_provenance_build_params(&build_params)?;
            let value = provenance::provenance_build_record_value(&provenance::ProvenanceBuildRecordInput {
                expected_artifact_ref: &expected_artifact_ref,
                source_refs: &source_refs,
                dependency_closure_ref: &dependency_closure_ref,
                toolchain_refs: &toolchain_refs,
                build_params: &build_params,
                builder_ref: &builder_ref,
                nix_derivation_refs: &nix_derivation_refs,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("provenance build record ref={reference} expected_artifact={expected_artifact_ref}"),
            );
            Ok(())
        }
        ProvenanceCommand::VerifyBuild {
            build_record,
            actual_artifact_ref,
            prior_diagnostics,
            receipt_out,
        } => {
            let build_record_value = read_preserves_file(&build_record)?;
            let verification = provenance::verify_provenance_build(&provenance::ProvenanceBuildVerificationInput {
                build_record_value: &build_record_value,
                actual_artifact_ref: &actual_artifact_ref,
                prior_diagnostics: &prior_diagnostics,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &verification.receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "provenance build verification decision={} expected={} actual={} receipt={} record={}",
                    verification.decision,
                    verification.expected_artifact_ref,
                    verification.actual_artifact_ref,
                    verification.receipt_ref,
                    verification.build_record_ref
                ),
            );
            Ok(())
        }
        ProvenanceCommand::Record {
            artifact_ref,
            trust_state,
            source_refs,
            dependency_closure_ref,
            toolchain_refs,
            builder_ref,
            review_refs,
            test_refs,
            source_gate_refs,
            policy_refs,
            build_record_refs,
            out,
        } => {
            let value = provenance::provenance_record_value(&provenance::ProvenanceRecordInput {
                artifact_ref: &artifact_ref,
                trust_state: &trust_state,
                source_refs: &source_refs,
                dependency_closure_ref: &dependency_closure_ref,
                toolchain_refs: &toolchain_refs,
                builder_ref: &builder_ref,
                review_refs: &review_refs,
                test_refs: &test_refs,
                source_gate_refs: &source_gate_refs,
                policy_refs: &policy_refs,
                build_record_refs: &build_record_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("provenance record ref={reference} artifact={artifact_ref} trust_state={trust_state}"),
            );
            Ok(())
        }
        ProvenanceCommand::Fixture { artifact_ref, out } => {
            let value = provenance::synthetic_reviewed_provenance_record(&artifact_ref)?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("provenance fixture ref={reference} artifact={artifact_ref} trust_state=reviewed"),
            );
            Ok(())
        }
        ProvenanceCommand::Evaluate {
            operation,
            profile,
            artifact_ref,
            provenance_paths,
            build_verification_paths,
            prior_diagnostics,
            receipt_out,
        } => {
            let mut provenance_values = CliBoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance evidence");
            for path in provenance_paths {
                provenance_values.push(read_preserves_file(&path)?)?;
            }
            let provenance_values = provenance_values.into_vec();
            let mut build_verification_values =
                CliBoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance build verification evidence");
            for path in build_verification_paths {
                build_verification_values.push(read_preserves_file(&path)?)?;
            }
            let build_verification_values = build_verification_values.into_vec();
            let evaluation = provenance::evaluate_provenance(&provenance::ProvenanceEvaluationInput {
                operation: &operation,
                profile: &profile,
                artifact_ref: &artifact_ref,
                provenance_values: &provenance_values,
                build_verification_values: &build_verification_values,
                prior_diagnostics: &prior_diagnostics,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt_value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "provenance decision={} operation={} artifact={} receipt={} matched={}",
                    evaluation.decision,
                    operation,
                    artifact_ref,
                    evaluation.receipt_ref,
                    evaluation.matched_record_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        ProvenanceCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", provenance::provenance_summary(&value)?);
            Ok(())
        }
    }
}

fn run_protocol_command(command: ProtocolCommand) -> Result<()> {
    match command {
        ProtocolCommand::Install { manifest, out } => {
            let manifest_value = read_preserves_file(&manifest)?;
            let install = protocol_session::install_protocol_manifest_value(&manifest_value)?;
            write_protocol_install(&out, &manifest_value, &install)?;
            println!(
                "protocol install decision={} receipt={} protocol={} endpoints={} out={}",
                install.decision,
                install.receipt_ref,
                install.manifest.protocol_id,
                install.endpoints.len(),
                out.display()
            );
            Ok(())
        }
        ProtocolCommand::RunRequestResponse { out } => {
            let lifecycle = protocol_session::request_response_lifecycle()?;
            write_protocol_lifecycle(&out, &lifecycle)?;
            println!(
                "protocol request-response receipt={} operations={} out={}",
                lifecycle.install.receipt_ref,
                lifecycle.operations.len(),
                out.display()
            );
            Ok(())
        }
        ProtocolCommand::GateLifecycle { dir, receipt_out } => {
            let gate = protocol_session::gate_protocol_session_lifecycle(read_protocol_lifecycle_gate_input(&dir)?)?;
            emit_named_receipt(receipt_out.as_ref(), "protocol session gate receipt", &gate.value)?;
            println!(
                "protocol session gate {} install={} protocol={} sessions={} operations={} diagnostics={}",
                gate.decision,
                gate.install_ref,
                gate.protocol_ref,
                gate.session_ids.len(),
                gate.operation_count,
                gate.diagnostics.len()
            );
            Ok(())
        }
        ProtocolCommand::Show { receipt } => {
            let value = read_preserves_file(&receipt)?;
            println!("{}", protocol_session::protocol_summary(&value)?);
            Ok(())
        }
    }
}

fn run_raft_command(command: RaftCommand) -> Result<()> {
    match command {
        RaftCommand::RunFixture { out } => {
            let runtime = raft_control_plane::run_control_registry_fixture()?;
            let read = raft_control_plane::read_control_registry(&raft_control_plane::ControlRegistryReadInput {
                state: runtime.state.value.clone(),
                group_ref: runtime.manifest.manifest_ref.clone(),
                committed_term: runtime.term,
                committed_index: runtime.committed_index,
                read_index: runtime.committed_index,
                namespace: "protocol".to_string(),
                name: "proto:request-response".to_string(),
                authority_refs: vec![cli_synthetic_ref("raft-read-authority")?],
                resource_refs: runtime.manifest.resource_refs.clone(),
            })?;
            let log_refs = runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect::<Vec<_>>();
            let snapshot = raft_control_plane::snapshot_control_registry(&raft_control_plane::RaftSnapshotInput {
                group_ref: runtime.manifest.manifest_ref.clone(),
                term: runtime.term,
                index: runtime.committed_index,
                state: runtime.state.value.clone(),
                log_refs,
            })?;
            let recovery = raft_control_plane::recover_control_registry(&raft_control_plane::RaftRecoveryInput {
                group_ref: runtime.manifest.manifest_ref.clone(),
                snapshot: snapshot.value.clone(),
                log_entries: Vec::new(),
            })?;
            write_raft_fixture(&out, &runtime, &read.value, &snapshot.value, &recovery.value)?;
            println!(
                "raft fixture committed={} entries={} state={} out={}",
                runtime.committed_index,
                runtime.state.entries.len(),
                runtime.state.state_ref,
                out.display()
            );
            Ok(())
        }
        RaftCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", raft_artifact_summary(&value)?);
            Ok(())
        }
    }
}

fn write_raft_fixture(
    out: &Path,
    runtime: &raft_control_plane::ControlRegistryRuntime,
    read_receipt: &preserves::IOValue,
    snapshot: &preserves::IOValue,
    recovery: &preserves::IOValue,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("manifest.preserves"), &to_text(&runtime.manifest.value)?)?;
    write_file(&out.join("state.preserves"), &to_text(&runtime.state.value)?)?;
    write_file(&out.join("read-receipt.preserves"), &to_text(read_receipt)?)?;
    write_file(&out.join("snapshot.preserves"), &to_text(snapshot)?)?;
    write_file(&out.join("recovery-receipt.preserves"), &to_text(recovery)?)?;
    write_file(&out.join("summary.txt"), &raft_control_plane::control_registry_summary(runtime))?;
    write_indexed_values(
        out,
        "log-entry",
        &runtime.log_entries.iter().map(|entry| entry.value.clone()).collect::<Vec<_>>(),
    )?;
    write_indexed_values(
        out,
        "commit-receipt",
        &runtime.commit_receipts.iter().map(|receipt| receipt.value.clone()).collect::<Vec<_>>(),
    )?;
    write_indexed_values(
        out,
        "registry-receipt",
        &runtime.registry_receipts.iter().map(|receipt| receipt.value.clone()).collect::<Vec<_>>(),
    )?;
    write_indexed_values(
        out,
        "predicate-receipt",
        &runtime.predicate_receipts.iter().map(|receipt| receipt.value.clone()).collect::<Vec<_>>(),
    )
}

fn raft_artifact_summary(value: &preserves::IOValue) -> Result<String> {
    match ledger::artifact_kind(value) {
        "control-registry-state" => {
            let state = raft_control_plane::parse_control_registry_state(value)?;
            Ok(format!(
                "control registry state ref={} entries={} sessions={}",
                state.state_ref,
                state.entries.len(),
                state.client_sessions.len()
            ))
        }
        "raft-group-manifest" => {
            let manifest = raft_control_plane::parse_raft_group_manifest(value)?;
            Ok(format!(
                "raft group manifest ref={} group={} members={}",
                manifest.manifest_ref,
                manifest.group_id,
                manifest.members.len()
            ))
        }
        kind => Ok(format!("raft artifact kind={} ref={}", kind, canonical_hash(value)?)),
    }
}

struct CliBoundedItems<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> CliBoundedItems<T> {
    fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    fn push(&mut self, value: T) -> Result<()> {
        if self.values.len() >= self.maximum {
            return Err(MoltenError::invalid_harness(format!("{} count exceeds {}", self.label, self.maximum)));
        }
        self.values.push(value);
        Ok(())
    }

    fn into_vec(self) -> Vec<T> {
        self.values
    }
}

impl<T: PartialEq> CliBoundedItems<T> {
    fn push_unique(&mut self, value: T) -> Result<()> {
        if !self.values.contains(&value) {
            self.push(value)?;
        }
        Ok(())
    }
}

fn run_coordination_command(command: CoordinationCommand) -> Result<()> {
    match command {
        CoordinationCommand::Manifest {
            service_id,
            services,
            control_group_ref,
            queue_capacity,
            semaphore_capacity,
            rate_limit,
            barrier_parties,
            policy_refs,
            resource_refs,
            out,
        } => {
            let control_group_ref = match control_group_ref {
                Some(reference) => reference,
                None => canonical_hash(&raft_control_plane::control_registry_fixture_manifest_value()?)?,
            };
            let services = if services.is_empty() {
                coordination::coordination_supported_services()
            } else {
                services
            };
            let value =
                coordination::coordination_service_manifest_value(&coordination::CoordinationServiceManifestInput {
                    service_id,
                    services,
                    control_group_ref,
                    queue_capacity,
                    semaphore_capacity,
                    rate_limit,
                    barrier_parties,
                    policy_refs,
                    resource_refs,
                })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(is_written_to_file, &format!("coordination manifest ref={reference}"));
            Ok(())
        }
        CoordinationCommand::Request {
            service,
            operation,
            key,
            client_session,
            operation_id_ref,
            payload,
            authority_refs,
            resource_refs,
            policy_refs,
            out,
        } => {
            let payload = payload.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let value = coordination::coordination_request_value(&coordination::CoordinationRequestInput {
                service,
                operation,
                key,
                client_session,
                operation_id_ref,
                payload,
                authority_refs,
                resource_refs,
                policy_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(is_written_to_file, &format!("coordination request ref={reference}"));
            Ok(())
        }
        CoordinationCommand::Apply {
            manifest,
            requests,
            out,
        } => {
            if requests.is_empty() {
                return Err(MoltenError::invalid_harness("coordination apply requires at least one --request file"));
            }
            let manifest_value = read_preserves_file(&manifest)?;
            let mut runtime = coordination::new_coordination_runtime(&manifest_value)?;
            let manifest_ref = runtime.manifest.manifest_ref.clone();
            let mut decision = "pass";
            let mut evidence_values =
                CliBoundedItems::new(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "coordination apply evidence");
            evidence_values.push(manifest_value)?;
            let mut receipt_refs =
                CliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "coordination apply receipts");
            let mut assertion_refs =
                CliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "coordination apply assertions");
            for request in requests {
                let request_value = read_preserves_file(&request)?;
                let result = coordination::apply_coordination_request(&mut runtime, &request_value)?;
                if result.receipt.decision != "pass" {
                    decision = "deny";
                }
                receipt_refs.push(result.receipt.receipt_ref.clone())?;
                for assertion in &result.assertions {
                    assertion_refs.push(assertion.assertion_ref.clone())?;
                }
                for value in &result.evidence_values {
                    evidence_values.push(value.clone())?;
                }
            }
            let final_state_value = coordination::coordination_state_snapshot_value(&runtime.state)?;
            let final_state_ref = canonical_hash(&final_state_value)?;
            evidence_values.push(final_state_value)?;
            let evidence_values = evidence_values.into_vec();
            let receipt_refs = receipt_refs.into_vec();
            let assertion_refs = assertion_refs.into_vec();
            let evidence_refs = evidence_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
            let report_value = coordination::coordination_apply_report_value(coordination::ApplyReportValueInput {
                decision,
                manifest_ref: &manifest_ref,
                final_state_ref: &final_state_ref,
                receipt_refs: &receipt_refs,
                assertion_refs: &assertion_refs,
                evidence_refs: &evidence_refs,
            })?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&report_value)?)?;
            write_indexed_values(&out, "evidence", &evidence_values)?;
            println!(
                "coordination apply decision={} manifest={} state={} receipts={} assertions={} evidence={} out={}",
                decision,
                manifest_ref,
                final_state_ref,
                receipt_refs.len(),
                assertion_refs.len(),
                evidence_refs.len(),
                out.display()
            );
            Ok(())
        }
        CoordinationCommand::RunFixture { out } => {
            let run = coordination::run_coordination_fixture()?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&run.report_value)?)?;
            write_indexed_values(&out, "evidence", &run.evidence_values)?;
            println!(
                "coordination fixture decision={} manifest={} state={} receipts={} assertions={} out={}",
                run.decision,
                run.manifest_ref,
                run.final_state_ref,
                run.receipt_refs.len(),
                run.assertion_refs.len(),
                out.display()
            );
            Ok(())
        }
        CoordinationCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", coordination::coordination_summary(&value)?);
            Ok(())
        }
    }
}

fn run_secrets_command(command: SecretsCommand) -> Result<()> {
    match command {
        SecretsCommand::RunFixture { out } => {
            let run = secrets::run_secrets_fixture()?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&run.value)?)?;
            write_file(&out.join("secret.preserves"), &to_text(&run.secret.value)?)?;
            write_file(&out.join("encrypted-ref.preserves"), &to_text(&run.encrypted.value)?)?;
            write_file(&out.join("redaction-marker.preserves"), &to_text(&run.marker.value)?)?;
            write_file(&out.join("redaction-transform.preserves"), &to_text(&run.transform.value)?)?;
            write_file(&out.join("reveal-denied.preserves"), &to_text(&run.reveal_denied.value)?)?;
            write_file(&out.join("reveal-pass.preserves"), &to_text(&run.reveal_pass.value)?)?;
            write_file(&out.join("decrypt-denied.preserves"), &to_text(&run.decrypt_denied.value)?)?;
            write_file(&out.join("decrypt-pass.preserves"), &to_text(&run.decrypt_pass.value)?)?;
            write_file(&out.join("commitment-replay.preserves"), &to_text(&run.replay.value)?)?;
            write_file(&out.join("cleanup.preserves"), &to_text(&run.cleanup.value)?)?;
            write_file(&out.join("private-bundle-profile.preserves"), &to_text(&run.private_bundle.value)?)?;
            write_indexed_values(&out, "evidence", &run.evidence_values)?;
            write_file(&out.join("summary.txt"), &secrets::fixture_report_summary(&run.value)?)?;
            println!("secrets fixture ok report={} out={}", run.report_ref, out.display());
            Ok(())
        }
        SecretsCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            match secrets::fixture_report_summary(&value) {
                Ok(summary) => println!("{summary}"),
                Err(_) => println!("{}", secrets::secrets_summary(&value)?),
            }
            Ok(())
        }
    }
}

fn run_plugin_command(command: PluginCommand) -> Result<()> {
    match command {
        PluginCommand::Install {
            manifest,
            registry,
            out,
        } => {
            let manifest_value = read_preserves_file(&manifest)?;
            let receipt = plugin_host::install_plugin(&registry, &manifest_value)?;
            write_file(&out, &to_text(&receipt.value)?)?;
            println!(
                "plugin install decision={} receipt={} manifest={} out={}",
                receipt.decision,
                receipt.receipt_ref,
                receipt.manifest_ref,
                out.display()
            );
            Ok(())
        }
        PluginCommand::RunFixture { state_root, out } => {
            let run = plugin_host::minimal_plugin_fixture(&state_root)?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&run.report_value)?)?;
            write_indexed_values(&out, "evidence", &run.evidence_values)?;
            println!(
                "plugin fixture decision={} manifest={} install={} health={} removal={} out={}",
                run.decision,
                run.manifest_ref,
                run.install_receipt_ref,
                run.health_receipt_ref,
                run.removal_receipt_ref,
                out.display()
            );
            Ok(())
        }
        PluginCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", plugin_host::plugin_summary(&value)?);
            Ok(())
        }
    }
}

fn run_service_command(command: ServiceCommand) -> Result<()> {
    match command {
        ServiceCommand::Run { suite, out } => {
            let suite_value = read_preserves_file(&suite)?;
            let run = service_runtime::run_service_runtime_suite_value(&suite_value)?;
            write_service_runtime_run(&out, &suite_value, &run)?;
            println!(
                "service runtime run report={} suite={} lifecycle={} readiness={} out={}",
                run.report_ref,
                run.suite_ref,
                run.lifecycle_receipts.len(),
                run.readiness_assertions.len(),
                out.display()
            );
            Ok(())
        }
        ServiceCommand::RunTwoService { out } => {
            let suite_value = service_runtime::two_service_suite_value()?;
            let run = service_runtime::run_service_runtime_suite_value(&suite_value)?;
            write_service_runtime_run(&out, &suite_value, &run)?;
            println!(
                "service runtime two-service report={} suite={} lifecycle={} readiness={} out={}",
                run.report_ref,
                run.suite_ref,
                run.lifecycle_receipts.len(),
                run.readiness_assertions.len(),
                out.display()
            );
            Ok(())
        }
        ServiceCommand::Supervise { suite, out } => {
            let suite_value = read_preserves_file(&suite)?;
            let run = service_supervision::run_service_supervision_suite_value(&suite_value)?;
            write_service_supervision_run(&out, &suite_value, &run)?;
            println!(
                "service supervision run report={} suite={} monitors={} cleanup={} out={}",
                run.report_ref,
                run.suite_ref,
                run.monitor_notifications.len(),
                run.cleanup_receipts.len(),
                out.display()
            );
            Ok(())
        }
        ServiceCommand::RunSupervisionFixture { out } => {
            let suite_value = service_supervision::supervision_fixture_suite_value()?;
            let run = service_supervision::run_service_supervision_suite_value(&suite_value)?;
            write_service_supervision_run(&out, &suite_value, &run)?;
            println!(
                "service supervision fixture report={} suite={} monitors={} cleanup={} out={}",
                run.report_ref,
                run.suite_ref,
                run.monitor_notifications.len(),
                run.cleanup_receipts.len(),
                out.display()
            );
            Ok(())
        }
        ServiceCommand::Show { report } => {
            let value = read_preserves_file(&report)?;
            println!("{}", service_runtime::service_runtime_summary(&value)?);
            Ok(())
        }
        ServiceCommand::ShowSupervision { report } => {
            let value = read_preserves_file(&report)?;
            println!("{}", service_supervision::service_supervision_summary(&value)?);
            Ok(())
        }
        ServiceCommand::GateSupervision { report, receipt_out } => {
            let value = read_preserves_file(&report)?;
            let gate = service_supervision::gate_service_supervision_report(&value)?;
            emit_named_receipt(receipt_out.as_ref(), "service supervision gate receipt", &gate.value)?;
            println!(
                "service supervision gate {} report={} suite={} restart={} monitors={} cleanup={} diagnostics={}",
                gate.decision,
                gate.report_ref,
                gate.suite_ref,
                gate.restart_decision.as_deref().unwrap_or("none"),
                gate.monitor_count,
                gate.cleanup_count,
                gate.diagnostics.len()
            );
            Ok(())
        }
        ServiceCommand::Replay { report } => {
            let value = read_preserves_file(&report)?;
            let replay = service_runtime::replay_service_runtime_report(&value)?;
            println!(
                "service runtime replay {} expected={} actual={}",
                replay.decision, replay.expected_report_ref, replay.actual_report_ref
            );
            Ok(())
        }
        ServiceCommand::ReplaySupervision { report } => {
            let value = read_preserves_file(&report)?;
            let replay = service_supervision::replay_service_supervision_report(&value)?;
            println!(
                "service supervision replay {} expected={} actual={}",
                replay.decision, replay.expected_report_ref, replay.actual_report_ref
            );
            Ok(())
        }
    }
}

fn write_protocol_install(
    out: &Path,
    manifest_value: &preserves::IOValue,
    install: &protocol_session::ProtocolInstallReceipt,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("manifest.preserves"), &to_text(manifest_value)?)?;
    write_file(&out.join("install-receipt.preserves"), &to_text(&install.value)?)?;
    write_file(&out.join("summary.txt"), &protocol_session::protocol_summary(&install.value)?)?;
    let endpoints_dir = out.join("endpoints");
    fs::create_dir_all(&endpoints_dir).map_err(MoltenError::from)?;
    write_indexed_values(
        &endpoints_dir,
        "endpoint",
        &install.endpoints.iter().map(|endpoint| endpoint.value.clone()).collect::<Vec<_>>(),
    )
}

fn write_protocol_lifecycle(out: &Path, lifecycle: &protocol_session::RequestResponseLifecycle) -> Result<()> {
    write_protocol_install(out, &lifecycle.manifest_value, &lifecycle.install)?;
    write_indexed_values(
        out,
        "initial-state",
        &lifecycle.initial_states.iter().map(|state| state.value.clone()).collect::<Vec<_>>(),
    )?;
    let mut messages = Vec::with_capacity(lifecycle.operations.len());
    let mut receipts = Vec::with_capacity(lifecycle.operations.len());
    let mut next_states = Vec::with_capacity(lifecycle.operations.len());
    for operation in &lifecycle.operations {
        if let Some(message) = &operation.message {
            messages.push(message.value.clone());
        }
        receipts.push(operation.receipt.value.clone());
        if let Some(state) = &operation.next_state {
            next_states.push(state.value.clone());
        }
    }
    write_indexed_values(out, "message", &messages)?;
    write_indexed_values(out, "operation", &receipts)?;
    write_indexed_values(out, "next-state", &next_states)
}

fn read_protocol_lifecycle_gate_input(dir: &Path) -> Result<protocol_session::ProtocolSessionGateInput> {
    Ok(protocol_session::ProtocolSessionGateInput {
        install_receipt: read_preserves_file(&dir.join("install-receipt.preserves"))?,
        initial_states: read_indexed_values(dir, "initial-state")?,
        operation_receipts: read_indexed_values(dir, "operation")?,
        messages: read_indexed_values(dir, "message")?,
        next_states: read_indexed_values(dir, "next-state")?,
    })
}

fn read_indexed_values(dir: &Path, prefix: &str) -> Result<Vec<preserves::IOValue>> {
    let mut values = Vec::with_capacity(PROTOCOL_LIFECYCLE_INDEX_LIMIT.min(16));
    for index in 0..PROTOCOL_LIFECYCLE_INDEX_LIMIT {
        let path = dir.join(format!("{prefix}-{index}.preserves"));
        if !path.exists() {
            return Ok(values);
        }
        values.push(read_preserves_file(&path)?);
    }
    let overflow_path = dir.join(format!("{prefix}-{PROTOCOL_LIFECYCLE_INDEX_LIMIT}.preserves"));
    if overflow_path.exists() {
        return Err(MoltenError::invalid_harness(format!("protocol lifecycle {prefix} evidence exceeds index limit")));
    }
    Ok(values)
}

fn write_service_runtime_run(
    out: &Path,
    suite_value: &preserves::IOValue,
    run: &service_runtime::ServiceRuntimeRun,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("suite.preserves"), &to_text(suite_value)?)?;
    write_file(&out.join("report.preserves"), &to_text(&run.value)?)?;
    write_file(&out.join("summary.txt"), &service_runtime::service_runtime_summary(&run.value)?)?;
    write_indexed_values(out, "lifecycle", &run.lifecycle_receipts)?;
    write_indexed_values(out, "status", &run.statuses)?;
    write_indexed_values(out, "readiness", &run.readiness_assertions)?;
    write_indexed_values(out, "replay-identity", &run.replay_identities)?;
    write_indexed_values(out, "turn-context", &run.turn_contexts)
}

fn write_service_supervision_run(
    out: &Path,
    suite_value: &preserves::IOValue,
    run: &service_supervision::ServiceSupervisionRun,
) -> Result<()> {
    fs::create_dir_all(out).map_err(MoltenError::from)?;
    write_file(&out.join("suite.preserves"), &to_text(suite_value)?)?;
    write_file(&out.join("report.preserves"), &to_text(&run.value)?)?;
    write_file(&out.join("summary.txt"), &service_supervision::service_supervision_summary(&run.value)?)?;
    write_indexed_values(out, "failure", &run.failure_markers)?;
    write_indexed_values(out, "status", &run.statuses)?;
    write_indexed_values(out, "lifecycle", &run.lifecycle_receipts)?;
    write_indexed_values(out, "monitor-notification", &run.monitor_notifications)?;
    write_indexed_values(out, "restart-decision", &run.restart_decisions)?;
    write_indexed_values(out, "scheduled-demand", &run.scheduled_demands)?;
    write_indexed_values(out, "cleanup", &run.cleanup_receipts)?;
    write_indexed_values(out, "retraction", &run.retractions)?;
    write_indexed_values(out, "retention", &run.retention_inputs)
}

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
}

fn run_remote_command(command: RemoteCommand) -> Result<()> {
    match command {
        RemoteCommand::Envelope { command } => run_remote_envelope_command(command),
        RemoteCommand::PublishLocal {
            transport_root,
            envelope,
            node,
            receipt_out,
        } => {
            let envelope_value = read_preserves_file(&envelope)?;
            let envelope = remote_dataspace::parse_envelope(&envelope_value)?;
            let published = remote_dataspace::publish_local_gossip(&transport_root, &envelope, &node)?;
            emit_named_receipt(receipt_out.as_ref(), "remote dataspace publish receipt", &published.receipt_value)?;
            println!("remote publish-local ok envelope={} root={}", published.envelope_ref, transport_root.display());
            Ok(())
        }
        RemoteCommand::DeliverLocal {
            transport_root,
            topic,
            envelope_ref,
            receiver_peer,
            out,
            receipt_out,
        } => {
            let delivered =
                remote_dataspace::deliver_local_gossip(&transport_root, &topic, &envelope_ref, &receiver_peer)?;
            if let Some(out) = out {
                write_file(&out, &to_text(&delivered.envelope.value)?)?;
                println!("remote delivered envelope {} written to {}", delivered.envelope.envelope_ref, out.display());
            }
            emit_named_receipt(receipt_out.as_ref(), "remote dataspace deliver receipt", &delivered.receipt_value)?;
            println!(
                "remote deliver-local ok envelope={} topic={} receiver={}",
                delivered.envelope.envelope_ref, topic, receiver_peer
            );
            Ok(())
        }
        RemoteCommand::RunTwoPeer { transport_root, out } => {
            let harness =
                remote_dataspace::two_peer_service_ready_harness(&transport_root, remote_evidence_fixture()?)?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("delivery-log.preserves"), &to_text(&harness.delivery_log.value)?)?;
            write_file(&out.join("admission-receipt.preserves"), &to_text(&harness.admission_receipt_value)?)?;
            write_file(&out.join("gate-receipt.preserves"), &to_text(&harness.gate_receipt_value)?)?;
            let turn_context_ref = remote_gate_turn_context_ref(&harness.gate_receipt_value)?;
            write_file(&out.join("turn-context-ref.preserves"), &to_text(&string(&turn_context_ref))?)?;
            let summary = record("remote-dataspace-summary-v1", vec![
                record("delivery-log", vec![string(&harness.delivery_log.log_ref)]),
                record("admission-receipt", vec![string(&canonical_hash(&harness.admission_receipt_value)?)]),
                record("gate-receipt", vec![string(&canonical_hash(&harness.gate_receipt_value)?)]),
                record("turn-context-ref", vec![string(&turn_context_ref)]),
            ]);
            write_file(&out.join("summary.preserves"), &to_text(&summary)?)?;
            println!(
                "remote run-two-peer ok delivery_log={} gate_receipt={} out={}",
                harness.delivery_log.log_ref,
                canonical_hash(&harness.gate_receipt_value)?,
                out.display()
            );
            Ok(())
        }
        RemoteCommand::Gate {
            delivery_log,
            admission_receipts,
            turn_context_refs,
            receipt_out,
        } => {
            let log_value = read_preserves_file(&delivery_log)?;
            let log = remote_dataspace::parse_delivery_log(&log_value)?;
            let receipts =
                admission_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let receipt = remote_dataspace::remote_dataspace_gate_receipt_value(&log, &receipts, &turn_context_refs)?;
            emit_named_receipt(receipt_out.as_ref(), "remote dataspace gate receipt", &receipt)
        }
    }
}

fn run_remote_envelope_command(command: RemoteEnvelopeCommand) -> Result<()> {
    match command {
        RemoteEnvelopeCommand::Build {
            from_peer,
            from_actor,
            to_peer,
            topic,
            operation,
            payload,
            content_refs,
            capability_refs,
            evidence_refs,
            out,
        } => {
            let payload = read_preserves_file(&payload)?;
            let operation = parse_remote_operation(&operation)?;
            let envelope = remote_dataspace::build_envelope(remote_dataspace::RemoteDataspaceEnvelopeInput {
                from_peer,
                from_actor,
                to_peer,
                topic,
                operation,
                payload,
                content_refs,
                capability_refs,
                evidence_refs,
            })?;
            write_file(&out, &to_text(&envelope.value)?)?;
            println!("remote envelope {} written to {}", envelope.envelope_ref, out.display());
            Ok(())
        }
    }
}

fn parse_remote_operation(operation: &str) -> Result<remote_dataspace::RemoteDataspaceOperation> {
    match operation {
        "message" => Ok(remote_dataspace::RemoteDataspaceOperation::Message),
        "assert" => Ok(remote_dataspace::RemoteDataspaceOperation::Assert),
        "retract" => Ok(remote_dataspace::RemoteDataspaceOperation::Retract),
        "observe" => Ok(remote_dataspace::RemoteDataspaceOperation::Observe),
        _ => Err(MoltenError::invalid_harness(format!(
            "unsupported remote dataspace operation {operation}; expected message/assert/retract/observe"
        ))),
    }
}

fn remote_evidence_fixture() -> Result<remote_dataspace::RemoteDeliveryEvidence> {
    Ok(remote_dataspace::RemoteDeliveryEvidence {
        peer_bootstrap_refs: vec![cli_synthetic_ref("remote-bootstrap")?],
        capability_refs: vec![cli_synthetic_ref("remote-capability")?],
        policy_refs: vec![cli_synthetic_ref("remote-policy")?],
        resource_refs: vec![cli_synthetic_ref("remote-resource")?],
        authority_refs: vec![cli_synthetic_ref("remote-authority")?],
    })
}

fn cli_synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("remote-cli-ref", vec![string(label)]))
}

fn remote_gate_turn_context_ref(gate_receipt: &preserves::IOValue) -> Result<String> {
    let fields = gate_receipt
        .collect_simple_record("remote-dataspace-gate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected remote dataspace gate receipt"))?;
    let context = molten::preserves_rail::value_to_iovalue(&fields[4]);
    let refs = context
        .collect_simple_record("turn-journal-context-refs", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected remote turn context refs"))?;
    let sequence = refs[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected turn context ref sequence"))?;
    let first = sequence.iter().next().ok_or_else(|| MoltenError::invalid_harness("missing turn context ref"))?;
    first
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness("expected string turn context ref"))
}

fn remote_dataspace_gate_summary(value: &preserves::IOValue) -> Result<String> {
    if molten::ledger::artifact_kind(value) != "remote-dataspace-gate-receipt" {
        return Err(MoltenError::invalid_harness("not a remote dataspace gate receipt"));
    }
    Ok(format!("remote dataspace gate receipt ref={}", canonical_hash(value)?))
}

fn run_dogfood_command(command: DogfoodCommand) -> Result<()> {
    match command {
        DogfoodCommand::LocalNode {
            state_root,
            out,
            release_gate_out,
        } => {
            let run = operator_dogfood::run_local_node_dogfood(&operator_dogfood::LocalNodeDogfoodInput {
                state_root: &state_root,
            })?;
            write_file(&out, &to_text(&run.report_value)?)?;
            if let (Some(path), Some(value)) = (release_gate_out.as_ref(), run.release_gate_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            println!(
                "dogfood local-node decision={} report={} release-gate={}",
                run.decision,
                run.report_ref,
                run.release_gate_ref.as_deref().unwrap_or("none")
            );
            Ok(())
        }
        DogfoodCommand::NixReleaseExport { output_path, out } => {
            let evidence =
                operator_dogfood::nix_dogfood_release_evidence_value(&operator_dogfood::NixDogfoodEvidenceInput {
                    output_path: &output_path,
                })?;
            let parsed = operator_dogfood::parse_nix_dogfood_evidence(&evidence)?;
            write_file(&out, &to_text(&evidence)?)?;
            println!(
                "dogfood nix-release-export evidence={} report={} release-gate={}",
                parsed.evidence_ref, parsed.report_ref, parsed.release_gate_ref
            );
            Ok(())
        }
        DogfoodCommand::NixReleaseVerify {
            output_path,
            evidence,
            receipt_out,
        } => {
            let evidence_value = read_preserves_file(&evidence)?;
            let receipt = operator_dogfood::verify_nix_dogfood_evidence(&operator_dogfood::NixDogfoodVerifyInput {
                output_path: &output_path,
                evidence_value: &evidence_value,
            })?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood nix-release-verify decision={} receipt={} evidence={}",
                receipt.decision, receipt.receipt_ref, receipt.evidence_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleaseBundleExport { output_path, out } => {
            let bundle =
                operator_dogfood::release_evidence_bundle_value(&operator_dogfood::ReleaseEvidenceBundleInput {
                    output_path: &output_path,
                })?;
            let parsed = operator_dogfood::parse_release_evidence_bundle(&bundle)?;
            write_file(&out, &to_text(&bundle)?)?;
            println!(
                "dogfood release-bundle-export bundle={} report={} release-gate={} nix-verify={}",
                parsed.bundle_ref, parsed.report_ref, parsed.release_gate_ref, parsed.nix_verify_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleaseBundleVerify {
            output_path,
            bundle,
            receipt_out,
            signed_members,
            require_signed_members,
            signed_purpose,
            signed_trust_root,
            signed_key,
            signed_key_ledger,
            signed_key_ref,
            signed_key_id,
            signed_signer,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let signed_member_values = read_preserves_files(&signed_members)?;
            ensure_keyring_selector_has_ledger(
                signed_key_ledger.as_deref(),
                signed_key_ref.as_deref(),
                signed_key_id.as_deref(),
            )?;
            let keyring = match signed_key_ledger.as_ref() {
                Some(ledger) => load_signed_receipt_keyring(ledger)?,
                None => SignedReceiptKeyring {
                    keys: Vec::new(),
                    revocations: Vec::new(),
                },
            };
            let receipt = operator_dogfood::verify_release_evidence_bundle(
                &operator_dogfood::ReleaseEvidenceBundleVerifyInput {
                    output_path: &output_path,
                    bundle_value: &bundle_value,
                    signed_member_values: &signed_member_values,
                    signed_purpose: &signed_purpose,
                    signed_trust_root: &signed_trust_root,
                    signed_key: &signed_key,
                    signed_keys: &keyring.keys,
                    signed_key_revocations: &keyring.revocations,
                    signed_key_ref: signed_key_ref.as_deref(),
                    signed_key_id: signed_key_id.as_deref(),
                    signed_signer: signed_signer.as_deref(),
                    is_signed_members_required: require_signed_members,
                },
            )?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood release-bundle-verify decision={} receipt={} bundle={}",
                receipt.decision, receipt.receipt_ref, receipt.bundle_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleasePromote {
            output_path,
            bundle_verify,
            receipt_out,
            signed_key_ledger,
            signed_trust_root,
            signed_key_ref,
            signed_key_id,
            signed_signer,
            source_evidence,
            octet_evidence,
            cairn_evidence,
        } => {
            let bundle_verify_value = read_preserves_file(&bundle_verify)?;
            let keyring = load_signed_receipt_keyring(&signed_key_ledger)?;
            let receipt =
                operator_dogfood::release_promotion_gate_receipt_value(&operator_dogfood::ReleasePromotionGateInput {
                    output_path: &output_path,
                    bundle_verify_value: &bundle_verify_value,
                    source_evidence: &source_evidence,
                    octet_evidence: &octet_evidence,
                    cairn_evidence: &cairn_evidence,
                    signed_keys: &keyring.keys,
                    signed_key_revocations: &keyring.revocations,
                    signed_trust_root: &signed_trust_root,
                    signed_signer: signed_signer.as_deref(),
                    signed_key_ref: signed_key_ref.as_deref(),
                    signed_key_id: signed_key_id.as_deref(),
                })?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood release-promote decision={} receipt={} bundle-verify={} key={} source={} octet={} cairn={}",
                receipt.decision,
                receipt.receipt_ref,
                receipt.bundle_verify_ref,
                receipt.selected_key_ref,
                receipt.source_ref,
                receipt.octet_ref,
                receipt.cairn_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleasePromotionSummary {
            output_path,
            out,
            signed_key_ledger,
            signed_trust_root,
            signed_key_ref,
            signed_key_id,
            signed_signer,
        } => {
            let key_ledger = signed_key_ledger.unwrap_or_else(|| output_path.join("signed-keyring"));
            let keyring = load_signed_receipt_keyring(&key_ledger)?;
            let summary =
                operator_dogfood::release_promotion_summary_value(&operator_dogfood::ReleasePromotionSummaryInput {
                    output_path: &output_path,
                    signed_keys: &keyring.keys,
                    signed_key_revocations: &keyring.revocations,
                    signed_trust_root: &signed_trust_root,
                    signed_signer: signed_signer.as_deref(),
                    signed_key_ref: signed_key_ref.as_deref(),
                    signed_key_id: signed_key_id.as_deref(),
                })?;
            write_file(&out, &to_text(&summary.value)?)?;
            println!(
                "dogfood release-promotion-summary decision={} summary={} promotion={} signed={} key={} source={} octet={} cairn={}",
                summary.decision,
                summary.summary_ref,
                summary.promotion_ref,
                summary.signed_envelope_ref,
                summary.signed_key_ref,
                summary.source_ref,
                summary.octet_ref,
                summary.cairn_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleaseExport {
            output_path,
            out,
            manifest_out,
        } => {
            let manifest =
                operator_dogfood::release_export_manifest_value(&operator_dogfood::ReleaseExportManifestInput {
                    output_path: &output_path,
                })?;
            write_file(&manifest_out, &to_text(&manifest.value)?)?;
            write_release_export_archive(&output_path, &out, &manifest)?;
            println!(
                "dogfood release-export manifest={} promotion-summary={} members={} archive={}",
                manifest.manifest_ref,
                manifest.promotion_summary_ref,
                manifest.member_refs.len(),
                out.display()
            );
            Ok(())
        }
        DogfoodCommand::ReleaseExportVerify { bundle, receipt_out } => {
            let archive = read_release_export_archive(&bundle)?;
            let receipt = operator_dogfood::verify_release_export(&operator_dogfood::ReleaseExportVerifyInput {
                manifest_value: archive.manifest_value.as_ref(),
                member_refs: &archive.member_refs,
                archive_diagnostics: &archive.diagnostics,
            })?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood release-export-verify decision={} receipt={} manifest={} promotion-summary={}",
                receipt.decision, receipt.receipt_ref, receipt.manifest_ref, receipt.promotion_summary_ref
            );
            Ok(())
        }
        DogfoodCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", operator_dogfood::operator_dogfood_summary(&value)?);
            Ok(())
        }
    }
}

fn run_node_command(command: NodeCommand) -> Result<()> {
    match command {
        NodeCommand::Init {
            state_root,
            node_id,
            config_out,
            identity_receipt_out,
        } => {
            let init = node_daemon::init_local_node(&node_daemon::NodeDaemonInitInput {
                state_root: &state_root,
                node_id: &node_id,
            })?;
            if let Some(path) = config_out.as_ref() {
                write_file(path, &to_text(&init.config_value)?)?;
            }
            if let Some(path) = identity_receipt_out.as_ref() {
                write_file(path, &to_text(&init.identity_receipt_value)?)?;
            }
            println!(
                "node init config={} identity={} identity_receipt={} state_root={}",
                init.config_ref,
                init.identity_ref,
                init.identity_receipt_ref,
                state_root.display()
            );
            Ok(())
        }
        NodeCommand::Run {
            state_root,
            startup_out,
        } => {
            let run = node_daemon::run_local_node(&node_daemon::NodeDaemonRunInput {
                state_root: &state_root,
            })?;
            if let Some(path) = startup_out.as_ref() {
                write_file(path, &to_text(&run.startup_value)?)?;
            }
            println!(
                "node run startup={} adapters={} state_root={}",
                run.startup_ref,
                run.adapter_receipt_refs.len(),
                state_root.display()
            );
            Ok(())
        }
        NodeCommand::RunLoop {
            state_root,
            max_requests,
            receipt_out,
            heartbeat_out,
        } => {
            let loop_run = node_daemon::run_control_loop(&node_daemon::NodeControlLoopInput {
                state_root: &state_root,
                max_requests,
            })?;
            if let Some(path) = heartbeat_out.as_ref() {
                write_file(path, &to_text(&loop_run.heartbeat_receipt_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control loop receipt", &loop_run.loop_receipt_value)?;
            println!(
                "node run-loop loop_receipt={} heartbeat={} processed={} stopped={}",
                loop_run.loop_receipt_ref,
                loop_run.heartbeat_receipt_ref,
                loop_run.processed_request_refs.len(),
                if loop_run.has_stopped { "yes" } else { "no" }
            );
            Ok(())
        }
        NodeCommand::Serve {
            state_root,
            topic,
            max_ticks,
            max_requests_per_tick,
            live_iroh,
            live_max_events,
            live_event_timeout_ms,
            service_receipt_out,
            live_ticket_out,
            supervisor_policy,
            receipt_out,
        } => {
            let supervisor_policy_value =
                supervisor_policy.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            if live_iroh {
                let runtime =
                    tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
                let served = runtime.block_on(node_daemon::serve_node_control_live_listener(
                    &node_daemon::NodeControlLiveServeInput {
                        state_root: &state_root,
                        topic: &topic,
                        max_events: live_max_events,
                        event_timeout_ms: live_event_timeout_ms,
                        max_requests_per_tick,
                        supervisor_policy_value: supervisor_policy_value.as_ref(),
                    },
                ))?;
                if let Some(path) = service_receipt_out.as_ref() {
                    write_file(path, &to_text(&served.service.service_receipt_value)?)?;
                }
                if let Some(path) = live_ticket_out.as_ref()
                    && let Some(ticket_value) = served.live_ticket_value.as_ref()
                {
                    write_file(path, &to_text(ticket_value)?)?;
                }
                emit_named_receipt(
                    receipt_out.as_ref(),
                    "node control live listener receipt",
                    &served.listener_receipt_value,
                )?;
                println!(
                    "node serve live-iroh listener={} service={} endpoint={} events={} transports={} processed={} stopped={}",
                    served.listener_receipt_ref,
                    served.service.service_receipt_ref,
                    served.bound_endpoint_id,
                    served.observed_events,
                    served.transport_receipt_refs.len(),
                    served.service.processed_request_refs.len(),
                    if served.service.has_stopped { "yes" } else { "no" }
                );
                Ok(())
            } else {
                let served = node_daemon::serve_node_control(&node_daemon::NodeControlServeInput {
                    state_root: &state_root,
                    topic: &topic,
                    max_ticks,
                    max_requests_per_tick,
                    supervisor_policy_value: supervisor_policy_value.as_ref(),
                })?;
                emit_named_receipt(
                    receipt_out.as_ref(),
                    "node control service run receipt",
                    &served.service_receipt_value,
                )?;
                println!(
                    "node serve decision={} receipt={} ticks={} heartbeats={} ingress={} loops={} processed={} stopped={}",
                    served.decision,
                    served.service_receipt_ref,
                    served.ticks,
                    served.heartbeat_receipt_refs.len(),
                    served.ingress_receipt_refs.len(),
                    served.loop_receipt_refs.len(),
                    served.processed_request_refs.len(),
                    if served.has_stopped { "yes" } else { "no" }
                );
                Ok(())
            }
        }
        NodeCommand::Status {
            state_root,
            health_out,
            receipt_out,
        } => {
            let status = node_daemon::status_local_node(&node_daemon::NodeDaemonStatusInput {
                state_root: &state_root,
            })?;
            if let Some(path) = health_out.as_ref() {
                write_file(path, &to_text(&status.health_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &status.control_receipt_value)?;
            println!(
                "node status {} health={} control_receipt={}",
                status.status, status.health_ref, status.control_receipt_ref
            );
            Ok(())
        }
        NodeCommand::Stop {
            state_root,
            shutdown_out,
            receipt_out,
        } => {
            let stop = node_daemon::stop_local_node(&node_daemon::NodeDaemonStopInput {
                state_root: &state_root,
            })?;
            if let Some(path) = shutdown_out.as_ref() {
                write_file(path, &to_text(&stop.shutdown_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &stop.control_receipt_value)?;
            println!("node stop shutdown={} control_receipt={}", stop.shutdown_ref, stop.control_receipt_ref);
            Ok(())
        }
        NodeCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", node_daemon::node_daemon_summary(&value)?);
            Ok(())
        }
        NodeCommand::ControlRequest {
            operation,
            out,
            target,
            payload,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
        } => {
            let value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: &operation,
                target_ref: target.as_deref(),
                payload_ref: payload.as_deref(),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &evidence_refs,
            })?;
            write_file(&out, &to_text(&value)?)?;
            println!("node control request {} written to {}", canonical_hash(&value)?, out.display());
            Ok(())
        }
        NodeCommand::ProvenanceFixture { artifact_ref, out } => {
            let value = provenance::synthetic_reviewed_provenance_record(&artifact_ref)?;
            write_file(&out, &to_text(&value)?)?;
            println!("node provenance fixture {} written to {}", canonical_hash(&value)?, out.display());
            Ok(())
        }
        NodeCommand::AuthorityGrantFixture {
            state_root,
            peer,
            node,
            operations,
            target_scope,
            resource_scope,
            epoch,
            expires_at,
            policy_refs,
            revocation_refs,
            evidence_refs,
            out,
        } => {
            let operations = if operations.is_empty() {
                vec!["status".to_string()]
            } else {
                operations
            };
            let value =
                node_daemon::node_control_authority_grant_value(&node_daemon::NodeControlAuthorityGrantInput {
                    peer_id: &peer,
                    node_id: &node,
                    operations: &operations,
                    target_scope: &target_scope,
                    resource_scope: &resource_scope,
                    epoch,
                    expires_at,
                    policy_refs: &policy_refs,
                    revocation_refs: &revocation_refs,
                    evidence_refs: &evidence_refs,
                })?;
            let grant_ref = canonical_hash(&value)?;
            write_file(&out, &to_text(&value)?)?;
            if let Some(state_root) = state_root.as_ref() {
                node_daemon::import_node_control_authority_grant(state_root, &value)?;
            }
            println!("node authority grant {} written to {}", grant_ref, out.display());
            Ok(())
        }
        NodeCommand::AuthorityGrantImport {
            state_root,
            grant,
            peer,
            node,
            operations,
            target_scope,
            resource_scope,
            as_of_epoch,
            receipt_out,
        } => {
            let grant_value = read_preserves_file(&grant)?;
            let imported = node_daemon::import_node_control_authority_grant_checked(
                &node_daemon::NodeControlAuthorityGrantImportInput {
                    state_root: &state_root,
                    grant_value: &grant_value,
                    expected_peer: peer.as_deref(),
                    expected_node: node.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(receipt_out.as_ref(), "node authority grant import receipt", &imported.receipt_value)?;
            println!(
                "node authority grant import decision={} grant={} imported={} diagnostics={}",
                imported.decision,
                imported.grant_ref,
                imported.imported_refs.len(),
                imported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::SupervisorPolicyFixture {
            state_root,
            max_restarts,
            restart_window_ticks,
            heartbeat_timeout_ticks,
            shutdown_drain_ticks,
            allow_stale_lock_recovery,
            policy_refs,
            evidence_refs,
            out,
        } => {
            let value =
                node_daemon::node_control_supervisor_policy_value(&node_daemon::NodeControlSupervisorPolicyInput {
                    max_restarts,
                    restart_window_ticks,
                    heartbeat_timeout_ticks,
                    shutdown_drain_ticks,
                    stale_lock_recovery: allow_stale_lock_recovery,
                    policy_refs: &policy_refs,
                    evidence_refs: &evidence_refs,
                })?;
            let policy_ref = canonical_hash(&value)?;
            write_file(&out, &to_text(&value)?)?;
            if let Some(state_root) = state_root.as_ref() {
                node_daemon::import_node_control_supervisor_policy(state_root, &value)?;
            }
            println!("node supervisor policy {} written to {}", policy_ref, out.display());
            Ok(())
        }
        NodeCommand::LiveTicketExport {
            state_root,
            topic,
            policy_refs,
            evidence_refs,
            out,
        } => {
            let ticket =
                node_daemon::export_node_control_live_ticket(&node_daemon::NodeControlLiveTicketExportInput {
                    state_root: &state_root,
                    topic: &topic,
                    policy_refs: &policy_refs,
                    evidence_refs: &evidence_refs,
                })?;
            write_file(&out, &to_text(&ticket.value)?)?;
            println!("node live ticket {} written to {}", ticket.ticket_ref, out.display());
            Ok(())
        }
        NodeCommand::LiveTicketImport {
            state_root,
            ticket,
            peer_admission,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            as_of_sequence,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let peer_admission_value = peer_admission.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let imported =
                node_daemon::import_node_control_live_ticket(&node_daemon::NodeControlLiveTicketImportInput {
                    state_root: &state_root,
                    ticket_value: &ticket_value,
                    peer_admission_value: peer_admission_value.as_ref(),
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    as_of_sequence,
                })?;
            emit_named_receipt(receipt_out.as_ref(), "node live ticket import receipt", &imported.receipt_value)?;
            println!(
                "node live ticket import decision={} ticket={} admission={} imported={} diagnostics={}",
                imported.decision,
                imported.ticket_ref,
                imported.peer_admission_ref.as_deref().unwrap_or("none"),
                imported.imported_refs.len(),
                imported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LivePeerAdmit {
            state_root,
            peer,
            sequence,
            expires_at,
            policy_refs,
            evidence_refs,
            receipt_out,
            ticket,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let admission = node_daemon::admit_node_control_live_peer(&node_daemon::NodeControlLivePeerAdmitInput {
                state_root: &state_root,
                ticket_value: &ticket_value,
                peer_id: &peer,
                sequence,
                expires_at,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node live peer admission", &admission.value)?;
            println!(
                "node live peer admit decision={} admission={} peer={} node={} topic={}",
                admission.decision, admission.admission_ref, admission.peer_id, admission.node_id, admission.topic
            );
            Ok(())
        }
        NodeCommand::ControlSubmit {
            state_root,
            request,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let submitted = node_daemon::submit_control_request(&node_daemon::NodeControlSubmitInput {
                state_root: &state_root,
                request_value: &request_value,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control queue receipt", &submitted.queue_receipt_value)?;
            println!(
                "node control submit request={} queue_receipt={} inbox={}",
                submitted.request_ref,
                submitted.queue_receipt_ref,
                submitted.inbox_path.display()
            );
            Ok(())
        }
        NodeCommand::ControlDispatch {
            state_root,
            request,
            receipt_out,
        } => {
            let dispatched = node_daemon::dispatch_control_request(&node_daemon::NodeControlDispatchInput {
                state_root: &state_root,
                request_path: request.as_deref(),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &dispatched.control_receipt_value)?;
            println!(
                "node control dispatch operation={} request={} control_receipt={} subreceipts={}",
                dispatched.operation,
                dispatched.request_ref,
                dispatched.control_receipt_ref,
                dispatched.subreceipt_refs.len()
            );
            Ok(())
        }
        NodeCommand::ControlIngressBuild {
            request,
            out,
            from_peer,
            to_node,
            topic,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
        } => {
            let request_value = read_preserves_file(&request)?;
            let envelope = node_daemon::node_control_ingress_envelope(&node_daemon::NodeControlIngressEnvelopeInput {
                request_value: &request_value,
                from_peer: &from_peer,
                to_node: &to_node,
                topic: &topic,
                sequence,
                peer_bootstrap_refs: &peer_bootstrap_refs,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &evidence_refs,
            })?;
            write_file(&out, &to_text(&envelope.value)?)?;
            println!(
                "node control ingress envelope={} request={} written to {}",
                envelope.envelope_ref,
                envelope.request.request_ref,
                out.display()
            );
            Ok(())
        }
        NodeCommand::ControlIngressLiveBuild {
            request,
            out,
            from_peer,
            to_node,
            topic,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
        } => {
            let request_value = read_preserves_file(&request)?;
            let envelope =
                node_daemon::node_control_live_ingress_envelope(&node_daemon::NodeControlIngressEnvelopeInput {
                    request_value: &request_value,
                    from_peer: &from_peer,
                    to_node: &to_node,
                    topic: &topic,
                    sequence,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                })?;
            write_file(&out, &to_text(&envelope.value)?)?;
            println!(
                "node control live ingress envelope={} request={} written to {}",
                envelope.envelope_ref,
                envelope.request.request_ref,
                out.display()
            );
            Ok(())
        }
        NodeCommand::ControlIngressLiveLoopback {
            state_root,
            request,
            from_peer,
            to_node,
            topic,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
            publish_receipt_out,
            receive_receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let loopback = runtime.block_on(node_daemon::node_control_live_iroh_loopback(
                &node_daemon::NodeControlLiveLoopbackInput {
                    state_root: &state_root,
                    request_value: &request_value,
                    from_peer: &from_peer,
                    to_node: &to_node,
                    topic: &topic,
                    sequence,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                },
            ))?;
            if let Some(path) = publish_receipt_out.as_ref() {
                write_file(path, &to_text(&loopback.publish_receipt_value)?)?;
            }
            emit_named_receipt(
                receive_receipt_out.as_ref(),
                "node control live transport receipt",
                &loopback.receive_receipt_value,
            )?;
            println!(
                "node control live ingress loopback envelope={} publish_receipt={} receive_receipt={} ingress_receipt={} enqueued={}",
                loopback.envelope_ref,
                loopback.publish_receipt_ref,
                loopback.receive_receipt_ref,
                loopback.ingress_receipt_ref,
                if loopback.has_enqueued { "yes" } else { "no" }
            );
            Ok(())
        }
        NodeCommand::ControlIngressLiveSend {
            state_root,
            request,
            ticket,
            from_peer,
            sequence,
            operation_id,
            expected_node,
            expected_topic,
            expected_endpoint,
            max_attempts,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
            join_timeout_ms,
            transport_receipt_out,
            retry_receipts_dir,
            duplicate_receipt_out,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let ticket_value = read_preserves_file(&ticket)?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let sent = runtime.block_on(node_daemon::send_node_control_live_ingress(
                &node_daemon::NodeControlLiveSendInput {
                    state_root: state_root.as_deref(),
                    request_value: &request_value,
                    receiver_ticket_value: &ticket_value,
                    from_peer: &from_peer,
                    sequence,
                    expected_operation_ref: operation_id.as_deref(),
                    expected_receiver_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    max_attempts,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                    join_timeout_ms,
                },
            ))?;
            if let (Some(path), Some(value)) = (transport_receipt_out.as_ref(), sent.transport_receipt_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            if let Some(dir) = retry_receipts_dir.as_ref() {
                fs::create_dir_all(dir).map_err(MoltenError::from)?;
                for (reference, value) in sent.retry_receipt_refs.iter().zip(sent.retry_receipt_values.iter()) {
                    let path = dir.join(format!("{}.preserves", reference.replace(':', "-")));
                    write_file(&path, &to_text(value)?)?;
                }
            }
            if let (Some(path), Some(value)) = (duplicate_receipt_out.as_ref(), sent.duplicate_receipt_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "node control live send receipt", &sent.send_receipt_value)?;
            println!(
                "node control live ingress send envelope={} operation={} ticket={} endpoint={} transport_receipt={} send_receipt={} retries={} duplicate_receipt={}",
                sent.envelope_ref,
                sent.operation_ref,
                sent.receiver_ticket_ref,
                sent.receiver_endpoint_id,
                sent.transport_receipt_ref.as_deref().unwrap_or("none"),
                sent.send_receipt_ref,
                sent.retry_receipt_refs.len(),
                sent.duplicate_receipt_ref.as_deref().unwrap_or("none")
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundle {
            state_root,
            ticket,
            peer_admission,
            authority_grant,
            send_receipt,
            receive_receipts,
            listener_receipt,
            service_receipt,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let peer_admission_value = read_preserves_file(&peer_admission)?;
            let authority_grant_value = read_preserves_file(&authority_grant)?;
            let send_receipt_value = read_preserves_file(&send_receipt)?;
            let receive_values =
                receive_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
            let listener_value = listener_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let service_receipt_value = read_preserves_file(&service_receipt)?;
            let workflow =
                node_daemon::node_control_live_workflow_receipt(&node_daemon::NodeControlLiveWorkflowInput {
                    state_root: state_root.as_deref(),
                    receiver_ticket_value: &ticket_value,
                    peer_admission_value: &peer_admission_value,
                    authority_grant_value: &authority_grant_value,
                    send_receipt_value: &send_receipt_value,
                    receive_receipt_values: &receive_value_refs,
                    listener_receipt_value: listener_value.as_ref(),
                    service_receipt_value: &service_receipt_value,
                })?;
            emit_named_receipt(receipt_out.as_ref(), "node control live workflow receipt", &workflow.receipt_value)?;
            println!(
                "node live workflow bundle decision={} receipt={} diagnostics={}",
                workflow.decision,
                workflow.receipt_ref,
                workflow.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleExport {
            ticket,
            peer_admission,
            authority_grant,
            receipt_values,
            out,
            receipt_out,
        } => {
            let ticket_value = read_preserves_file(&ticket)?;
            let peer_admission_value = read_preserves_file(&peer_admission)?;
            let authority_grant_value = read_preserves_file(&authority_grant)?;
            let receipt_values =
                receipt_values.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let receipt_value_refs = receipt_values.iter().collect::<Vec<_>>();
            let exported = node_daemon::export_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleExportInput {
                    receiver_ticket_value: &ticket_value,
                    peer_admission_value: &peer_admission_value,
                    authority_grant_value: &authority_grant_value,
                    receipt_values: &receipt_value_refs,
                },
            )?;
            write_file(&out, &to_text(&exported.bundle.bundle_value)?)?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle export receipt",
                &exported.receipt_value,
            )?;
            println!(
                "node live workflow bundle export decision={} bundle={} ticket={} admission={} grant={} diagnostics={}",
                exported.decision,
                exported.bundle.bundle_ref,
                exported.bundle.ticket_ref,
                exported.bundle.peer_admission_ref,
                exported.bundle.authority_grant_ref,
                exported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleVerify {
            bundle,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let verified = node_daemon::verify_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleVerifyInput {
                    bundle_value: &bundle_value,
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle verify receipt",
                &verified.receipt_value,
            )?;
            println!(
                "node live workflow bundle verify decision={} bundle={} ticket={} admission={} grant={} diagnostics={}",
                verified.decision,
                verified.bundle_ref,
                verified.ticket_ref.as_deref().unwrap_or("none"),
                verified.peer_admission_ref.as_deref().unwrap_or("none"),
                verified.authority_grant_ref.as_deref().unwrap_or("none"),
                verified.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleGate {
            bundle,
            verify_receipt,
            require_verify_receipt,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let verify_receipt_value = verify_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let gated = node_daemon::gate_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleGateInput {
                    bundle_value: &bundle_value,
                    verify_receipt_value: verify_receipt_value.as_ref(),
                    require_verify_receipt,
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle gate receipt",
                &gated.receipt_value,
            )?;
            println!(
                "node live workflow bundle gate decision={} bundle={} verify={} recomputed-verify={} diagnostics={}",
                gated.decision,
                gated.bundle_ref,
                gated.verify_receipt_ref.as_deref().unwrap_or("none"),
                gated.recomputed_verify_receipt_ref,
                gated.diagnostics.len()
            );
            print_live_workflow_bundle_gate_next_step(&gated);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleApply {
            state_root,
            bundle,
            gate_receipt,
            require_gate_receipt,
            request,
            send,
            from_peer,
            sequence,
            operation_id,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs,
            max_attempts,
            join_timeout_ms,
            send_receipt_out,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let gate_receipt_value = gate_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let request_value = request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let runtime =
                tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(MoltenError::from)?;
            let applied = runtime.block_on(node_daemon::apply_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleApplyInput {
                    state_root: &state_root,
                    bundle_value: &bundle_value,
                    gate_receipt_value: gate_receipt_value.as_ref(),
                    is_gate_receipt_required: require_gate_receipt,
                    request_value: request_value.as_ref(),
                    should_send: send,
                    from_peer: from_peer.as_deref(),
                    sequence,
                    expected_operation_ref: operation_id.as_deref(),
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                    peer_bootstrap_refs: &peer_bootstrap_refs,
                    authority_refs: &authority_refs,
                    policy_refs: &policy_refs,
                    resource_refs: &resource_refs,
                    evidence_refs: &evidence_refs,
                    max_attempts,
                    join_timeout_ms,
                },
            ))?;
            if let (Some(path), Some(value)) = (send_receipt_out.as_ref(), applied.send_receipt_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle apply receipt",
                &applied.receipt_value,
            )?;
            println!(
                "node live workflow bundle apply decision={} bundle={} gate={} import={} imported={} send={} diagnostics={}",
                applied.decision,
                applied.bundle_ref,
                applied.gate_receipt_ref.as_deref().unwrap_or("none"),
                applied.import_receipt_ref.as_deref().unwrap_or("none"),
                applied.imported_refs.len(),
                applied.send_receipt_ref.as_deref().unwrap_or("none"),
                applied.diagnostics.len()
            );
            print_live_workflow_bundle_apply_next_step(&applied, request_value.is_some(), send);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleReconcile {
            apply_receipt,
            send_receipt,
            ingress_receipt,
            queue_receipt,
            control_receipt,
            expected_envelope,
            expected_operation,
            expected_request,
            receipt_out,
        } => {
            let apply_receipt_value = read_preserves_file(&apply_receipt)?;
            let send_receipt_value = send_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let ingress_receipt_value = ingress_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let queue_receipt_value = queue_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let control_receipt_value = control_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let reconciled = node_daemon::reconcile_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleReconcileInput {
                    apply_receipt_value: &apply_receipt_value,
                    send_receipt_value: send_receipt_value.as_ref(),
                    ingress_receipt_value: ingress_receipt_value.as_ref(),
                    queue_receipt_value: queue_receipt_value.as_ref(),
                    control_receipt_value: control_receipt_value.as_ref(),
                    expected_envelope_ref: expected_envelope.as_deref(),
                    expected_operation_ref: expected_operation.as_deref(),
                    expected_request_ref: expected_request.as_deref(),
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle reconcile receipt",
                &reconciled.receipt_value,
            )?;
            println!(
                "node live workflow bundle reconcile decision={} bundle={} apply={} ingress={} queue={} control={} diagnostics={}",
                reconciled.decision,
                reconciled.bundle_ref,
                reconciled.apply_receipt_ref,
                reconciled.ingress_receipt_ref.as_deref().unwrap_or("none"),
                reconciled.queue_receipt_ref.as_deref().unwrap_or("none"),
                reconciled.control_receipt_ref.as_deref().unwrap_or("none"),
                reconciled.diagnostics.len()
            );
            print_live_workflow_bundle_reconcile_next_step(&reconciled);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleAckExport {
            apply_receipt,
            send_receipt,
            ingress_receipt,
            queue_receipt,
            control_receipt,
            reconcile_receipt,
            out,
            receipt_out,
        } => {
            let apply_receipt_value = read_preserves_file(&apply_receipt)?;
            let send_receipt_value = send_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let ingress_receipt_value = ingress_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let queue_receipt_value = queue_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let control_receipt_value = control_receipt.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let reconcile_receipt_value = read_preserves_file(&reconcile_receipt)?;
            let exported = node_daemon::export_node_control_live_workflow_bundle_ack(
                &node_daemon::NodeControlLiveWorkflowBundleAckExportInput {
                    apply_receipt_value: &apply_receipt_value,
                    send_receipt_value: send_receipt_value.as_ref(),
                    ingress_receipt_value: ingress_receipt_value.as_ref(),
                    queue_receipt_value: queue_receipt_value.as_ref(),
                    control_receipt_value: control_receipt_value.as_ref(),
                    reconcile_receipt_value: &reconcile_receipt_value,
                },
            )?;
            write_file(&out, &to_text(&exported.ack.ack_value)?)?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle ack export receipt",
                &exported.receipt_value,
            )?;
            println!(
                "node live workflow bundle ack export decision={} ack={} bundle={} receiver_decision={} diagnostics={}",
                exported.decision,
                exported.ack.ack_ref,
                exported.ack.bundle_ref,
                exported.receiver_decision,
                exported.diagnostics.len()
            );
            print_live_workflow_bundle_ack_export_next_step(&exported);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleAckImport {
            state_root,
            ack,
            expected_bundle,
            expected_envelope,
            expected_operation,
            expected_request,
            receipt_out,
        } => {
            let ack_value = read_preserves_file(&ack)?;
            let imported = node_daemon::import_node_control_live_workflow_bundle_ack(
                &node_daemon::NodeControlLiveWorkflowBundleAckImportInput {
                    state_root: &state_root,
                    ack_value: &ack_value,
                    expected_bundle_ref: expected_bundle.as_deref(),
                    expected_envelope_ref: expected_envelope.as_deref(),
                    expected_operation_ref: expected_operation.as_deref(),
                    expected_request_ref: expected_request.as_deref(),
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle ack import receipt",
                &imported.receipt_value,
            )?;
            println!(
                "node live workflow bundle ack import decision={} ack={} bundle={} imported={} receiver_decision={} diagnostics={}",
                imported.decision,
                imported.ack_ref,
                imported.bundle_ref,
                imported.imported_refs.len(),
                imported.receiver_decision,
                imported.diagnostics.len()
            );
            print_live_workflow_bundle_ack_import_next_step(&imported);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleProtocolGate {
            bundle,
            gate_receipt,
            apply_receipt,
            reconcile_receipt,
            ack,
            expected_envelope,
            expected_operation,
            expected_request,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let gate_receipt_value = read_preserves_file(&gate_receipt)?;
            let apply_receipt_value = read_preserves_file(&apply_receipt)?;
            let reconcile_receipt_value = read_preserves_file(&reconcile_receipt)?;
            let ack_value = read_preserves_file(&ack)?;
            let gated = node_daemon::gate_node_control_live_workflow_protocol(
                &node_daemon::NodeControlLiveWorkflowProtocolGateInput {
                    bundle_value: &bundle_value,
                    gate_receipt_value: &gate_receipt_value,
                    apply_receipt_value: &apply_receipt_value,
                    reconcile_receipt_value: &reconcile_receipt_value,
                    ack_value: &ack_value,
                    expected_envelope_ref: expected_envelope.as_deref(),
                    expected_operation_ref: expected_operation.as_deref(),
                    expected_request_ref: expected_request.as_deref(),
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow protocol gate receipt",
                &gated.receipt_value,
            )?;
            println!(
                "node live workflow protocol gate decision={} receipt={} protocol={} session={} operations={} messages={} diagnostics={}",
                gated.decision,
                gated.receipt_ref,
                gated.protocol_ref,
                gated.session_id,
                gated.operation_count,
                gated.message_count,
                gated.diagnostics.len()
            );
            print_live_workflow_protocol_gate_next_step(&gated);
            Ok(())
        }
        NodeCommand::LiveWorkflowBundleImport {
            state_root,
            bundle,
            expected_node,
            expected_topic,
            expected_endpoint,
            expected_peer,
            operations,
            target_scope,
            resource_scope,
            as_of_sequence,
            as_of_epoch,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let imported = node_daemon::import_node_control_live_workflow_bundle(
                &node_daemon::NodeControlLiveWorkflowBundleImportInput {
                    state_root: &state_root,
                    bundle_value: &bundle_value,
                    expected_node: expected_node.as_deref(),
                    expected_topic: expected_topic.as_deref(),
                    expected_endpoint: expected_endpoint.as_deref(),
                    expected_peer: expected_peer.as_deref(),
                    expected_operations: &operations,
                    expected_target_scope: target_scope.as_deref(),
                    expected_resource_scope: resource_scope.as_deref(),
                    as_of_sequence,
                    as_of_epoch,
                },
            )?;
            emit_named_receipt(
                receipt_out.as_ref(),
                "node control live workflow bundle import receipt",
                &imported.receipt_value,
            )?;
            println!(
                "node live workflow bundle import decision={} bundle={} ticket_import={} authority_import={} imported={} diagnostics={}",
                imported.decision,
                imported.bundle_ref,
                imported.ticket_import_ref.as_deref().unwrap_or("none"),
                imported.authority_import_ref.as_deref().unwrap_or("none"),
                imported.imported_refs.len(),
                imported.diagnostics.len()
            );
            Ok(())
        }
        NodeCommand::ControlIngressPublish {
            state_root,
            envelope,
            receipt_out,
        } => {
            let envelope_value = read_preserves_file(&envelope)?;
            let published = node_daemon::publish_node_control_ingress(&node_daemon::NodeControlIngressPublishInput {
                state_root: &state_root,
                envelope_value: &envelope_value,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control ingress receipt", &published.receipt_value)?;
            println!(
                "node control ingress publish envelope={} receipt={} path={}",
                published.envelope_ref,
                published.receipt_ref,
                published.envelope_path.display()
            );
            Ok(())
        }
        NodeCommand::ControlIngressDeliver {
            state_root,
            topic,
            envelope_ref,
            receipt_out,
        } => {
            let delivered = node_daemon::deliver_node_control_ingress(&node_daemon::NodeControlIngressDeliverInput {
                state_root: &state_root,
                topic: &topic,
                envelope_ref: &envelope_ref,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node control ingress receipt", &delivered.ingress_receipt_value)?;
            println!(
                "node control ingress deliver envelope={} request={} receipt={} enqueued={}",
                delivered.envelope_ref,
                delivered.request_ref,
                delivered.ingress_receipt_ref,
                if delivered.has_enqueued { "yes" } else { "no" }
            );
            Ok(())
        }
        NodeCommand::ControlDeny {
            request,
            startup,
            diagnostic,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let request = node_runtime::parse_node_control_request(&request_value)?;
            let receipt = node_runtime::node_control_deny_receipt_value(&request, &startup, &diagnostic)?;
            emit_named_receipt(receipt_out.as_ref(), "node control receipt", &receipt)?;
            println!("node control deny receipt={}", canonical_hash(&receipt)?);
            Ok(())
        }
        NodeCommand::Shutdown {
            startup,
            adapters,
            drained_jobs,
            index_receipt_refs,
            receipt_out,
        } => {
            let adapter_receipts = parse_node_adapter_receipt_args(&adapters)?;
            let receipt = node_runtime::node_shutdown_receipt_value(&node_runtime::ShutdownReceiptValueInput {
                decision: "pass",
                startup_receipt_ref: &startup,
                adapter_receipts: &adapter_receipts,
                drained_job_refs: &drained_jobs,
                index_receipt_refs: &index_receipt_refs,
                diagnostics: &[],
            })?;
            emit_named_receipt(receipt_out.as_ref(), "node shutdown receipt", &receipt)?;
            println!("node shutdown receipt={}", canonical_hash(&receipt)?);
            Ok(())
        }
        NodeCommand::Health {
            startup_receipt,
            shutdown,
            index_receipt_refs,
            head_refs,
            open_job_refs,
            receipt_out,
        } => {
            let startup_value = read_preserves_file(&startup_receipt)?;
            let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
            let receipt =
                node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
                    startup_receipt: &startup,
                    shutdown_receipt_ref: shutdown.as_deref(),
                    index_receipt_refs: &index_receipt_refs,
                    head_refs: &head_refs,
                    open_job_refs: &open_job_refs,
                    diagnostics: &[],
                })?;
            emit_named_receipt(receipt_out.as_ref(), "node health receipt", &receipt)?;
            println!("node health receipt={}", canonical_hash(&receipt)?);
            Ok(())
        }
    }
}

fn print_live_workflow_bundle_ack_export_next_step(exported: &node_daemon::NodeControlLiveWorkflowBundleAckExport) {
    if exported.decision != "pass" {
        println!(
            "next-step=collect-receiver-evidence command=\"molten node live-workflow-bundle-reconcile ... --ingress-receipt <receipt> --queue-receipt <receipt>\""
        );
        return;
    }
    println!(
        "next-step=import-ack command=\"molten node live-workflow-bundle-ack-import --state-root <sender> <ack>\""
    );
}

fn print_live_workflow_bundle_ack_import_next_step(imported: &node_daemon::NodeControlLiveWorkflowBundleAckImport) {
    if imported.decision != "pass" {
        println!("next-step=inspect-ack-diagnostics command=\"molten node show <ack-import-receipt>\"");
        return;
    }
    if imported.receiver_decision == "pass" {
        println!("next-step=inspect-receiver-control command=\"molten node show <control-receipt>\"");
    } else {
        println!("next-step=inspect-receiver-denial command=\"molten node show <reconcile-receipt>\"");
    }
}

fn print_live_workflow_protocol_gate_next_step(gated: &node_daemon::NodeControlLiveWorkflowProtocolGate) {
    if gated.decision == "pass" {
        println!("next-step=archive-workflow-protocol command=\"molten node show <protocol-gate-receipt>\"");
    } else {
        println!(
            "next-step=inspect-workflow-protocol-diagnostics command=\"molten node show <protocol-gate-receipt>\""
        );
    }
}

fn print_live_workflow_bundle_reconcile_next_step(reconciled: &node_daemon::NodeControlLiveWorkflowBundleReconcile) {
    if reconciled.decision == "pass" {
        if reconciled.control_receipt_ref.is_some() {
            println!("next-step=inspect-control-receipt command=\"molten node show <control-receipt>\"");
        } else {
            println!("next-step=run-receiver-control-loop command=\"molten node run-loop --state-root <receiver>\"");
        }
        return;
    }
    let has_missing_ingress = reconciled
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"));
    if has_missing_ingress {
        println!(
            "next-step=wait-or-import-receiver-ingress command=\"molten node live-workflow-bundle-reconcile ... --ingress-receipt <receipt>\""
        );
        return;
    }
    let has_control_denial =
        reconciled.diagnostics.iter().any(|diagnostic| diagnostic.contains("receiver control receipt"));
    if has_control_denial {
        println!("next-step=inspect-receiver-denial command=\"molten node show <control-receipt>\"");
        return;
    }
    println!("next-step=inspect-reconcile-diagnostics command=\"molten node show <reconcile-receipt>\"");
}

fn print_live_workflow_bundle_apply_next_step(
    applied: &node_daemon::NodeControlLiveWorkflowBundleApply,
    has_request: bool,
    was_send_requested: bool,
) {
    if applied.decision == "pass" {
        if was_send_requested {
            println!("next-step=inspect-live-send-receipt command=\"molten node show <send-receipt>\"");
        } else if has_request {
            println!(
                "next-step=send-live-workflow-bundle command=\"molten node live-workflow-bundle-apply ... --send\""
            );
        } else {
            println!(
                "next-step=dry-run-or-send-request command=\"molten node live-workflow-bundle-apply ... --request <request> [--send]\""
            );
        }
        return;
    }
    let has_gate_problem = applied.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("gate receipt")
            || diagnostic.contains("requires a current gate receipt")
            || diagnostic.contains("recomputed verify")
    });
    if has_gate_problem {
        println!("next-step=rerun-gate command=\"molten node live-workflow-bundle-gate ... --receipt-out ...\"");
        return;
    }
    let has_address_problem = applied
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("no endpoint addresses") || diagnostic.contains("unsupported address"));
    if has_address_problem {
        println!("next-step=refresh-bound-live-ticket command=\"molten node serve --live-iroh --live-ticket-out ...\"");
        return;
    }
    println!("next-step=inspect-apply-diagnostics command=\"molten node show <apply-receipt>\"");
}

fn print_live_workflow_bundle_gate_next_step(gated: &node_daemon::NodeControlLiveWorkflowBundleGate) {
    if gated.decision == "pass" {
        println!("next-step=import-bundle command=\"molten node live-workflow-bundle-import ...\"");
        return;
    }
    let has_malformed_bundle = gated.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("bundle parse failed") || diagnostic.contains("unsupported receipt kind")
    });
    if has_malformed_bundle {
        println!("next-step=fix-malformed-bundle rerun=\"molten node live-workflow-bundle-verify ...\"");
        return;
    }
    let has_verify_receipt_problem = gated.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("verify receipt")
            || diagnostic.contains("requires a current verify receipt")
            || diagnostic.contains("does not match recomputed")
    });
    if has_verify_receipt_problem {
        println!(
            "next-step=rerun-verify-receipt command=\"molten node live-workflow-bundle-verify ... --receipt-out ...\""
        );
        return;
    }
    println!(
        "next-step=import-missing-ticket-or-grant command=\"molten node live-ticket-import ...; molten node authority-grant-import ...\""
    );
}

fn parse_node_adapter_receipt_args(args: &[String]) -> Result<Vec<node_runtime::NodeAdapterReceiptRef>> {
    args.iter()
        .map(|arg| {
            let (name, receipt_ref) = arg.split_once('=').ok_or_else(|| {
                MoltenError::invalid_harness(format!("node adapter receipt arg `{arg}` must be name=blake3:ref"))
            })?;
            Ok(node_runtime::NodeAdapterReceiptRef {
                name: name.to_string(),
                receipt_ref: receipt_ref.to_string(),
            })
        })
        .collect()
}

fn run_octet_command(command: OctetCommand) -> Result<()> {
    match command {
        OctetCommand::Gate {
            artifacts,
            profile,
            receipt_out,
        } => {
            let evaluation = octet_gate::evaluate_octet_gate(&octet_gate::OctetGateInput {
                artifacts_dir: artifacts.clone(),
                profile,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet gate receipt", &evaluation.receipt_value)?;
            if evaluation.decision != "pass" {
                return Err(MoltenError::invalid_harness(format!(
                    "octet gate denied receipt={} artifacts={}",
                    evaluation.receipt_ref,
                    artifacts.display()
                )));
            }
            println!("octet gate pass receipt={}", evaluation.receipt_ref);
            Ok(())
        }
        OctetCommand::SourceGate { command } => run_octet_source_gate_command(command),
        OctetCommand::Baseline { command } => run_octet_baseline_command(command),
        OctetCommand::Review { command } => run_octet_review_command(command),
        OctetCommand::Artifacts { command } => run_octet_artifacts_command(command),
        OctetCommand::Remediation { command } => run_octet_remediation_command(command),
    }
}

fn run_octet_remediation_command(command: OctetRemediationCommand) -> Result<()> {
    match command {
        OctetRemediationCommand::Plan {
            artifacts,
            lib_artifacts,
            focused_object_corpus,
            receipt_out,
        } => {
            let plan =
                octet_remediation::build_octet_remediation_plan(&octet_remediation::OctetRemediationPlanInput {
                    artifacts_dir: artifacts,
                    lib_artifacts_dir: lib_artifacts,
                    focused_object_corpus,
                })?;
            emit_named_receipt(receipt_out.as_ref(), "octet remediation plan", &plan.value)?;
            println!("octet remediation plan receipt={}", plan.plan_ref);
            Ok(())
        }
    }
}

fn run_octet_source_gate_command(command: OctetSourceGateCommand) -> Result<()> {
    match command {
        OctetSourceGateCommand::Validate {
            consumer,
            subject,
            gate_receipt,
            source_scope,
            receipt_out,
        } => {
            let gate_receipt_value = read_preserves_file(&gate_receipt)?;
            let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
                consumer,
                subject_ref: subject,
                gate_receipt_value: Some(gate_receipt_value),
                source_scope,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet source gate validation", &validation.value)?;
            if validation.decision != "pass" {
                return Err(MoltenError::invalid_harness(format!(
                    "octet source gate validation denied receipt={}",
                    validation.validation_ref
                )));
            }
            println!("octet source gate validation pass receipt={}", validation.validation_ref);
            Ok(())
        }
    }
}

fn run_octet_artifacts_command(command: OctetArtifactsCommand) -> Result<()> {
    match command {
        OctetArtifactsCommand::Import {
            artifacts,
            ledger,
            receipt_out,
        } => {
            let imported = octet_gate::import_octet_artifacts_to_ledger(&octet_gate::OctetArtifactLedgerInput {
                artifacts_dir: artifacts,
                ledger_root: ledger.clone(),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet artifact ledger receipt", &imported.receipt_value)?;
            println!(
                "octet artifacts import decision={} receipt={} imported={} ledger={}",
                imported.decision,
                imported.receipt_ref,
                imported.imported_refs.len(),
                ledger.display()
            );
            Ok(())
        }
    }
}

fn run_octet_review_command(command: OctetReviewCommand) -> Result<()> {
    match command {
        OctetReviewCommand::Write {
            out,
            profile,
            expires_at,
            finding_keys,
            rationale,
        } => {
            let review = octet_gate::build_octet_review_manifest(&octet_gate::OctetReviewManifestInput {
                profile,
                expires_at,
                finding_keys,
                rationale,
            })?;
            write_file(&out, &to_text(&review.review_value)?)?;
            println!("octet review manifest {} written to {}", review.review_ref, out.display());
            Ok(())
        }
    }
}

fn run_octet_baseline_command(command: OctetBaselineCommand) -> Result<()> {
    match command {
        OctetBaselineCommand::Write {
            artifacts,
            out,
            created_at,
            expires_at,
            target_next,
        } => {
            let baseline = octet_gate::build_octet_warning_baseline(&octet_gate::OctetWarningBaselineInput {
                artifacts_dir: artifacts,
                created_at,
                expires_at,
                target_next,
            })?;
            write_file(&out, &to_text(&baseline.baseline_value)?)?;
            println!(
                "octet warning baseline {} written to {} findings={} critical={}",
                baseline.baseline_ref,
                out.display(),
                baseline.finding_count,
                baseline.critical_count
            );
            Ok(())
        }
        OctetBaselineCommand::Check {
            artifacts,
            baseline,
            profile,
            as_of,
            receipt_out,
            reviews,
        } => {
            let baseline_value = read_preserves_file(&baseline)?;
            let review_values = reviews.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            let evaluation = octet_gate::check_octet_warning_baseline(&octet_gate::OctetBaselineCheckInput {
                artifacts_dir: artifacts.clone(),
                baseline_value,
                profile,
                as_of,
                review_values,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "octet baseline receipt", &evaluation.receipt_value)?;
            if evaluation.decision != "pass" {
                return Err(MoltenError::invalid_harness(format!(
                    "octet baseline denied receipt={} artifacts={}",
                    evaluation.receipt_ref,
                    artifacts.display()
                )));
            }
            println!("octet baseline pass receipt={}", evaluation.receipt_ref);
            Ok(())
        }
    }
}

fn run_repro_command(command: ReproCommand) -> Result<()> {
    match command {
        ReproCommand::Export {
            report,
            out,
            profile,
            failure_out,
        } => {
            let artifact_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "export")?;
            let export_profile = match ReproExportProfile::parse(&profile) {
                Ok(profile) => profile,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &artifact_value)?;
                    return Err(error);
                }
            };
            let command = vec![
                "molten".to_string(),
                "test".to_string(),
                "repro".to_string(),
                "export".to_string(),
                report.display().to_string(),
                "--out".to_string(),
                out.display().to_string(),
                "--profile".to_string(),
                profile,
            ];
            if parse_failure(&artifact_value).is_ok() {
                export_failure_repro(&artifact_value, &out, &command, failure_out.as_ref())
            } else {
                export_report_repro(&artifact_value, &out, &command, export_profile, failure_out.as_ref())
            }
        }
        ReproCommand::Verify {
            bundle,
            failure_out,
            receipt_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "verify")?;
            let receipt = match repro_verify_receipt_value(&bundle_value) {
                Ok(receipt) => receipt,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "verify", &error, &bundle_value)?;
                    return Err(error);
                }
            };
            if let Err(error) = emit_repro_verify_receipt(receipt_out.as_ref(), &receipt) {
                write_optional_artifact_failure(failure_out.as_ref(), "export", &error, &bundle_value)?;
                return Err(error);
            }
            Ok(())
        }
        ReproCommand::Unpack {
            bundle,
            out,
            reveal_receipts,
            failure_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "unpack")?;
            let reveal_receipt_values =
                reveal_receipts.iter().map(|path| read_preserves_file(path)).collect::<Result<Vec<_>>>()?;
            unpack_report_repro(&bundle_value, &out, &reveal_receipt_values, failure_out.as_ref())
        }
        ReproCommand::Publish {
            bundle,
            store,
            node,
            receipt_out,
            failure_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "publish")?;
            let published = match publish_bundle(&store, &bundle_value, &node) {
                Ok(published) => published,
                Err(error) => {
                    write_optional_artifact_failure(failure_out.as_ref(), "publish", &error, &bundle_value)?;
                    return Err(error);
                }
            };
            emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &published.receipt_value)?;
            println!("repro publish ok ticket={} bundle={}", published.ticket, published.bundle_ref);
            Ok(())
        }
        ReproCommand::Fetch {
            ticket,
            store,
            out,
            ledger,
            expected_bundle_ref,
            peer,
            receipt_out,
            failure_out,
        } => {
            let fetched = match fetch_bundle(&FetchBundleInput {
                root: &store,
                ticket: &ticket,
                expected_bundle_ref: expected_bundle_ref.as_deref(),
                peer: &peer,
                out: out.as_deref(),
                ledger_root: ledger.as_deref(),
            }) {
                Ok(fetched) => fetched,
                Err(error) => {
                    write_optional_failure(failure_out.as_ref(), "fetch", &error, None)?;
                    return Err(error);
                }
            };
            emit_named_receipt(receipt_out.as_ref(), "iroh repro exchange receipt", &fetched.receipt_value)?;
            println!("repro fetch ok ticket={} bundle={}", fetched.ticket, fetched.bundle_ref);
            Ok(())
        }
    }
}

fn export_report_repro(
    report_value: &preserves::IOValue,
    out: &Path,
    command: &[String],
    profile: ReproExportProfile,
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle_value = match repro_bundle_value_with_export_profile(report_value, command, profile) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let bundle = match parse_repro_bundle(&bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let exported_report_value = bundle.report_value.as_ref().unwrap_or(report_value);
    let suite_value = match report_suite_value(exported_report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, exported_report_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("report.preserves"), &to_text(exported_report_value)?)?;
        write_file(&out.join("suite.preserves"), &to_text(&suite_value)?)?;
        write_file(&out.join("summary.txt"), &report_summary(exported_report_value)?)?;
        write_file(&out.join("commands.txt"), REPORT_REPRO_COMMANDS)?;
        if let Some(gate_receipt_value) = bundle.gate_receipt_value.as_ref() {
            write_file(&out.join("gate-receipt.preserves"), &to_text(gate_receipt_value)?)?;
        }
        if let Some(value) = bundle.export_profile_value.as_ref() {
            write_file(&out.join("export-profile.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_manifest_value.as_ref() {
            write_file(&out.join("redaction-transform-manifest.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_receipt_value.as_ref() {
            write_file(&out.join("redaction-transform-receipt.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.private_bundle_profile_value.as_ref() {
            write_file(&out.join("private-bundle-profile.preserves"), &to_text(value)?)?;
        }
        write_file(&out.join("refs.preserves"), &to_text(&bundle_value)?)?;
        Ok(())
    })();
    if let Err(error) = export {
        write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
        return Err(error);
    }
    println!("repro bundle written to {}", out.display());
    Ok(())
}

fn unpack_report_repro(
    bundle_value: &preserves::IOValue,
    out: &Path,
    reveal_receipt_values: &[preserves::IOValue],
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle = match parse_repro_bundle(bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    if bundle.loss_classification.as_deref() == Some("requires-reveal") {
        if let Err(error) = validate_repro_reveal_receipts(&bundle.encrypted_refs, reveal_receipt_values) {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    } else if !reveal_receipt_values.is_empty() {
        let error =
            MoltenError::invalid_harness("reveal receipts are only accepted for encrypted-private repro bundles");
        write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
        return Err(error);
    }
    let verify_receipt = if bundle.loss_classification.as_deref().unwrap_or("gate-preserving") == "gate-preserving" {
        match repro_verify_receipt_value(bundle_value) {
            Ok(receipt) => Some(receipt),
            Err(error) => {
                write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
                return Err(error);
            }
        }
    } else {
        None
    };
    let report_value = match bundle.report_value.as_ref() {
        Some(report_value) => report_value,
        None => {
            let error = MoltenError::invalid_harness("repro unpack requires an embedded report");
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let suite_value = match report_suite_value(report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("refs.preserves"), &to_text(bundle_value)?)?;
        write_file(&out.join("report.preserves"), &to_text(report_value)?)?;
        write_file(&out.join("suite.preserves"), &to_text(&suite_value)?)?;
        if let Some(gate_receipt_value) = bundle.gate_receipt_value.as_ref() {
            write_file(&out.join("gate-receipt.preserves"), &to_text(gate_receipt_value)?)?;
        }
        if let Some(verify_receipt) = verify_receipt.as_ref() {
            write_file(&out.join("verify-receipt.preserves"), &to_text(verify_receipt)?)?;
        }
        if let Some(value) = bundle.redaction_transform_receipt_value.as_ref() {
            write_file(&out.join("redaction-transform-receipt.preserves"), &to_text(value)?)?;
        }
        if let Some(value) = bundle.redaction_transform_manifest_value.as_ref() {
            write_file(&out.join("redaction-transform-manifest.preserves"), &to_text(value)?)?;
        }
        for (index, receipt) in reveal_receipt_values.iter().enumerate() {
            write_file(&out.join(format!("reveal-receipt-{index}.preserves")), &to_text(receipt)?)?;
        }
        write_file(&out.join("summary.txt"), &repro_bundle_summary(bundle_value)?)?;
        write_file(&out.join("commands.txt"), REPORT_REPRO_COMMANDS)?;
        Ok(())
    })();
    if let Err(error) = export {
        write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
        return Err(error);
    }
    println!("repro bundle unpacked to {}", out.display());
    Ok(())
}

fn validate_repro_reveal_receipts(encrypted_refs: &[String], receipt_values: &[preserves::IOValue]) -> Result<()> {
    if encrypted_refs.is_empty() {
        return Err(MoltenError::invalid_harness("encrypted-private repro bundle has no encrypted refs to reveal"));
    }
    if receipt_values.is_empty() {
        return Err(MoltenError::invalid_harness(
            "encrypted-private repro unpack requires at least one passing reveal receipt",
        ));
    }
    let expected_refs = encrypted_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let mut authorized_refs = std::collections::BTreeSet::new();
    for receipt_value in receipt_values {
        let receipt = secrets::parse_reveal_receipt(receipt_value)?;
        if receipt.decision != "pass" {
            return Err(MoltenError::invalid_harness(
                "unauthorized reveal receipt cannot unpack private repro material",
            ));
        }
        let encrypted_ref = receipt
            .encrypted_ref
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("reveal receipt does not bind an encrypted repro reference"))?;
        if !expected_refs.contains(encrypted_ref) {
            return Err(MoltenError::invalid_harness("reveal receipt encrypted ref is not part of this repro bundle"));
        }
        authorized_refs.insert(encrypted_ref.clone());
    }
    for encrypted_ref in encrypted_refs {
        if !authorized_refs.contains(encrypted_ref) {
            return Err(MoltenError::invalid_harness(
                "reveal receipts do not authorize every encrypted repro reference",
            ));
        }
    }
    Ok(())
}

fn export_failure_repro(
    failure_value: &preserves::IOValue,
    out: &Path,
    command: &[String],
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle_value = match failure_repro_bundle_value_with_command(failure_value, command) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("failure.preserves"), &to_text(failure_value)?)?;
        write_file(&out.join("summary.txt"), &failure_summary(failure_value)?)?;
        write_file(&out.join("commands.txt"), FAILURE_REPRO_COMMANDS)?;
        write_file(&out.join("refs.preserves"), &to_text(&bundle_value)?)?;
        Ok(())
    })();
    if let Err(error) = export {
        write_optional_artifact_failure(failure_out, "export", &error, failure_value)?;
        return Err(error);
    }
    println!("failure repro bundle written to {}", out.display());
    Ok(())
}

const REPORT_REPRO_COMMANDS: &str = "molten test repro verify refs.preserves\nmolten test report validate report.preserves\nmolten test replay report.preserves\nmolten test report show report.preserves\nmolten test gate check refs.preserves\nmolten test repro unpack refs.preserves --out unpacked\n";
const FAILURE_REPRO_COMMANDS: &str =
    "molten test report show failure.preserves\nmolten test gate check refs.preserves\n";

fn write_release_export_archive(
    output_path: &Path,
    archive_path: &Path,
    manifest: &operator_dogfood::ReleaseExportManifest,
) -> Result<()> {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    let archive_file = fs::File::create(archive_path).map_err(MoltenError::from)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 19).map_err(MoltenError::from)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        to_text(&manifest.value)?.as_bytes(),
    )?;
    for (name, expected_ref) in &manifest.member_refs {
        let bytes = fs::read(output_path.join(name)).map_err(MoltenError::from)?;
        let actual_ref = operator_dogfood::release_export_file_ref(name, &bytes);
        if actual_ref != *expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "release export member {name} ref changed before archive write: manifest={expected_ref} observed={actual_ref}"
            )));
        }
        append_release_export_bytes(&mut builder, name, &bytes)?;
    }
    let encoder = builder.into_inner().map_err(MoltenError::from)?;
    encoder.finish().map_err(MoltenError::from)?;
    Ok(())
}

fn append_release_export_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o444);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder.append_data(&mut header, name, std::io::Cursor::new(bytes)).map_err(MoltenError::from)
}

#[derive(Debug)]
struct ReleaseExportArchiveRead {
    manifest_value: Option<preserves::IOValue>,
    member_refs: Vec<(String, String)>,
    diagnostics: Vec<String>,
}

fn read_release_export_archive(path: &Path) -> Result<ReleaseExportArchiveRead> {
    let archive_file = fs::File::open(path).map_err(MoltenError::from)?;
    let decoder = zstd::stream::read::Decoder::new(archive_file).map_err(MoltenError::from)?;
    let mut archive = tar::Archive::new(decoder);
    let mut manifest_value = None;
    let mut seen_names = Vec::with_capacity(operator_dogfood::release_export_member_names().len().saturating_add(16));
    let mut member_refs = Vec::with_capacity(operator_dogfood::release_export_member_names().len().saturating_add(16));
    let mut diagnostics = Vec::with_capacity(8);
    let entries = archive.entries().map_err(MoltenError::from)?;
    for entry in entries {
        let mut entry = entry.map_err(MoltenError::from)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry.path().map_err(MoltenError::from)?.to_string_lossy().replace('\\', "/");
        if seen_names.iter().any(|seen| seen == &name) {
            diagnostics.push(format!("duplicate release export archive member: {name}"));
        }
        seen_names.push(name.clone());
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).map_err(MoltenError::from)?;
        if name == "release-export-manifest.preserves" {
            if manifest_value.is_some() {
                diagnostics.push("duplicate release export manifest member".to_string());
            }
            let text = String::from_utf8(bytes).map_err(|error| {
                MoltenError::invalid_harness(format!("release export manifest is not UTF-8: {error}"))
            })?;
            manifest_value = Some(parse_text(&text)?);
        } else {
            member_refs.push((name.clone(), operator_dogfood::release_export_file_ref(&name, &bytes)));
        }
    }
    member_refs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(ReleaseExportArchiveRead {
        manifest_value,
        member_refs,
        diagnostics,
    })
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn read_preserves_files(paths: &[PathBuf]) -> Result<Vec<preserves::IOValue>> {
    let mut values = CliBoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "Preserves input files");
    for path in paths {
        values.push(read_preserves_file(path)?)?;
    }
    Ok(values.into_vec())
}

fn values_canonical_refs(values: &[preserves::IOValue]) -> Result<Vec<String>> {
    let mut refs = CliBoundedItems::new(PROVENANCE_CLI_EVIDENCE_LIMIT, "Preserves input refs");
    for value in values {
        refs.push(canonical_hash(value)?)?;
    }
    Ok(refs.into_vec())
}

fn read_preserves_file_with_failure(
    path: &Path,
    failure_out: Option<&PathBuf>,
    phase: &'static str,
) -> Result<preserves::IOValue> {
    let text = match fs::read_to_string(path).map_err(MoltenError::from) {
        Ok(text) => text,
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            return Err(error);
        }
    };
    match parse_text(&text) {
        Ok(value) => Ok(value),
        Err(error) => {
            write_optional_failure(failure_out, phase, &error, None)?;
            Err(error)
        }
    }
}

fn run_failure_phase(error: &MoltenError) -> &'static str {
    match error {
        MoltenError::InvalidHarness(_) => "preflight",
        MoltenError::Io(_) | MoltenError::Preserves(_) | MoltenError::HarnessDivergence(_) => "execute",
    }
}

fn write_optional_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    diagnostics: Option<Vec<preserves::IOValue>>,
) -> Result<()> {
    let failure = failure_value(phase, error, diagnostics.unwrap_or_default());
    emit_failure(path, &failure)
}

fn write_optional_suite_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    suite_value: &preserves::IOValue,
) -> Result<()> {
    let failure = suite_failure_value(phase, error, suite_value)?;
    emit_failure(path, &failure)
}

fn write_optional_report_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    report_value: &preserves::IOValue,
) -> Result<()> {
    let failure = report_failure_value(phase, error, report_value)?;
    emit_failure(path, &failure)
}

fn write_optional_artifact_failure(
    path: Option<&PathBuf>,
    phase: &'static str,
    error: &MoltenError,
    artifact_value: &preserves::IOValue,
) -> Result<()> {
    let artifact_ref = molten::preserves_rail::canonical_hash(artifact_value)?;
    write_optional_failure(
        path,
        phase,
        error,
        Some(vec![
            molten::preserves_rail::record("artifact-ref", vec![molten::preserves_rail::string(&artifact_ref)]),
            molten::preserves_rail::record("artifact", vec![artifact_value.clone()]),
        ]),
    )
}

fn emit_gate_receipt(path: Option<&PathBuf>, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("gate receipt {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("gate receipt {receipt_ref}");
    }
    Ok(())
}

fn emit_repro_verify_receipt(path: Option<&PathBuf>, receipt: &preserves::IOValue) -> Result<()> {
    emit_named_receipt(path, "repro verify receipt", receipt)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn emit_failure(path: Option<&PathBuf>, failure: &preserves::IOValue) -> Result<()> {
    let failure_text = to_text(failure)?;
    let failure_ref = molten::preserves_rail::canonical_hash(failure)?;
    if let Some(path) = path {
        write_file(path, &failure_text)?;
        eprintln!("failure {failure_ref} written to {}", path.display());
    } else {
        println!("{failure_text}");
        eprintln!("failure {failure_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use molten::authority;
    use molten::harness::parse_repro_bundle;

    use super::*;

    #[test]
    fn cli_run_writes_canonical_failure_file() {
        let dir = temp_dir("run-failure");
        let suite = dir.join("bad.preserves");
        let failure = dir.join("failure.preserves");
        write_file(
            &suite,
            r#"<harness-suite-v1 "molten.harness.suite.v1" "bad" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              [<send "producer" "missing" "hello">]>"#,
        )
        .expect("write suite");

        let error = run_test_command(TestCommand::Run {
            suite,
            report_out: Some(failure.clone()),
        })
        .expect_err("run should fail");
        assert!(error.to_string().contains("unknown actor missing"));
        let failure_value = read_preserves_file(&failure).expect("read failure");
        let failure = parse_failure(&failure_value).expect("parse failure");
        assert_eq!(failure.phase, "preflight");
        assert_eq!(failure.kind, "invalid-harness");
    }

    #[test]
    fn cli_gate_rejects_failure_artifact_with_canonical_failure() {
        let dir = temp_dir("gate-failure");
        let failure_artifact = dir.join("input.failure.preserves");
        let gate_failure = dir.join("gate.failure.preserves");
        let synthetic = failure_value("preflight", &MoltenError::invalid_harness("synthetic"), Vec::new());
        write_file(&failure_artifact, &to_text(&synthetic).expect("render failure")).expect("write failure");

        let error = run_gate_command(GateCommand::Check {
            artifact: failure_artifact,
            failure_out: Some(gate_failure.clone()),
            receipt_out: None,
        })
        .expect_err("gate should reject failure evidence");
        assert!(error.to_string().contains("cannot satisfy pass evidence gate"));
        let failure_value = read_preserves_file(&gate_failure).expect("read gate failure");
        let failure = parse_failure(&failure_value).expect("parse gate failure");
        assert_eq!(failure.phase, "validate");
        assert_eq!(failure.kind, "invalid-harness");
    }

    #[test]
    fn cli_repro_export_accepts_failure_artifact() {
        let dir = temp_dir("failure-repro");
        let failure_artifact = dir.join("input.failure.preserves");
        let out = dir.join("bundle");
        let synthetic = failure_value("execute", &MoltenError::invalid_harness("synthetic"), Vec::new());
        write_file(&failure_artifact, &to_text(&synthetic).expect("render failure")).expect("write failure");

        run_repro_command(ReproCommand::Export {
            report: failure_artifact,
            out: out.clone(),
            profile: "deny-sensitive".to_string(),
            failure_out: None,
        })
        .expect("export failure repro");
        let bundle = read_preserves_file(&out.join("refs.preserves")).expect("read refs");
        let parsed = parse_repro_bundle(&bundle).expect("parse bundle");
        assert_eq!(parsed.kind, molten::harness::HarnessReproBundleKind::Failure);
        assert!(out.join("failure.preserves").exists());
        assert!(out.join("commands.txt").exists());

        let verify_failure = dir.join("verify.failure.preserves");
        let verify_error = run_repro_command(ReproCommand::Verify {
            bundle: out.join("refs.preserves"),
            failure_out: Some(verify_failure.clone()),
            receipt_out: None,
        })
        .expect_err("failure repro verify should fail");
        assert!(verify_error.to_string().contains("diagnostic-only"));
        let verify_failure_value = read_preserves_file(&verify_failure).expect("read verify failure");
        let verify_failure = parse_failure(&verify_failure_value).expect("parse verify failure");
        assert_eq!(verify_failure.phase, "verify");

        let unpack_failure = dir.join("unpack.failure.preserves");
        let unpack_error = run_repro_command(ReproCommand::Unpack {
            bundle: out.join("refs.preserves"),
            out: dir.join("unpacked-failure"),
            reveal_receipts: Vec::new(),
            failure_out: Some(unpack_failure.clone()),
        })
        .expect_err("failure repro unpack should fail");
        let unpack_error_message = unpack_error.to_string();
        let expected_unpack_messages = ["diagnostic-only", "embedded report"];
        assert!(
            expected_unpack_messages.iter().any(|message| unpack_error_message.contains(message)),
            "unexpected unpack error: {unpack_error_message}"
        );
        let unpack_failure_value = read_preserves_file(&unpack_failure).expect("read unpack failure");
        let unpack_failure = parse_failure(&unpack_failure_value).expect("parse unpack failure");
        assert_eq!(unpack_failure.phase, "unpack");
    }

    #[test]
    fn cli_chunk_store_commands_work() {
        let dir = temp_dir("chunk-cli");
        let input = dir.join("input.bin");
        let store = dir.join("chunk-store");
        let manifest = dir.join("manifest.preserves");
        let full = dir.join("full.bin");
        let range = dir.join("range.bin");
        fs::write(&input, b"aaaabbbbcccc").expect("write input");
        run_chunk_command(ChunkCommand::Put {
            input,
            store: store.clone(),
            kind: "artifact".to_string(),
            chunk_size: 4,
            manifest_out: Some(manifest.clone()),
            receipt_out: Some(dir.join("put-receipt.preserves")),
        })
        .expect("chunk put");
        let manifest_value = read_preserves_file(&manifest).expect("read manifest");
        let manifest_ref = molten::preserves_rail::canonical_hash(&manifest_value).expect("manifest ref");
        run_chunk_command(ChunkCommand::Verify {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            receipt_out: Some(dir.join("verify-receipt.preserves")),
        })
        .expect("chunk verify");
        run_chunk_command(ChunkCommand::Read {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            out: full.clone(),
            receipt_out: Some(dir.join("read-receipt.preserves")),
        })
        .expect("chunk read");
        assert_eq!(fs::read(&full).expect("read full"), b"aaaabbbbcccc");
        run_chunk_command(ChunkCommand::Range {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            offset: 2,
            length: 8,
            out: range.clone(),
            receipt_out: Some(dir.join("range-receipt.preserves")),
        })
        .expect("chunk range");
        assert_eq!(fs::read(&range).expect("read range"), b"aabbbbcc");
        let mirror = dir.join("chunk-store-mirror");
        run_chunk_command(ChunkCommand::Sync {
            manifest_ref: manifest_ref.clone(),
            from: store.clone(),
            store: mirror.clone(),
            receipt_out: Some(dir.join("sync-receipt.preserves")),
        })
        .expect("chunk sync");
        run_chunk_command(ChunkCommand::Read {
            manifest_ref: manifest_ref.clone(),
            store: mirror,
            out: dir.join("mirror-full.bin"),
            receipt_out: None,
        })
        .expect("read synced chunk store");
        let iroh_store = dir.join("chunk-iroh-store");
        run_chunk_command(ChunkCommand::IrohPublish {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            iroh_store: iroh_store.clone(),
            node: "node:cli".to_string(),
            receipt_out: Some(dir.join("iroh-publish-receipt.preserves")),
        })
        .expect("chunk iroh publish");
        let iroh_dest = dir.join("chunk-iroh-dest");
        run_chunk_command(ChunkCommand::IrohFetch {
            ticket: format!("iroh-local-chunk:{manifest_ref}"),
            iroh_store: iroh_store.clone(),
            store: iroh_dest.clone(),
            expected_manifest_ref: Some(manifest_ref.clone()),
            peer: "peer:cli".to_string(),
            receipt_out: Some(dir.join("iroh-fetch-receipt.preserves")),
        })
        .expect("chunk iroh fetch");
        run_chunk_command(ChunkCommand::Read {
            manifest_ref: manifest_ref.clone(),
            store: iroh_dest,
            out: dir.join("iroh-full.bin"),
            receipt_out: None,
        })
        .expect("read iroh-fetched chunk store");
        run_chunk_command(ChunkCommand::IndexStatus { store: store.clone() }).expect("chunk index status");
        run_chunk_command(ChunkCommand::IndexRebuild {
            store: store.clone(),
            receipt_out: Some(dir.join("index-rebuild-receipt.preserves")),
        })
        .expect("chunk index rebuild");
        run_chunk_command(ChunkCommand::ReceiptList { store: store.clone() }).expect("chunk receipt list");
        let receipt_ref = chunk_store::list_receipt_refs(&store)
            .expect("list receipt refs")
            .into_iter()
            .next()
            .expect("receipt ref");
        run_chunk_command(ChunkCommand::ReceiptShow {
            receipt_ref,
            store: store.clone(),
        })
        .expect("chunk receipt show");
        let lineage_out = dir.join("chunk-lineage.preserves");
        run_chunk_command(ChunkCommand::Lineage {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            lineage_out: Some(lineage_out.clone()),
        })
        .expect("chunk lineage");
        assert!(fs::read_to_string(lineage_out).expect("read lineage").contains("chunk-lineage-v1"));
        run_chunk_command(ChunkCommand::Pin {
            manifest_ref: manifest_ref.clone(),
            store: store.clone(),
            receipt_out: Some(dir.join("pin-receipt.preserves")),
        })
        .expect("chunk pin");
        run_chunk_command(ChunkCommand::Unpin {
            manifest_ref,
            store: store.clone(),
            receipt_out: Some(dir.join("unpin-receipt.preserves")),
        })
        .expect("chunk unpin");
        run_chunk_command(ChunkCommand::Gc {
            store,
            dry_run: false,
            apply_refs: Vec::new(),
            retention: retention_cli_args("chunk-gc"),
            receipt_out: Some(dir.join("gc-receipt.preserves")),
        })
        .expect("chunk gc");
    }

    #[test]
    fn cli_transcript_commands_work() {
        let dir = temp_dir("transcript-cli");
        let markdown = dir.join("example.md");
        let transcript_out = dir.join("transcript.preserves");
        let run_receipt = dir.join("transcript-run-receipt.preserves");
        let rendered = dir.join("rendered.md");
        write_file(
            &markdown,
            "```preserves:hide\n<value \"cli\">\n```\n```expect\n<expect-output <value \"cli\">>\n```\n",
        )
        .expect("write transcript markdown");
        run_transcript_command(TranscriptCommand::Parse {
            markdown: markdown.clone(),
            out: transcript_out.clone(),
            dependency_refs: Vec::new(),
            dependency_closure_hash: None,
            handler_profile_ref: None,
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            revocation_refs: Vec::new(),
            seed_ref: None,
            expected_refs: Vec::new(),
        })
        .expect("transcript parse");
        run_transcript_command(TranscriptCommand::Run {
            transcript: transcript_out.clone(),
            cache: Some(dir.join("transcript-cache")),
            state: "fresh".to_string(),
            save_root: None,
            out: Some(rendered.clone()),
            receipt_out: Some(run_receipt.clone()),
            failure_out: Some(dir.join("transcript.failure.preserves")),
        })
        .expect("transcript run");
        assert!(fs::read_to_string(&rendered).expect("read rendered").contains("output hidden"));
        run_transcript_command(TranscriptCommand::Show {
            transcript: transcript_out.clone(),
        })
        .expect("transcript show");
        run_transcript_command(TranscriptCommand::Render {
            transcript: transcript_out,
            receipt: Some(run_receipt),
            out: dir.join("rendered-again.md"),
        })
        .expect("transcript render");
    }

    #[test]
    fn cli_eval_cache_commands_work() {
        let dir = temp_dir("cache-cli");
        let cache = dir.join("eval-cache");
        let input = dir.join("input.preserves");
        let output = dir.join("output.preserves");
        let key_out = dir.join("key.preserves");
        let value_out = dir.join("value.preserves");
        let hit_out = dir.join("hit.preserves");
        let dependency_ref = test_ref("cache-cli-dependency");
        let policy_ref = test_ref("cache-cli-policy");
        write_file(&input, "<schema-shape <record \"x\">>").expect("write cache input");
        write_file(&output, "<fingerprint \"ok\">").expect("write cache output");
        run_cache_command(CacheCommand::Put {
            input,
            cache: cache.clone(),
            output: Some(output),
            operation: "schema-fingerprint".to_string(),
            version: "v1".to_string(),
            dependencies: vec![dependency_ref.clone()],
            dependency_closure_hash: None,
            handler_profile_ref: None,
            policy_refs: vec![policy_ref.clone()],
            capability_refs: Vec::new(),
            revocation_refs: Vec::new(),
            tool_ref: None,
            tool_version: "cli-test".to_string(),
            assumption_refs: Vec::new(),
            tier: eval_cache::TIER_PURE.to_string(),
            status: eval_cache::STATUS_PASS.to_string(),
            evidence_refs: Vec::new(),
            diagnostics: Vec::new(),
            key_out: Some(key_out.clone()),
            value_out: Some(value_out.clone()),
            receipt_out: Some(dir.join("put-receipt.preserves")),
        })
        .expect("cache put");
        let key = eval_cache::parse_eval_cache_key(&read_preserves_file(&key_out).expect("read key"))
            .expect("parse cache key");
        run_cache_command(CacheCommand::Get {
            key_ref: key.key_ref.clone(),
            cache: cache.clone(),
            current_policy_refs: Vec::new(),
            current_capability_refs: Vec::new(),
            current_revocation_refs: Vec::new(),
            semantic_enabled: true,
            out: Some(hit_out.clone()),
            receipt_out: Some(dir.join("hit-receipt.preserves")),
        })
        .expect("cache get");
        assert_eq!(fs::read_to_string(&hit_out).expect("read hit"), "<fingerprint \"ok\">");
        run_cache_command(CacheCommand::Status { cache: cache.clone() }).expect("cache status");
        run_cache_command(CacheCommand::List {
            cache: cache.clone(),
            operation: Some("schema-fingerprint".to_string()),
            tier: Some(eval_cache::TIER_PURE.to_string()),
            status: Some(eval_cache::STATUS_PASS.to_string()),
            dependency_ref: Some(dependency_ref.clone()),
            policy_ref: Some(policy_ref),
            capability_ref: None,
            revocation_ref: None,
            evidence_ref: None,
        })
        .expect("cache list");
        run_cache_command(CacheCommand::Show {
            reference: key.key_ref.clone(),
            cache: cache.clone(),
        })
        .expect("cache show key");
        run_cache_command(CacheCommand::Show {
            reference: eval_cache::parse_eval_cache_value(&read_preserves_file(&value_out).expect("read value"))
                .expect("parse cache value")
                .value_ref,
            cache: cache.clone(),
        })
        .expect("cache show value");
        let cache_retention_object = RetentionCliObject {
            root: &cache,
            label: "cache-invalidate",
            object_ref: &key.key_ref,
            object_kind: "eval-cache-key",
            retention_class: retention::CLASS_EPHEMERAL_CACHE,
            action: retention::ACTION_TOMBSTONE,
        };
        let cache_retention = retention_cli_args_for_object(cache_retention_object);
        let cache_apply_refs = vec![retention_apply_ref(
            cache_retention_object,
            "eval-cache-invalidate",
            &cache_retention,
        )];
        let invalidate_receipt = dir.join("invalidate-receipt.preserves");
        run_cache_command(CacheCommand::Invalidate {
            cache: cache.clone(),
            key_ref: None,
            dependency_ref: Some(dependency_ref),
            policy_ref: None,
            capability_ref: None,
            revocation_ref: None,
            operation: None,
            reason: "cli-test".to_string(),
            apply_refs: cache_apply_refs,
            retention: cache_retention,
            receipt_out: Some(invalidate_receipt.clone()),
        })
        .expect("cache invalidate");
        let invalidate_text = fs::read_to_string(&invalidate_receipt).expect("read invalidate receipt");
        assert!(invalidate_text.contains("retention-execution"));
        let error = run_cache_command(CacheCommand::Get {
            key_ref: key.key_ref,
            cache,
            current_policy_refs: Vec::new(),
            current_capability_refs: Vec::new(),
            current_revocation_refs: Vec::new(),
            semantic_enabled: true,
            out: None,
            receipt_out: None,
        })
        .expect_err("invalidated key should miss");
        assert!(error.to_string().contains("tombstoned"), "{error}");
    }

    #[test]
    fn cli_typed_storage_commands_work() {
        let dir = temp_dir("storage-cli");
        let store = dir.join("typed-storage");
        let value_file = dir.join("value.preserves");
        let typed_ref_out = dir.join("typed-ref.preserves");
        let get_out = dir.join("get.preserves");
        write_file(&value_file, "<profile \"alice\" 7>").expect("write value");
        run_storage_command(StorageCommand::Put {
            value: value_file,
            store: store.clone(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: None,
            producer_ref: None,
            ref_out: Some(typed_ref_out.clone()),
            receipt_out: Some(dir.join("put-receipt.preserves")),
        })
        .expect("storage put");
        let typed_ref_value = read_preserves_file(&typed_ref_out).expect("read typed ref");
        let typed_ref = typed_storage::parse_typed_ref_value(&typed_ref_value).expect("parse typed ref");
        let source_schema_ref = typed_ref.schema_ref.clone();
        let source_storage_ref = typed_ref.storage_ref.clone();
        run_storage_command(StorageCommand::Get {
            store: store.clone(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(source_schema_ref.clone()),
            migration_recipe: None,
            out: Some(get_out.clone()),
            receipt_out: Some(dir.join("get-receipt.preserves")),
        })
        .expect("storage get");
        assert_eq!(fs::read_to_string(&get_out).expect("read get"), "<profile \"alice\" 7>");
        run_storage_command(StorageCommand::Verify {
            storage_ref: source_storage_ref,
            store: store.clone(),
            schema_ref: Some(source_schema_ref.clone()),
            receipt_out: Some(dir.join("verify-receipt.preserves")),
        })
        .expect("storage verify");
        let recipe_path = dir.join("migration-recipe.preserves");
        let target_schema_ref = test_ref("storage-cli-target-schema");
        run_storage_command(StorageCommand::Recipe {
            source_schema_ref,
            target_schema_ref: target_schema_ref.clone(),
            transformer_ref: test_ref("storage-cli-transformer"),
            transformer_kind: "schema-rename".to_string(),
            mode: "explicit".to_string(),
            out: recipe_path.clone(),
        })
        .expect("storage recipe");
        run_storage_command(StorageCommand::Migrate {
            recipe: recipe_path.clone(),
            store: store.clone(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            ref_out: Some(dir.join("migrated-ref.preserves")),
            receipt_out: Some(dir.join("migrate-receipt.preserves")),
        })
        .expect("storage migrate");
        run_storage_command(StorageCommand::Get {
            store: store.clone(),
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(target_schema_ref),
            migration_recipe: None,
            out: Some(dir.join("get-migrated.preserves")),
            receipt_out: Some(dir.join("get-migrated-receipt.preserves")),
        })
        .expect("storage get migrated");
        let error = run_storage_command(StorageCommand::Get {
            store,
            namespace: "profiles".to_string(),
            key: "alice".to_string(),
            schema_ref: Some(test_ref("wrong-schema")),
            migration_recipe: None,
            out: None,
            receipt_out: None,
        })
        .expect_err("wrong schema get denied");
        assert!(error.to_string().contains("schema ref"), "{error}");
    }

    #[test]
    fn cli_artifact_registry_commands_work() {
        let dir = temp_dir("artifact-cli");
        let registry = dir.join("registry");
        let base_payload = dir.join("base.preserves");
        let dep_payload = dir.join("dependent.preserves");
        let base_out = dir.join("base-artifact.preserves");
        let dep_out = dir.join("dependent-artifact.preserves");
        write_file(&base_payload, "<schema \"base\">").expect("write base payload");
        write_file(&dep_payload, "<module \"dependent\">").expect("write dep payload");
        run_artifact_command(ArtifactCommand::Install {
            payload: base_payload,
            registry: registry.clone(),
            kind: "schema".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(base_out.clone()),
            receipt_out: Some(dir.join("base-install-receipt.preserves")),
        })
        .expect("install base artifact");
        let base_value = read_preserves_file(&base_out).expect("read base artifact");
        let base = artifacts::parse_artifact_value(&base_value).expect("parse base artifact");
        run_artifact_command(ArtifactCommand::Install {
            payload: dep_payload,
            registry: registry.clone(),
            kind: "steel".to_string(),
            dependencies: vec![base.artifact_ref.clone()],
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(dep_out.clone()),
            receipt_out: Some(dir.join("dep-install-receipt.preserves")),
        })
        .expect("install dependent artifact");
        let dep_value = read_preserves_file(&dep_out).expect("read dependent artifact");
        let dep = artifacts::parse_artifact_value(&dep_value).expect("parse dependent artifact");
        run_artifact_command(ArtifactCommand::List {
            registry: registry.clone(),
            kind: None,
        })
        .expect("artifact list");
        run_artifact_command(ArtifactCommand::View {
            artifact_ref: dep.artifact_ref.clone(),
            registry: registry.clone(),
            payload: false,
        })
        .expect("artifact view envelope");
        run_artifact_command(ArtifactCommand::View {
            artifact_ref: dep.artifact_ref.clone(),
            registry: registry.clone(),
            payload: true,
        })
        .expect("artifact view payload");
        run_artifact_command(ArtifactCommand::NameSet {
            registry: registry.clone(),
            kind: "name".to_string(),
            name: "app/main".to_string(),
            artifact_ref: dep.artifact_ref.clone(),
            receipt_out: Some(dir.join("name-set-receipt.preserves")),
        })
        .expect("artifact name set");
        run_artifact_command(ArtifactCommand::NameShow {
            registry: registry.clone(),
            kind: "name".to_string(),
            name: "app/main".to_string(),
        })
        .expect("artifact name show");
        run_artifact_command(ArtifactCommand::Deps {
            artifact_ref: dep.artifact_ref.clone(),
            registry: registry.clone(),
        })
        .expect("artifact deps");
        run_artifact_command(ArtifactCommand::Closure {
            artifact_ref: dep.artifact_ref.clone(),
            registry: registry.clone(),
            receipt_out: Some(dir.join("closure-receipt.preserves")),
        })
        .expect("artifact closure");
        run_artifact_command(ArtifactCommand::Impact {
            artifact_ref: base.artifact_ref.clone(),
            registry: registry.clone(),
            receipt_out: Some(dir.join("impact-receipt.preserves")),
        })
        .expect("artifact impact");
        run_artifact_command(ArtifactCommand::IndexRebuild {
            registry,
            receipt_out: Some(dir.join("rebuild-receipt.preserves")),
        })
        .expect("artifact index rebuild");
    }

    #[test]
    fn cli_rewrite_commands_work() {
        let dir = temp_dir("rewrite-cli");
        let registry = dir.join("registry");
        let payload = dir.join("doc.preserves");
        let artifact_out = dir.join("doc-artifact.preserves");
        let matches_out = dir.join("rewrite-matches.preserves");
        let plan_out = dir.join("rewrite-plan.preserves");
        let apply_receipt = dir.join("rewrite-apply-receipt.preserves");
        let upgrade_plan = dir.join("rewrite-upgrade-plan.preserves");
        write_file(&payload, r#"<doc "old" ["old" "keep"]>"#).expect("write rewrite payload");
        run_artifact_command(ArtifactCommand::Install {
            payload,
            registry: registry.clone(),
            kind: "doc".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(artifact_out),
            receipt_out: Some(dir.join("doc-install-receipt.preserves")),
        })
        .expect("install rewrite artifact");
        run_rewrite_command(RewriteCommand::Find {
            registry: registry.clone(),
            pattern_kind: "string-equals".to_string(),
            pattern: "old".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            matches_out: Some(matches_out.clone()),
            receipt_out: Some(dir.join("rewrite-find-receipt.preserves")),
        })
        .expect("rewrite find");
        assert!(fs::read_to_string(&matches_out).expect("read matches").contains("rewrite-match-v1"));
        run_rewrite_command(RewriteCommand::Preview {
            registry: registry.clone(),
            from: "old".to_string(),
            to: "new".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            plan_out: Some(plan_out.clone()),
            receipt_out: Some(dir.join("rewrite-preview-receipt.preserves")),
        })
        .expect("rewrite preview");
        run_rewrite_command(RewriteCommand::Show {
            artifact: plan_out.clone(),
        })
        .expect("rewrite show plan");
        run_rewrite_command(RewriteCommand::Apply {
            registry: registry.clone(),
            from: "old".to_string(),
            to: "new".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            plan_out: None,
            receipt_out: Some(apply_receipt.clone()),
            upgrade_plan_out: Some(upgrade_plan.clone()),
            session_id: "rewrite-cli-session".to_string(),
        })
        .expect("rewrite apply");
        run_rewrite_command(RewriteCommand::Show {
            artifact: apply_receipt,
        })
        .expect("rewrite show receipt");
        assert!(fs::read_to_string(upgrade_plan).expect("read upgrade plan").contains("upgrade-plan-v1"));
        let docs = artifacts::list_artifacts(&registry, Some("doc")).expect("list rewritten docs");
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|artifact| {
            artifacts::read_payload(&registry, &artifact.artifact_ref)
                .and_then(|value| to_text(&value))
                .is_ok_and(|text| text.contains("new"))
        }));
    }

    #[test]
    fn cli_protocol_commands_work() {
        let dir = temp_dir("protocol-cli");
        let out = dir.join("request-response");
        run_protocol_command(ProtocolCommand::RunRequestResponse { out: out.clone() })
            .expect("run protocol request-response lifecycle");
        let receipt = out.join("install-receipt.preserves");
        assert!(receipt.exists());
        run_protocol_command(ProtocolCommand::Show {
            receipt: receipt.clone(),
        })
        .expect("show protocol install");
        let gate_receipt = dir.join("protocol-gate.preserves");
        run_protocol_command(ProtocolCommand::GateLifecycle {
            dir: out.clone(),
            receipt_out: Some(gate_receipt.clone()),
        })
        .expect("gate protocol lifecycle");
        let gate = protocol_session::parse_protocol_session_gate_receipt(
            &read_preserves_file(&gate_receipt).expect("read protocol gate receipt"),
        )
        .expect("parse protocol gate receipt");
        assert_eq!(gate.decision, "pass");
        let install_out = dir.join("install-only");
        run_protocol_command(ProtocolCommand::Install {
            manifest: out.join("manifest.preserves"),
            out: install_out.clone(),
        })
        .expect("install protocol manifest");
        assert!(read_preserves_file(&install_out.join("endpoints").join("endpoint-0.preserves")).is_ok());
    }

    #[test]
    fn cli_raft_commands_work() {
        let dir = temp_dir("raft-cli");
        let out = dir.join("fixture");
        run_raft_command(RaftCommand::RunFixture { out: out.clone() }).expect("run raft fixture");
        assert!(out.join("manifest.preserves").exists());
        assert!(out.join("state.preserves").exists());
        assert!(out.join("read-receipt.preserves").exists());
        assert!(out.join("snapshot.preserves").exists());
        run_raft_command(RaftCommand::Show {
            artifact: out.join("state.preserves"),
        })
        .expect("show raft state");
    }

    #[test]
    fn cli_delivery_idempotency_commands_work() {
        let dir = temp_dir("delivery-cli");
        let root = dir.join("store");
        let scope_out = dir.join("scope.preserves");
        let policy_ref = cli_synthetic_ref("delivery-policy").expect("policy ref");
        let evidence_ref = cli_synthetic_ref("delivery-evidence").expect("evidence ref");
        let payload_ref = cli_synthetic_ref("delivery-payload").expect("payload ref");
        let result_ref = cli_synthetic_ref("delivery-result").expect("result ref");
        run_delivery_command(DeliveryCommand::Scope {
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: "peer:b:services".to_string(),
            retention_refs: vec![policy_ref.clone()],
            out: Some(scope_out.clone()),
        })
        .expect("write delivery scope");
        assert!(scope_out.exists());
        let operation_out = dir.join("operation.preserves");
        run_delivery_command(DeliveryCommand::OperationId {
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: Some("peer:b:services".to_string()),
            scope_ref: None,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 1,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            out: Some(operation_out.clone()),
        })
        .expect("write operation id");
        run_delivery_command(DeliveryCommand::Show {
            artifact: operation_out.clone(),
        })
        .expect("show operation id");
        let first_receipt = dir.join("first.preserves");
        run_delivery_command(DeliveryCommand::Check {
            root: root.clone(),
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: Some("peer:b:services".to_string()),
            scope_ref: None,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 1,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            semantic_result_ref: Some(result_ref.clone()),
            gap_policy: "deny".to_string(),
            receipt_out: Some(first_receipt.clone()),
        })
        .expect("first delivery check");
        let first = delivery_idempotency::parse_idempotency_receipt(
            &read_preserves_file(&first_receipt).expect("read first receipt"),
        )
        .expect("parse first receipt");
        assert_eq!(first.decision, "first");
        run_delivery_command(DeliveryCommand::ReceiptShow {
            receipt_ref: first.receipt_ref.clone(),
            root: root.clone(),
        })
        .expect("show stored receipt");
        let duplicate_receipt = dir.join("duplicate.preserves");
        run_delivery_command(DeliveryCommand::Check {
            root: root.clone(),
            scope_profile: delivery_idempotency::SCOPE_REMOTE_TOPIC.to_string(),
            scope_name: Some("peer:b:services".to_string()),
            scope_ref: None,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 1,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            semantic_result_ref: Some(result_ref),
            gap_policy: "deny".to_string(),
            receipt_out: Some(duplicate_receipt.clone()),
        })
        .expect("duplicate delivery check");
        let duplicate = delivery_idempotency::parse_idempotency_receipt(
            &read_preserves_file(&duplicate_receipt).expect("read duplicate receipt"),
        )
        .expect("parse duplicate receipt");
        assert_eq!(duplicate.decision, "duplicate");
        assert_eq!(duplicate.prior_receipt_ref.as_deref(), Some(first.receipt_ref.as_str()));
    }

    #[test]
    fn cli_retention_commands_work() {
        let dir = temp_dir("retention-cli");
        let root = dir.join("store");
        let policy_ref = cli_synthetic_ref("retention-policy").expect("policy ref");
        let evidence_ref = cli_synthetic_ref("retention-evidence").expect("evidence ref");
        let authority_ref = cli_synthetic_ref("retention-authority").expect("authority ref");
        let owner_ref = cli_synthetic_ref("retention-owner").expect("owner ref");
        let object_ref = cli_synthetic_ref("retention-object").expect("object ref");
        let class_out = dir.join("class.preserves");
        run_retention_command(RetentionCommand::Class {
            class_name: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            minimum_age_seconds: 0,
            maximum_age_seconds: Some(3600),
            deletion_authority_ref: authority_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            has_secret_redaction_hook: true,
            has_remote_gc_plan: true,
            has_compaction: false,
            out: Some(class_out.clone()),
        })
        .expect("retention class");
        run_retention_command(RetentionCommand::Show {
            artifact: class_out.clone(),
        })
        .expect("show retention class");
        let admission_out = dir.join("authority-admission.preserves");
        run_retention_command(RetentionCommand::Admit {
            root: root.clone(),
            kind: retention::ADMISSION_KIND_AUTHORITY.to_string(),
            decision: "pass".to_string(),
            requester_ref: owner_ref.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            bound_refs: vec![authority_ref.clone()],
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            is_reference_index_complete: true,
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(admission_out.clone()),
        })
        .expect("retention admission");
        run_retention_command(RetentionCommand::Show {
            artifact: admission_out,
        })
        .expect("show retention admission");
        let clearance_out = dir.join("remote-clearance.preserves");
        let remote_ref = cli_synthetic_ref("retention-remote").expect("remote ref");
        let peer_ref = cli_synthetic_ref("retention-peer").expect("peer ref");
        run_retention_command(RetentionCommand::RemoteClearance {
            root: root.clone(),
            decision: "pass".to_string(),
            requester_ref: owner_ref.clone(),
            peer_ref: peer_ref.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            remote_ref: remote_ref.clone(),
            policy_ref: policy_ref.clone(),
            authority_ref: authority_ref.clone(),
            evidence_refs: vec![evidence_ref.clone()],
            retained_refs: Vec::new(),
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(clearance_out.clone()),
        })
        .expect("retention remote clearance");
        run_retention_command(RetentionCommand::Show {
            artifact: clearance_out,
        })
        .expect("show retention remote clearance");
        let request_out = dir.join("remote-clearance-request.preserves");
        run_retention_command(RetentionCommand::RemoteClearanceRequest {
            root: root.clone(),
            requester_ref: owner_ref.clone(),
            peer_ref: peer_ref.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            remote_ref: remote_ref.clone(),
            policy_ref: policy_ref.clone(),
            authority_ref: authority_ref.clone(),
            evidence_refs: vec![evidence_ref.clone()],
            out: Some(request_out.clone()),
        })
        .expect("retention remote clearance request");
        run_retention_command(RetentionCommand::Show {
            artifact: request_out.clone(),
        })
        .expect("show retention remote clearance request");
        let response_out = dir.join("remote-clearance-response.preserves");
        run_retention_command(RetentionCommand::RemoteClearanceRespond {
            root: root.clone(),
            request: request_out.clone(),
            evidence_refs: vec![cli_synthetic_ref("retention-peer-evidence").expect("peer evidence ref")],
            retained_refs: Vec::new(),
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(response_out.clone()),
        })
        .expect("retention remote clearance response");
        run_retention_command(RetentionCommand::Show {
            artifact: response_out.clone(),
        })
        .expect("show retention remote clearance response");
        let import_out = dir.join("remote-clearance-import.preserves");
        run_retention_command(RetentionCommand::RemoteClearanceImport {
            root: root.clone(),
            request: request_out.clone(),
            response: response_out.clone(),
            expected_peer_ref: Some(peer_ref.clone()),
            expected_remote_ref: Some(remote_ref.clone()),
            out: Some(import_out.clone()),
        })
        .expect("retention remote clearance import");
        run_retention_command(RetentionCommand::Show {
            artifact: import_out.clone(),
        })
        .expect("show retention remote clearance import");
        let import = retention::parse_retention_remote_gc_clearance_import(
            &read_preserves_file(&import_out).expect("read clearance import"),
        )
        .expect("parse clearance import");
        assert_eq!(import.decision, "pass");
        assert!(import.clearance_ref.is_some());
        let retained_response_out = dir.join("remote-clearance-retained-response.preserves");
        run_retention_command(RetentionCommand::RemoteClearanceRespond {
            root: root.clone(),
            request: request_out.clone(),
            evidence_refs: Vec::new(),
            retained_refs: vec![cli_synthetic_ref("retention-remote-retained").expect("remote retained ref")],
            is_stale: false,
            revoked_refs: Vec::new(),
            diagnostics: Vec::new(),
            out: Some(retained_response_out.clone()),
        })
        .expect("retention retained remote clearance response");
        let retained_import_out = dir.join("remote-clearance-retained-import.preserves");
        run_retention_command(RetentionCommand::RemoteClearanceImport {
            root: root.clone(),
            request: request_out,
            response: retained_response_out,
            expected_peer_ref: Some(peer_ref),
            expected_remote_ref: Some(remote_ref),
            out: Some(retained_import_out.clone()),
        })
        .expect("retention retained remote clearance import");
        let retained_import = retention::parse_retention_remote_gc_clearance_import(
            &read_preserves_file(&retained_import_out).expect("read retained clearance import"),
        )
        .expect("parse retained clearance import");
        assert_eq!(retained_import.decision, "deny");
        assert!(retained_import.clearance_ref.is_none());
        assert!(retained_import.diagnostics.iter().any(|diagnostic| diagnostic.contains("retained")));
        let pin_out = dir.join("pin.preserves");
        let pin_receipt_out = dir.join("pin-receipt.preserves");
        run_retention_command(RetentionCommand::Pin {
            root: root.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            source: retention::SOURCE_SECRET_REDACTION.to_string(),
            reason: "reveal audit pending".to_string(),
            owner_ref: owner_ref.clone(),
            expiry_ref: None,
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            has_authority: true,
            pin_out: Some(pin_out.clone()),
            receipt_out: Some(pin_receipt_out.clone()),
        })
        .expect("pin retention object");
        let pin = retention::parse_retention_pin(&read_preserves_file(&pin_out).expect("read pin")).expect("parse pin");
        let denied_receipt = dir.join("delete-denied.preserves");
        run_retention_command(RetentionCommand::Check {
            root: root.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_DELETE.to_string(),
            requester_ref: owner_ref.clone(),
            is_reference_index_complete: true,
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
            receipt_out: Some(denied_receipt.clone()),
        })
        .expect("deny pinned delete");
        let denied =
            retention::parse_retention_receipt(&read_preserves_file(&denied_receipt).expect("read denied receipt"))
                .expect("parse denied receipt");
        assert_eq!(denied.decision, "deny");
        let unpin_receipt = dir.join("unpin-receipt.preserves");
        run_retention_command(RetentionCommand::Unpin {
            root: root.clone(),
            pin_ref: pin.pin_ref,
            requester_ref: owner_ref.clone(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            has_authority: true,
            receipt_out: Some(unpin_receipt),
        })
        .expect("unpin retention object");
        let tombstone_receipt = dir.join("tombstone-receipt.preserves");
        run_retention_command(RetentionCommand::Check {
            root: root.clone(),
            object_ref: object_ref.clone(),
            object_kind: "encrypted-ref".to_string(),
            retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
            action: retention::ACTION_TOMBSTONE.to_string(),
            requester_ref: owner_ref,
            is_reference_index_complete: true,
            retained_refs: Vec::new(),
            remote_refs: Vec::new(),
            policy_refs: vec![policy_ref],
            evidence_refs: vec![evidence_ref],
            has_delete_authority: true,
            has_remote_gc_clearance: true,
            receipt_out: Some(tombstone_receipt.clone()),
        })
        .expect("tombstone retention object");
        let tombstone = retention::parse_retention_receipt(
            &read_preserves_file(&tombstone_receipt).expect("read tombstone receipt"),
        )
        .expect("parse tombstone receipt");
        assert_eq!(tombstone.decision, "pass");
        assert!(tombstone.tombstone_ref.is_some());
        let audit_object_ref = cli_synthetic_ref("retention-audit-object").expect("audit object ref");
        let audit_object = RetentionCliObject {
            root: &root,
            label: "retention-audit",
            object_ref: &audit_object_ref,
            object_kind: "encrypted-ref",
            retention_class: retention::CLASS_PRIVATE_SECRET_REF,
            action: retention::ACTION_DELETE,
        };
        let audit_retention = retention_cli_args_for_object(audit_object);
        let audit_apply_ref = retention_apply_ref(audit_object, "ledger-gc", &audit_retention);
        let audit_execution = retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: retention::ACTION_DELETE,
            object_ref: &audit_object_ref,
            object_kind: "encrypted-ref",
            retention_class: retention::CLASS_PRIVATE_SECRET_REF,
            apply_ref: Some(&audit_apply_ref),
        })
        .expect("store audit execution gate");
        assert_eq!(audit_execution.decision, "pass");
        let audit_out = dir.join("gc-audit.preserves");
        run_retention_command(RetentionCommand::GcAudit {
            root: root.clone(),
            execution_ref: audit_execution.execution_ref,
            out: Some(audit_out.clone()),
        })
        .expect("retention gc audit");
        let audit =
            retention::parse_retention_gc_audit(&read_preserves_file(&audit_out).expect("read retention gc audit"))
                .expect("parse retention gc audit");
        assert_eq!(audit.decision, "pass");
        assert_eq!(audit.apply_ref.as_deref(), Some(audit_apply_ref.as_str()));
        assert!(audit.retention_receipt_ref.is_some());
        assert!(audit.tombstone_ref.is_some());
        run_retention_command(RetentionCommand::Show { artifact: audit_out }).expect("show retention gc audit");
        let fixture_out = dir.join("fixture");
        run_retention_command(RetentionCommand::RunFixture {
            out: fixture_out.clone(),
        })
        .expect("retention fixture");
        assert!(fixture_out.join("tombstone.preserves").exists());
    }

    #[test]
    fn cli_provenance_commands_work() {
        let dir = temp_dir("provenance-cli");
        let artifact_ref = cli_synthetic_ref("provenance-artifact").expect("artifact ref");
        let fixture_out = dir.join("reviewed.preserves");
        run_provenance_command(ProvenanceCommand::Fixture {
            artifact_ref: artifact_ref.clone(),
            out: Some(fixture_out.clone()),
        })
        .expect("write reviewed provenance fixture");
        run_provenance_command(ProvenanceCommand::Show {
            artifact: fixture_out.clone(),
        })
        .expect("show provenance fixture");
        let pass_receipt = dir.join("provenance-pass.preserves");
        run_provenance_command(ProvenanceCommand::Evaluate {
            operation: "install".to_string(),
            profile: "node-control".to_string(),
            artifact_ref: artifact_ref.clone(),
            provenance_paths: vec![fixture_out.clone()],
            build_verification_paths: Vec::new(),
            prior_diagnostics: Vec::new(),
            receipt_out: Some(pass_receipt.clone()),
        })
        .expect("evaluate passing provenance");
        let pass_summary =
            provenance::provenance_summary(&read_preserves_file(&pass_receipt).expect("read provenance pass receipt"))
                .expect("summarize provenance pass receipt");
        assert!(pass_summary.contains("decision=pass"));

        let sandbox_ref = cli_synthetic_ref("provenance-sandbox-artifact").expect("sandbox ref");
        let sandbox_out = dir.join("sandbox.preserves");
        run_provenance_command(ProvenanceCommand::Record {
            artifact_ref: sandbox_ref.clone(),
            trust_state: provenance::TRUST_STATE_SANDBOX_ONLY.to_string(),
            source_refs: vec![cli_synthetic_ref("provenance-source").expect("source ref")],
            dependency_closure_ref: cli_synthetic_ref("provenance-deps").expect("deps ref"),
            toolchain_refs: vec![cli_synthetic_ref("provenance-toolchain").expect("toolchain ref")],
            builder_ref: cli_synthetic_ref("provenance-builder").expect("builder ref"),
            review_refs: Vec::new(),
            test_refs: Vec::new(),
            source_gate_refs: Vec::new(),
            policy_refs: vec![cli_synthetic_ref("provenance-policy").expect("policy ref")],
            build_record_refs: Vec::new(),
            out: Some(sandbox_out.clone()),
        })
        .expect("write sandbox provenance record");
        let deny_receipt = dir.join("provenance-deny.preserves");
        run_provenance_command(ProvenanceCommand::Evaluate {
            operation: "run".to_string(),
            profile: "node-control".to_string(),
            artifact_ref: sandbox_ref,
            provenance_paths: vec![sandbox_out],
            build_verification_paths: Vec::new(),
            prior_diagnostics: Vec::new(),
            receipt_out: Some(deny_receipt.clone()),
        })
        .expect("evaluate denied provenance");
        let deny_summary =
            provenance::provenance_summary(&read_preserves_file(&deny_receipt).expect("read provenance deny receipt"))
                .expect("summarize provenance deny receipt");
        assert!(deny_summary.contains("decision=deny"));

        let build_record = dir.join("build-record.preserves");
        let actual_ref = cli_synthetic_ref("provenance-actual-artifact").expect("actual ref");
        run_provenance_command(ProvenanceCommand::BuildRecord {
            expected_artifact_ref: artifact_ref.clone(),
            source_refs: vec![cli_synthetic_ref("provenance-build-source").expect("build source ref")],
            dependency_closure_ref: cli_synthetic_ref("provenance-build-deps").expect("build deps ref"),
            toolchain_refs: vec![cli_synthetic_ref("provenance-build-toolchain").expect("build toolchain ref")],
            build_params: vec!["target=x86_64-linux".to_string()],
            builder_ref: cli_synthetic_ref("provenance-build-builder").expect("build builder ref"),
            nix_derivation_refs: vec![cli_synthetic_ref("provenance-build-derivation").expect("build derivation ref")],
            policy_refs: vec![cli_synthetic_ref("provenance-build-policy").expect("build policy ref")],
            evidence_refs: vec![cli_synthetic_ref("provenance-build-evidence").expect("build evidence ref")],
            out: Some(build_record.clone()),
        })
        .expect("write provenance build record");
        run_provenance_command(ProvenanceCommand::Show {
            artifact: build_record.clone(),
        })
        .expect("show provenance build record");
        let build_pass = dir.join("build-pass.preserves");
        run_provenance_command(ProvenanceCommand::VerifyBuild {
            build_record: build_record.clone(),
            actual_artifact_ref: artifact_ref.clone(),
            prior_diagnostics: Vec::new(),
            receipt_out: Some(build_pass.clone()),
        })
        .expect("verify provenance build pass");
        let build_pass_summary = provenance::provenance_summary(
            &read_preserves_file(&build_pass).expect("read provenance build pass receipt"),
        )
        .expect("summarize provenance build pass");
        assert!(build_pass_summary.contains("decision=pass"));
        let build_record_ref =
            canonical_hash(&read_preserves_file(&build_record).expect("read build record")).expect("build record ref");
        let reproducible_record = dir.join("reproducible.preserves");
        run_provenance_command(ProvenanceCommand::Record {
            artifact_ref: artifact_ref.clone(),
            trust_state: provenance::TRUST_STATE_REPRODUCIBLE_VERIFIED.to_string(),
            source_refs: vec![cli_synthetic_ref("provenance-repro-source").expect("repro source ref")],
            dependency_closure_ref: cli_synthetic_ref("provenance-repro-deps").expect("repro deps ref"),
            toolchain_refs: vec![cli_synthetic_ref("provenance-repro-toolchain").expect("repro toolchain ref")],
            builder_ref: cli_synthetic_ref("provenance-repro-builder").expect("repro builder ref"),
            review_refs: Vec::new(),
            test_refs: Vec::new(),
            source_gate_refs: Vec::new(),
            policy_refs: Vec::new(),
            build_record_refs: vec![build_record_ref],
            out: Some(reproducible_record.clone()),
        })
        .expect("write reproducible provenance record");
        let reproducible_receipt = dir.join("provenance-reproducible-pass.preserves");
        run_provenance_command(ProvenanceCommand::Evaluate {
            operation: "install".to_string(),
            profile: "node-control".to_string(),
            artifact_ref: artifact_ref.clone(),
            provenance_paths: vec![reproducible_record],
            build_verification_paths: vec![build_pass.clone()],
            prior_diagnostics: Vec::new(),
            receipt_out: Some(reproducible_receipt.clone()),
        })
        .expect("evaluate reproducible provenance");
        let reproducible_summary = provenance::provenance_summary(
            &read_preserves_file(&reproducible_receipt).expect("read reproducible receipt"),
        )
        .expect("summarize reproducible receipt");
        assert!(reproducible_summary.contains("decision=pass"));
        let build_deny = dir.join("build-deny.preserves");
        run_provenance_command(ProvenanceCommand::VerifyBuild {
            build_record,
            actual_artifact_ref: actual_ref,
            prior_diagnostics: Vec::new(),
            receipt_out: Some(build_deny.clone()),
        })
        .expect("verify provenance build deny");
        let build_deny_summary = provenance::provenance_summary(
            &read_preserves_file(&build_deny).expect("read provenance build deny receipt"),
        )
        .expect("summarize provenance build deny");
        assert!(build_deny_summary.contains("decision=deny"));
    }

    #[test]
    fn cli_service_runtime_commands_work() {
        let dir = temp_dir("service-cli");
        let out = dir.join("two-service");
        run_service_command(ServiceCommand::RunTwoService { out: out.clone() })
            .expect("run two-service service runtime");
        let report = out.join("report.preserves");
        assert!(report.exists());
        run_service_command(ServiceCommand::Show { report: report.clone() }).expect("show service runtime report");
        run_service_command(ServiceCommand::Replay { report: report.clone() }).expect("replay service runtime report");
        let rerun = dir.join("rerun");
        run_service_command(ServiceCommand::Run {
            suite: out.join("suite.preserves"),
            out: rerun.clone(),
        })
        .expect("rerun service runtime suite");
        assert!(read_preserves_file(&rerun.join("readiness-1.preserves")).is_ok());
    }

    #[test]
    fn cli_service_supervision_commands_work() {
        let dir = temp_dir("service-supervision-cli");
        let out = dir.join("supervision");
        run_service_command(ServiceCommand::RunSupervisionFixture { out: out.clone() })
            .expect("run service supervision fixture");
        let report = out.join("report.preserves");
        assert!(report.exists());
        run_service_command(ServiceCommand::ShowSupervision { report: report.clone() })
            .expect("show service supervision report");
        run_service_command(ServiceCommand::ReplaySupervision { report: report.clone() })
            .expect("replay service supervision report");
        let gate_receipt = dir.join("supervision-gate.preserves");
        run_service_command(ServiceCommand::GateSupervision {
            report: report.clone(),
            receipt_out: Some(gate_receipt.clone()),
        })
        .expect("gate service supervision report");
        let gate = service_supervision::parse_service_supervision_gate_receipt(
            &read_preserves_file(&gate_receipt).expect("read supervision gate receipt"),
        )
        .expect("parse supervision gate receipt");
        assert_eq!(gate.decision, "pass");
        let rerun = dir.join("supervision-rerun");
        run_service_command(ServiceCommand::Supervise {
            suite: out.join("suite.preserves"),
            out: rerun.clone(),
        })
        .expect("rerun service supervision suite");
        assert!(read_preserves_file(&rerun.join("monitor-notification-0.preserves")).is_ok());
    }

    #[test]
    fn cli_remote_dataspace_commands_work() {
        let dir = temp_dir("remote-cli");
        let payload = PathBuf::from("examples/remote-service-ready.preserves");
        let parsed_payload = read_preserves_file(&payload).expect("example payload parses");
        let payload_ref = canonical_hash(&parsed_payload).expect("payload ref");
        molten::preserves_rail::validate_content_ref(&payload_ref).expect("payload ref is canonical");
        let envelope_out = dir.join("envelope.preserves");
        run_remote_command(RemoteCommand::Envelope {
            command: RemoteEnvelopeCommand::Build {
                from_peer: "peer:a".to_owned(),
                from_actor: "producer".to_owned(),
                to_peer: "peer:b".to_owned(),
                topic: "services".to_owned(),
                operation: "assert".to_owned(),
                payload,
                content_refs: Vec::new(),
                capability_refs: Vec::new(),
                evidence_refs: Vec::new(),
                out: envelope_out.clone(),
            },
        })
        .expect("build remote envelope");
        let envelope = remote_dataspace::parse_envelope(&read_preserves_file(&envelope_out).expect("read envelope"))
            .expect("parse envelope");
        let transport_root = dir.join("transport");
        run_remote_command(RemoteCommand::PublishLocal {
            transport_root: transport_root.clone(),
            envelope: envelope_out.clone(),
            node: "peer:a".to_owned(),
            receipt_out: Some(dir.join("publish.preserves")),
        })
        .expect("publish remote envelope");
        run_remote_command(RemoteCommand::DeliverLocal {
            transport_root: transport_root.clone(),
            topic: "services".to_owned(),
            envelope_ref: envelope.envelope_ref,
            receiver_peer: "peer:b".to_owned(),
            out: Some(dir.join("delivered.preserves")),
            receipt_out: Some(dir.join("deliver.preserves")),
        })
        .expect("deliver remote envelope");

        let out = dir.join("two-peer");
        run_remote_command(RemoteCommand::RunTwoPeer {
            transport_root,
            out: out.clone(),
        })
        .expect("run two peer remote scenario");
        let turn_ref_value = read_preserves_file(&out.join("turn-context-ref.preserves")).expect("read turn ref");
        let turn_ref = turn_ref_value.as_string().expect("turn ref string").into_owned();
        let gate_out = dir.join("remote-gate.preserves");
        run_remote_command(RemoteCommand::Gate {
            delivery_log: out.join("delivery-log.preserves"),
            admission_receipts: vec![out.join("admission-receipt.preserves")],
            turn_context_refs: vec![turn_ref],
            receipt_out: Some(gate_out.clone()),
        })
        .expect("remote gate");
        let gate = read_preserves_file(&gate_out).expect("read remote gate");
        assert_eq!(ledger::artifact_kind(&gate), "remote-dataspace-gate-receipt");
        let missing = run_remote_command(RemoteCommand::Gate {
            delivery_log: out.join("delivery-log.preserves"),
            admission_receipts: Vec::new(),
            turn_context_refs: Vec::new(),
            receipt_out: None,
        })
        .expect_err("missing admission receipt denies remote gate");
        assert!(missing.to_string().contains("admission receipt"));
    }

    #[test]
    fn cli_job_dag_commands_work() {
        let dir = temp_dir("job-cli");
        let registry = dir.join("registry");
        let storage = dir.join("storage");
        let cache = dir.join("cache");
        let chunks = dir.join("chunks");
        let ledger_root = dir.join("ledger");
        let dag_file = dir.join("job.preserves");
        let output = dir.join("job-output.preserves");
        let run_receipt = dir.join("job-run-receipt.preserves");
        let source_stage = install_cli_stage_artifact(&registry, "source");
        let reduce_stage = install_cli_stage_artifact(&registry, "sum-u64");
        let materialize_stage = install_cli_stage_artifact(&registry, "materialize");
        let source = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&source_stage),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: record("source", vec![record("values", vec![molten::preserves_rail::sequence(vec![
                molten::preserves_rail::u64_value(1),
                molten::preserves_rail::u64_value(2),
            ])])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node");
        let reduce = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "sum",
            kind: "reduce",
            stage_artifact_ref: Some(&reduce_stage),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("sum-u64")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("reduce node");
        let materialize = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "out",
            kind: "materialize",
            stage_artifact_ref: Some(&materialize_stage),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("materialize", vec![string("inline")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("materialize node");
        let e1 = job_dag::job_edge_value(job_dag::EdgeValueInput {
            from_node: "source",
            from_port: "out",
            to_node: "sum",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("e1");
        let e2 = job_dag::job_edge_value(job_dag::EdgeValueInput {
            from_node: "sum",
            from_port: "out",
            to_node: "out",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("e2");
        let dag_value = job_dag::job_dag_value(job_dag::DagValueInput {
            nodes: vec![source, reduce, materialize],
            edges: vec![e1, e2],
            output_roots: &["out".to_string()],
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("dag value");
        let dag = job_dag::parse_job_dag_value(&dag_value).expect("parse dag");
        write_file(&dag_file, &to_text(&dag_value).expect("dag text")).expect("write dag");
        run_job_command(JobCommand::Install {
            dag: dag_file.clone(),
            registry: registry.clone(),
            receipt_out: Some(dir.join("job-install-receipt.preserves")),
            artifact_out: Some(dir.join("job-artifact.preserves")),
        })
        .expect("job install");
        run_job_command(JobCommand::Show {
            job: dag.job_ref.clone(),
            registry: registry.clone(),
        })
        .expect("job show");
        let plan_out = dir.join("job-plan.preserves");
        let profile_out = dir.join("job-profile.preserves");
        let fusion_out = dir.join("job-fusion.preserves");
        let target_registry = dir.join("target-registry");
        let sync_plan_out = dir.join("job-sync-plan.preserves");
        let sync_loopback_receipt = dir.join("job-sync-loopback-receipt.preserves");
        let source_artifacts = artifacts::list_artifacts(&registry, None).expect("list source artifacts");
        let mut provenance_paths = Vec::with_capacity(source_artifacts.len());
        for artifact in source_artifacts {
            let provenance_path = dir.join(format!("job-provenance-{}.preserves", provenance_paths.len()));
            let provenance_value =
                provenance::synthetic_reviewed_provenance_record(&artifact.artifact_ref).expect("provenance");
            write_file(&provenance_path, &to_text(&provenance_value).expect("provenance text"))
                .expect("write provenance");
            provenance_paths.push(provenance_path);
        }
        let admit_plan_out = dir.join("job-admit-plan.preserves");
        let admit_loopback_receipt = dir.join("job-admit-loopback-receipt.preserves");
        run_job_command(JobCommand::Plan {
            job: dag.job_ref.clone(),
            registry: registry.clone(),
            output_request: None,
            out: Some(plan_out.clone()),
            receipt_out: Some(dir.join("job-plan-receipt.preserves")),
        })
        .expect("job plan");
        run_job_command(JobCommand::Profile {
            job: dag.job_ref.clone(),
            registry: registry.clone(),
            cache: Some(cache.clone()),
            output_request: None,
            out: Some(profile_out.clone()),
            receipt_out: Some(dir.join("job-profile-receipt.preserves")),
        })
        .expect("job profile");
        run_job_command(JobCommand::FusionPreview {
            job: dag.job_ref.clone(),
            registry: registry.clone(),
            output_request: None,
            out: Some(fusion_out.clone()),
            receipt_out: Some(dir.join("job-fusion-receipt.preserves")),
        })
        .expect("job fusion preview");
        assert!(fs::read_to_string(&plan_out).expect("read plan").contains("job-plan-v1"));
        assert!(fs::read_to_string(&profile_out).expect("read profile").contains("job-profile-v1"));
        assert!(fs::read_to_string(&fusion_out).expect("read fusion").contains("job-fusion-plan-v1"));
        run_job_command(JobCommand::SyncPlan {
            job: dag.job_ref.clone(),
            source_registry: registry.clone(),
            target_registry: target_registry.clone(),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            out: Some(sync_plan_out.clone()),
            receipt_out: Some(dir.join("job-sync-plan-receipt.preserves")),
        })
        .expect("job sync plan");
        run_job_command(JobCommand::SyncLoopback {
            job: dag.job_ref.clone(),
            source_registry: registry.clone(),
            target_registry: target_registry.clone(),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            provenance_paths,
            build_verification_paths: Vec::new(),
            plan_out: Some(dir.join("job-sync-loopback-plan.preserves")),
            receipt_out: Some(sync_loopback_receipt.clone()),
        })
        .expect("job sync loopback");
        assert!(fs::read_to_string(&sync_plan_out).expect("read sync plan").contains("job-sync-plan-v1"));
        assert!(
            !artifacts::list_artifacts(&target_registry, Some(job_dag::JOB_ARTIFACT_KIND))
                .expect("target jobs")
                .is_empty()
        );
        let sync_ref =
            canonical_hash(&read_preserves_file(&sync_loopback_receipt).expect("read sync receipt")).expect("sync ref");
        let authority_context_ref = install_cli_job_execute_authority_context(&target_registry, &dag.job_ref);
        let source_gate_ref = install_cli_clean_octet_gate(&target_registry);
        let admission_policy_ref = cli_synthetic_ref("job-worker-admission-policy").expect("policy ref");
        let worker_resource_refs = vec![
            cli_synthetic_ref("job-worker-resource-a").expect("resource a"),
            cli_synthetic_ref("job-worker-resource-b").expect("resource b"),
            cli_synthetic_ref("job-worker-resource-c").expect("resource c"),
        ];
        run_job_command(JobCommand::AdmitPlan {
            job: dag.job_ref.clone(),
            target_registry: target_registry.clone(),
            sync_ref: Some(sync_ref.clone()),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: vec![admission_policy_ref.clone()],
            capability_refs: vec![authority_context_ref.clone()],
            evidence_refs: vec![sync_ref.clone(), source_gate_ref.clone()],
            resource_refs: worker_resource_refs.clone(),
            out: Some(admit_plan_out.clone()),
            receipt_out: Some(dir.join("job-admit-plan-receipt.preserves")),
        })
        .expect("job admit plan");
        run_job_command(JobCommand::AdmitLoopback {
            job: dag.job_ref.clone(),
            target_registry: target_registry.clone(),
            sync_ref: Some(sync_ref.clone()),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: vec![admission_policy_ref.clone()],
            capability_refs: vec![authority_context_ref.clone()],
            evidence_refs: vec![sync_ref.clone(), source_gate_ref],
            resource_refs: worker_resource_refs.clone(),
            plan_out: Some(dir.join("job-admit-loopback-plan.preserves")),
            receipt_out: Some(admit_loopback_receipt.clone()),
        })
        .expect("job admit loopback");
        assert!(fs::read_to_string(&admit_plan_out).expect("read admit plan").contains("job-admission-plan-v1"));
        let missing_execution_receipt = dir.join("job-execute-missing-admission-receipt.preserves");
        run_job_command(JobCommand::ExecuteLoopback {
            job: dag.job_ref.clone(),
            target_registry: target_registry.clone(),
            storage: storage.clone(),
            cache: cache.clone(),
            chunks: Some(chunks.clone()),
            admission_receipt: dir.join("missing-admission.preserves"),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            resource_refs: Vec::new(),
            request_out: Some(dir.join("job-execute-missing-request.preserves")),
            out: None,
            receipt_out: Some(missing_execution_receipt.clone()),
        })
        .expect_err("missing admission denies execution");
        assert_eq!(
            ledger::artifact_kind(&read_preserves_file(&missing_execution_receipt).expect("missing execution receipt")),
            "job-execution-receipt"
        );
        let worker_execution_request = dir.join("job-worker-execution-request.preserves");
        run_job_command(JobCommand::ExecuteLoopback {
            job: dag.job_ref.clone(),
            target_registry: target_registry.clone(),
            storage: storage.clone(),
            cache: cache.clone(),
            chunks: Some(chunks.clone()),
            admission_receipt: admit_loopback_receipt.clone(),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: vec![admission_policy_ref],
            capability_refs: vec![authority_context_ref.clone()],
            resource_refs: worker_resource_refs.clone(),
            request_out: Some(worker_execution_request.clone()),
            out: Some(dir.join("job-execute-loopback-output.preserves")),
            receipt_out: Some(dir.join("job-execute-loopback-receipt.preserves")),
        })
        .expect("job execute loopback pass");
        let worker_request = dir.join("job-worker-request.preserves");
        let peer_bootstrap_ref = cli_synthetic_ref("job-worker-peer-bootstrap").expect("peer bootstrap");
        let node_identity_ref = cli_synthetic_ref("job-worker-node-identity").expect("node identity");
        run_job_command(JobCommand::WorkerRequest {
            admission_receipt: admit_loopback_receipt.clone(),
            execution_request: worker_execution_request.clone(),
            sync_ref: Some(sync_ref),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            authority_refs: vec![authority_context_ref.clone()],
            resource_refs: worker_resource_refs.clone(),
            peer_bootstrap_refs: vec![peer_bootstrap_ref],
            node_identity_refs: vec![node_identity_ref],
            evidence_refs: Vec::new(),
            out: Some(worker_request.clone()),
        })
        .expect("job worker request");
        let worker_out = dir.join("job-worker-local");
        run_job_command(JobCommand::WorkerRunLocal {
            request: worker_request.clone(),
            target_registry: target_registry.clone(),
            storage: dir.join("worker-storage"),
            cache: dir.join("worker-cache"),
            chunks: Some(dir.join("worker-chunks")),
            admission_receipt: admit_loopback_receipt.clone(),
            execution_request: worker_execution_request.clone(),
            transport_root: dir.join("worker-transport"),
            from_peer: "peer:source".to_string(),
            from_actor: "source-worker".to_string(),
            topic: "molten.job.worker".to_string(),
            ledger: Some(ledger_root.clone()),
            out: worker_out.clone(),
        })
        .expect("job worker local run");
        let worker_receipt = read_preserves_file(&worker_out.join("worker-receipt.preserves")).expect("worker receipt");
        assert_eq!(ledger::artifact_kind(&worker_receipt), "job-worker-receipt");
        assert!(fs::read_to_string(worker_out.join("output.preserves")).expect("worker output").contains("3"));
        let worker_receipt_ref = canonical_hash(&worker_receipt).expect("worker receipt ref");
        run_job_command(JobCommand::ReceiptShow {
            receipt_ref: worker_receipt_ref,
            ledger: ledger_root.clone(),
        })
        .expect("job worker receipt show");
        let schedule_out = dir.join("job-worker-scheduled");
        run_job_command(JobCommand::WorkerScheduleLocal {
            request: worker_request.clone(),
            target_registry: target_registry.clone(),
            storage: dir.join("scheduled-worker-storage"),
            cache: dir.join("scheduled-worker-cache"),
            chunks: Some(dir.join("scheduled-worker-chunks")),
            admission_receipt: admit_loopback_receipt.clone(),
            execution_request: worker_execution_request.clone(),
            transport_root: dir.join("scheduled-worker-transport"),
            queue_key: "queue:job-worker".to_string(),
            lease_key: None,
            scheduler_session: "scheduler".to_string(),
            worker_session: "worker-a".to_string(),
            lease_token: None,
            from_peer: "peer:source".to_string(),
            from_actor: "source-worker".to_string(),
            topic: "molten.job.worker".to_string(),
            coordination_authority_refs: vec![authority_context_ref.clone()],
            coordination_resource_refs: worker_resource_refs.clone(),
            coordination_policy_refs: vec![cli_synthetic_ref("job-worker-schedule-policy").expect("schedule policy")],
            ledger: Some(ledger_root.clone()),
            out: schedule_out.clone(),
        })
        .expect("job worker scheduled local run");
        let schedule_receipt =
            read_preserves_file(&schedule_out.join("schedule-receipt.preserves")).expect("schedule receipt");
        assert_eq!(ledger::artifact_kind(&schedule_receipt), "job-worker-schedule-receipt");
        assert!(
            fs::read_to_string(schedule_out.join("worker").join("output.preserves"))
                .expect("scheduled worker output")
                .contains("3")
        );
        let enqueue_ref = canonical_hash(
            &read_preserves_file(&schedule_out.join("coordination").join("enqueue-receipt.preserves"))
                .expect("enqueue receipt"),
        )
        .expect("enqueue ref");
        let duplicate_enqueue_ref = canonical_hash(
            &read_preserves_file(&schedule_out.join("coordination").join("enqueue-duplicate-receipt.preserves"))
                .expect("duplicate enqueue receipt"),
        )
        .expect("duplicate enqueue ref");
        assert_eq!(enqueue_ref, duplicate_enqueue_ref);
        let schedule_receipt_ref = canonical_hash(&schedule_receipt).expect("schedule receipt ref");
        run_job_command(JobCommand::ReceiptShow {
            receipt_ref: schedule_receipt_ref,
            ledger: ledger_root.clone(),
        })
        .expect("job worker schedule receipt show");
        let stale_schedule_out = dir.join("job-worker-stale-schedule");
        run_job_command(JobCommand::WorkerScheduleLocal {
            request: worker_request,
            target_registry: target_registry.clone(),
            storage: dir.join("stale-worker-storage"),
            cache: dir.join("stale-worker-cache"),
            chunks: Some(dir.join("stale-worker-chunks")),
            admission_receipt: admit_loopback_receipt,
            execution_request: worker_execution_request,
            transport_root: dir.join("stale-worker-transport"),
            queue_key: "queue:job-worker".to_string(),
            lease_key: None,
            scheduler_session: "scheduler".to_string(),
            worker_session: "worker-a".to_string(),
            lease_token: Some(0),
            from_peer: "peer:source".to_string(),
            from_actor: "source-worker".to_string(),
            topic: "molten.job.worker".to_string(),
            coordination_authority_refs: vec![authority_context_ref],
            coordination_resource_refs: worker_resource_refs,
            coordination_policy_refs: vec![cli_synthetic_ref("job-worker-stale-policy").expect("stale policy")],
            ledger: None,
            out: stale_schedule_out.clone(),
        })
        .expect_err("stale schedule token denies before worker");
        let stale_receipt = job_dag::parse_job_worker_schedule_receipt_value(
            &read_preserves_file(&stale_schedule_out.join("schedule-receipt.preserves")).expect("stale receipt"),
        )
        .expect("parse stale schedule receipt");
        assert_eq!(stale_receipt.decision, "deny");
        assert!(stale_receipt.diagnostics.join(";").contains("stale fencing token"));
        assert!(!stale_schedule_out.join("worker").join("worker-receipt.preserves").exists());
        run_job_command(JobCommand::Run {
            job: dag.job_ref.clone(),
            registry: registry.clone(),
            storage,
            cache,
            chunks: Some(chunks),
            ledger: Some(ledger_root.clone()),
            output_request: None,
            out: Some(output.clone()),
            receipt_out: Some(run_receipt.clone()),
        })
        .expect("job run");
        assert!(fs::read_to_string(&output).expect("read output").contains("3"));
        let receipt_ref =
            canonical_hash(&read_preserves_file(&run_receipt).expect("read run receipt")).expect("receipt ref");
        run_job_command(JobCommand::Status {
            ledger: ledger_root.clone(),
            job: Some(dag.job_ref.clone()),
        })
        .expect("job status");
        run_job_command(JobCommand::ReceiptShow {
            receipt_ref,
            ledger: ledger_root,
        })
        .expect("job receipt show");
    }

    #[test]
    fn cli_catalog_commands_work() {
        let dir = temp_dir("catalog-cli");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let base_payload = dir.join("catalog-base.preserves");
        let dep_payload = dir.join("catalog-dependent.preserves");
        let base_out = dir.join("catalog-base-artifact.preserves");
        let dep_out = dir.join("catalog-dependent-artifact.preserves");
        let list_receipt = dir.join("catalog-list-receipt.preserves");
        let view_receipt = dir.join("catalog-view-receipt.preserves");
        write_file(&base_payload, r#"<schema "catalog-base">"#).expect("write catalog base payload");
        write_file(&dep_payload, r#"<doc "catalog-text" ["searchable"]>"#).expect("write catalog dep payload");
        run_artifact_command(ArtifactCommand::Install {
            payload: base_payload,
            registry: registry.clone(),
            kind: "schema".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(base_out.clone()),
            receipt_out: Some(dir.join("catalog-base-install-receipt.preserves")),
        })
        .expect("install catalog base");
        let base =
            artifacts::parse_artifact_value(&read_preserves_file(&base_out).expect("read base")).expect("parse base");
        run_artifact_command(ArtifactCommand::Install {
            payload: dep_payload,
            registry: registry.clone(),
            kind: "doc".to_string(),
            dependencies: vec![base.artifact_ref.clone()],
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(dep_out.clone()),
            receipt_out: Some(dir.join("catalog-dep-install-receipt.preserves")),
        })
        .expect("install catalog dependent");
        let dep =
            artifacts::parse_artifact_value(&read_preserves_file(&dep_out).expect("read dep")).expect("parse dep");
        ledger::import_artifact(&ledger_root, &dep.value).expect("import dep artifact to ledger");
        run_catalog_command(CatalogCommand::List {
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            kind: Some("doc".to_string()),
            hidden_refs: Vec::new(),
            receipt_out: Some(list_receipt.clone()),
        })
        .expect("catalog list");
        run_catalog_command(CatalogCommand::View {
            reference: dep.artifact_ref.clone(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            payload_inclusion_enabled: true,
            redaction_enabled: true,
            hidden_refs: Vec::new(),
            receipt_out: Some(view_receipt.clone()),
        })
        .expect("catalog view");
        run_catalog_command(CatalogCommand::Search {
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            artifact_kind: Some("doc".to_string()),
            ledger_kind: None,
            schema_ref: None,
            structural_fingerprint: None,
            effect_ref: None,
            policy_ref: None,
            capability_ref: None,
            evidence_ref: None,
            dependency_ref: Some(base.artifact_ref.clone()),
            dependent_ref: None,
            receipt_operation: None,
            receipt_decision: None,
            transcript_status: None,
            upgrade_status: None,
            text: Some("searchable".to_string()),
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            dependent_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-search-receipt.preserves")),
        })
        .expect("catalog search");
        run_catalog_command(CatalogCommand::Deps {
            reference: dep.artifact_ref.clone(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            transitive: false,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-deps-receipt.preserves")),
        })
        .expect("catalog deps");
        run_catalog_command(CatalogCommand::Dependents {
            reference: base.artifact_ref.clone(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            transitive: false,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-dependents-receipt.preserves")),
        })
        .expect("catalog dependents");
        run_catalog_command(CatalogCommand::ShortId {
            prefix: dep.artifact_ref[7..19].to_string(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            min_length: 8,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-short-id-receipt.preserves")),
        })
        .expect("catalog short id");
        let mcp_request = dir.join("catalog-mcp-request.preserves");
        let mcp_response = dir.join("catalog-mcp-response.preserves");
        let mcp_receipt = dir.join("catalog-mcp-receipt.preserves");
        write_file(
            &mcp_request,
            &to_text(
                &catalog_mcp::mcp_request_value("catalog.search", vec![
                    record("kind", vec![string("doc")]),
                    record("dependency-ref", vec![string(&base.artifact_ref)]),
                    record("text", vec![string("searchable")]),
                ])
                .expect("mcp request"),
            )
            .expect("render mcp request"),
        )
        .expect("write mcp request");
        run_catalog_command(CatalogCommand::McpCall {
            request: mcp_request,
            registry,
            ledger: Some(ledger_root),
            out: Some(mcp_response.clone()),
            receipt_out: Some(mcp_receipt.clone()),
        })
        .expect("catalog mcp call");
        assert!(fs::read_to_string(&mcp_response).expect("read mcp response").contains(&dep.artifact_ref));
        run_catalog_command(CatalogCommand::Show { artifact: mcp_receipt }).expect("catalog show MCP receipt");
        run_catalog_command(CatalogCommand::Show { artifact: list_receipt }).expect("catalog show receipt");
        run_catalog_command(CatalogCommand::Show { artifact: view_receipt }).expect("catalog show view receipt");
    }

    #[test]
    fn cli_dogfood_local_node_commands_work() {
        let dir = temp_dir("dogfood-cli");
        let state_root = dir.join("state");
        let report = dir.join("dogfood-report.preserves");
        let release_gate = dir.join("release-gate.preserves");
        run_dogfood_command(DogfoodCommand::LocalNode {
            state_root: state_root.clone(),
            out: report.clone(),
            release_gate_out: Some(release_gate.clone()),
        })
        .expect("dogfood local node");
        let report_value = read_preserves_file(&report).expect("read dogfood report");
        let parsed = operator_dogfood::parse_dogfood_report(&report_value).expect("parse dogfood report");
        assert_eq!(parsed.decision, "pass");
        assert!(fs::read_to_string(&release_gate).expect("read release gate").contains("release-gate-receipt-v1"));
        let ledger_root = state_root.join("ledger");
        run_receipts_command(ReceiptsCommand::List {
            ledger: ledger_root.clone(),
        })
        .expect("receipts list");
        run_receipts_command(ReceiptsCommand::Show {
            receipt_ref: parsed.report_ref.clone(),
            ledger: ledger_root.clone(),
        })
        .expect("receipts show dogfood report");
        run_receipts_command(ReceiptsCommand::Validate {
            receipt_ref: parsed.report_ref.clone(),
            ledger: ledger_root.clone(),
        })
        .expect("receipts validate dogfood report");
        let exported_report = dir.join("exported-dogfood-report.preserves");
        run_receipts_command(ReceiptsCommand::Export {
            receipt_ref: parsed.report_ref.clone(),
            ledger: ledger_root,
            out: exported_report.clone(),
            receipt_out: Some(dir.join("receipts-export.preserves")),
        })
        .expect("receipts export dogfood report");
        assert_eq!(
            canonical_hash(&read_preserves_file(&exported_report).expect("exported dogfood report"))
                .expect("exported ref"),
            parsed.report_ref
        );
        fs::write(
            dir.join("dogfood-summary.txt"),
            format!(
                "dogfood local-node decision=pass report={} release-gate={}\n",
                parsed.report_ref,
                canonical_hash(&read_preserves_file(&release_gate).expect("release gate value")).expect("release ref")
            ),
        )
        .expect("write summary");
        fs::write(dir.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n").expect("write nextest marker");
        let nix_evidence = dir.join("nix-dogfood-evidence.preserves");
        let nix_verify = dir.join("nix-dogfood-verify.preserves");
        run_dogfood_command(DogfoodCommand::NixReleaseExport {
            output_path: dir.clone(),
            out: nix_evidence.clone(),
        })
        .expect("dogfood nix release export");
        run_dogfood_command(DogfoodCommand::NixReleaseVerify {
            output_path: dir.clone(),
            evidence: nix_evidence.clone(),
            receipt_out: nix_verify.clone(),
        })
        .expect("dogfood nix release verify");
        let verify_value = read_preserves_file(&nix_verify).expect("read nix verify");
        let verify = operator_dogfood::parse_nix_dogfood_verify_receipt(&verify_value).expect("parse nix verify");
        assert_eq!(verify.decision, "pass");
        fs::write(dir.join("after-nextest.txt"), "/nix/store/stale-molten-nextest\n").expect("tamper nextest marker");
        let stale_verify = dir.join("nix-dogfood-verify-stale.preserves");
        run_dogfood_command(DogfoodCommand::NixReleaseVerify {
            output_path: dir.clone(),
            evidence: nix_evidence.clone(),
            receipt_out: stale_verify.clone(),
        })
        .expect("dogfood nix release verify stale marker");
        let stale_verify_value = read_preserves_file(&stale_verify).expect("read stale nix verify");
        let stale_verify_receipt =
            operator_dogfood::parse_nix_dogfood_verify_receipt(&stale_verify_value).expect("parse stale nix verify");
        assert_eq!(stale_verify_receipt.decision, "deny");
        assert!(
            stale_verify_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("nextest-marker-ref mismatch"))
        );
        fs::write(dir.join("dogfood-report.preserves"), "<tampered-dogfood-report>\n").expect("tamper report");
        let tampered_verify = dir.join("nix-dogfood-verify-tampered.preserves");
        run_dogfood_command(DogfoodCommand::NixReleaseVerify {
            output_path: dir.clone(),
            evidence: nix_evidence.clone(),
            receipt_out: tampered_verify.clone(),
        })
        .expect("dogfood nix release verify tampered report");
        let tampered_verify_value = read_preserves_file(&tampered_verify).expect("read tampered nix verify");
        let tampered_verify_receipt = operator_dogfood::parse_nix_dogfood_verify_receipt(&tampered_verify_value)
            .expect("parse tampered nix verify");
        assert_eq!(tampered_verify_receipt.decision, "deny");
        assert!(
            tampered_verify_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("Nix dogfood output observation failed"))
        );
        fs::write(dir.join("dogfood-report.preserves"), to_text(&report_value).expect("report text"))
            .expect("restore report");
        run_dogfood_command(DogfoodCommand::Show { artifact: report }).expect("dogfood show report");
        run_dogfood_command(DogfoodCommand::Show { artifact: release_gate }).expect("dogfood show gate");
        run_dogfood_command(DogfoodCommand::Show { artifact: nix_evidence }).expect("dogfood show nix evidence");
        run_dogfood_command(DogfoodCommand::Show { artifact: nix_verify }).expect("dogfood show nix verify");
    }

    #[test]
    fn cli_coordination_commands_work() {
        let dir = temp_dir("coordination-cli");
        let out = dir.join("coordination-fixture");
        run_coordination_command(CoordinationCommand::RunFixture { out: out.clone() }).expect("coordination fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read coordination report");
        assert!(
            to_text(&report_value)
                .expect("render coordination report")
                .contains("coordination-fixture-report-v1")
        );
        let manifest = out.join("evidence-0.preserves");
        let manifest_value = read_preserves_file(&manifest).expect("read coordination manifest");
        let parsed =
            coordination::parse_coordination_service_manifest(&manifest_value).expect("parse coordination manifest");
        assert_eq!(parsed.service_id, "coordination:local");
        run_coordination_command(CoordinationCommand::Show { artifact: manifest }).expect("coordination show manifest");

        let policy_ref = cli_synthetic_ref("coordination-cli-policy").expect("policy ref");
        let resource_ref = cli_synthetic_ref("coordination-cli-resource").expect("resource ref");
        let authority_ref = cli_synthetic_ref("coordination-cli-authority").expect("authority ref");
        let operation_id_ref = cli_synthetic_ref("coordination-cli-operation").expect("operation ref");
        let generated_manifest = dir.join("coordination.manifest.preserves");
        run_coordination_command(CoordinationCommand::Manifest {
            service_id: "coordination:local".to_string(),
            services: vec![coordination::SERVICE_QUEUE.to_string()],
            control_group_ref: None,
            queue_capacity: 2,
            semaphore_capacity: coordination::DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
            rate_limit: coordination::DEFAULT_COORDINATION_RATE_LIMIT,
            barrier_parties: coordination::DEFAULT_COORDINATION_BARRIER_PARTIES,
            policy_refs: vec![policy_ref.clone()],
            resource_refs: vec![resource_ref.clone()],
            out: Some(generated_manifest.clone()),
        })
        .expect("coordination manifest");
        let generated_manifest_value = read_preserves_file(&generated_manifest).expect("read generated manifest");
        let generated_manifest_parsed = coordination::parse_coordination_service_manifest(&generated_manifest_value)
            .expect("parse generated coordination manifest");
        assert_eq!(generated_manifest_parsed.services, vec![coordination::SERVICE_QUEUE.to_string()]);

        let payload = dir.join("queue-item.preserves");
        write_file(&payload, r#"<item "cli-one">"#).expect("write queue payload");
        let request = dir.join("coordination.request.preserves");
        run_coordination_command(CoordinationCommand::Request {
            service: coordination::SERVICE_QUEUE.to_string(),
            operation: coordination::OP_ENQUEUE.to_string(),
            key: "queue:cli".to_string(),
            client_session: "client-cli".to_string(),
            operation_id_ref,
            payload: Some(payload),
            authority_refs: vec![authority_ref],
            resource_refs: vec![resource_ref],
            policy_refs: vec![policy_ref],
            out: Some(request.clone()),
        })
        .expect("coordination request");
        run_coordination_command(CoordinationCommand::Show {
            artifact: request.clone(),
        })
        .expect("show request");

        let apply_out = dir.join("coordination-apply");
        run_coordination_command(CoordinationCommand::Apply {
            manifest: generated_manifest,
            requests: vec![request.clone(), request],
            out: apply_out.clone(),
        })
        .expect("coordination apply");
        let apply_report = read_preserves_file(&apply_out.join("report.preserves")).expect("read apply report");
        let parsed_report = coordination::parse_coordination_apply_report(&apply_report).expect("parse apply report");
        assert_eq!(parsed_report.decision, "pass");
        assert_eq!(parsed_report.receipt_refs.len(), 2);
        assert_eq!(parsed_report.receipt_refs[0], parsed_report.receipt_refs[1]);
        assert_eq!(parsed_report.assertion_refs.len(), 2);
        assert_eq!(parsed_report.assertion_refs[0], parsed_report.assertion_refs[1]);
        run_coordination_command(CoordinationCommand::Show {
            artifact: apply_out.join("report.preserves"),
        })
        .expect("coordination show apply report");
    }

    #[test]
    fn cli_secrets_commands_work() {
        let dir = temp_dir("secrets-cli");
        let out = dir.join("secrets-fixture");
        run_secrets_command(SecretsCommand::RunFixture { out: out.clone() }).expect("secrets fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read secrets report");
        let summary = secrets::fixture_report_summary(&report_value).expect("summary");
        assert!(summary.contains("plaintext=redacted"));
        let secret = out.join("secret.preserves");
        let secret_value = read_preserves_file(&secret).expect("read secret");
        let parsed = secrets::parse_secret_ref(&secret_value).expect("parse secret");
        assert_eq!(parsed.secret_id, "secret:fixture");
        run_secrets_command(SecretsCommand::Show { artifact: report }).expect("show report");
        run_secrets_command(SecretsCommand::Show { artifact: secret }).expect("show secret");
    }

    #[test]
    fn cli_plugin_lifecycle_commands_work() {
        let dir = temp_dir("plugin-cli");
        let state_root = dir.join("state");
        let out = dir.join("plugin-fixture");
        run_plugin_command(PluginCommand::RunFixture {
            state_root,
            out: out.clone(),
        })
        .expect("plugin fixture");
        let report = out.join("report.preserves");
        let report_value = read_preserves_file(&report).expect("read plugin report");
        assert!(to_text(&report_value).expect("render plugin report").contains("plugin-fixture-report-v1"));
        let manifest = out.join("evidence-0.preserves");
        let manifest_value = read_preserves_file(&manifest).expect("read plugin manifest");
        let parsed = plugin_host::parse_plugin_manifest(&manifest_value).expect("parse plugin manifest");
        assert_eq!(parsed.plugin_id, "plugin:minimal");
        run_plugin_command(PluginCommand::Show { artifact: report }).expect("plugin show report");
        run_plugin_command(PluginCommand::Show { artifact: manifest }).expect("plugin show manifest");
    }

    #[test]
    fn cli_schema_identity_commands_work() {
        let dir = temp_dir("schema-cli");
        let registry = dir.join("registry");
        let shape_file = dir.join("shape.preserves");
        let expected_identity_out = dir.join("expected-identity.preserves");
        let actual_identity_out = dir.join("actual-identity.preserves");
        let alias_out = dir.join("alias.preserves");
        let compat_out = dir.join("compat.preserves");
        let shape = r#"<shape "record" "profile" [<shape "field" "name" <shape "string">> <shape "field" "age" <shape "u64">>]>"#;
        write_file(&shape_file, shape).expect("write shape");
        let expected_schema_ref = test_ref("expected-schema-cli");
        let actual_schema_ref = test_ref("actual-schema-cli");
        run_schema_command(SchemaCommand::Identity {
            shape: shape_file.clone(),
            schema_ref: expected_schema_ref.clone(),
            mode: "structural".to_string(),
            brand_ref: None,
            out: expected_identity_out.clone(),
            receipt_out: Some(dir.join("expected-identity-receipt.preserves")),
        })
        .expect("schema expected identity");
        run_schema_command(SchemaCommand::Identity {
            shape: shape_file,
            schema_ref: actual_schema_ref.clone(),
            mode: "structural".to_string(),
            brand_ref: None,
            out: actual_identity_out.clone(),
            receipt_out: Some(dir.join("actual-identity-receipt.preserves")),
        })
        .expect("schema actual identity");
        run_schema_command(SchemaCommand::Alias {
            from_ref: actual_schema_ref,
            to_ref: expected_schema_ref,
            scope: "storage".to_string(),
            out: alias_out.clone(),
            receipt_out: Some(dir.join("alias-receipt.preserves")),
        })
        .expect("schema alias");
        run_schema_command(SchemaCommand::Compat {
            expected_identity: expected_identity_out.clone(),
            actual_identity: actual_identity_out.clone(),
            alias: Some(alias_out),
            migration_ref: None,
            out: Some(compat_out.clone()),
            receipt_out: Some(dir.join("compat-receipt.preserves")),
        })
        .expect("schema compat");
        assert!(fs::read_to_string(&compat_out).expect("read compat").contains("schema-compatibility-v1"));
        let schema_artifact = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "schema".to_string(),
            payload: record("schema-source", vec![string("cli")]),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install schema artifact");
        let identity_value = schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
            mode: "structural".to_string(),
            schema_ref: schema_artifact.artifact_ref.clone(),
            shape: parse_text(shape).expect("parse shape"),
            brand_ref: None,
            metadata_refs: vec![test_ref("metadata")],
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("identity value");
        let identity = schema_identity::parse_schema_identity(&identity_value).expect("parse identity");
        artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "schema-identity".to_string(),
            payload: identity_value,
            schema_refs: vec![schema_artifact.artifact_ref.clone()],
            dependency_refs: vec![schema_artifact.artifact_ref],
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install schema identity artifact");
        run_schema_command(SchemaCommand::SearchFingerprint {
            registry,
            fingerprint: identity.structural_fingerprint,
        })
        .expect("schema search fingerprint");
    }

    #[test]
    fn cli_upgrade_session_commands_work() {
        let dir = temp_dir("upgrade-cli");
        let ledger_root = dir.join("ledger");
        let store = dir.join("upgrades");
        let old = ledger::import_artifact(&ledger_root, &record("cli-old-artifact", vec![string("old")]))
            .expect("import old")
            .artifact_ref;
        let new = ledger::import_artifact(&ledger_root, &record("cli-new-artifact", vec![string("new")]))
            .expect("import new")
            .artifact_ref;
        let plan_out = dir.join("upgrade-plan.preserves");
        let source_gate = dir.join("octet-gate-receipt.preserves");
        write_file(
            &source_gate,
            &to_text(&octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"))
                .expect("source gate text"),
        )
        .expect("write source gate");
        run_upgrade_command(UpgradeCommand::PlanNameMove {
            ledger: ledger_root.clone(),
            registry: None,
            session_id: "cli-upgrade".to_string(),
            name: "app/main".to_string(),
            from_ref: old.clone(),
            to_ref: new.clone(),
            source_gate_receipts: vec![source_gate],
            out: plan_out.clone(),
        })
        .expect("plan name move");
        let plan_value = read_preserves_file(&plan_out).expect("read plan");
        let plan = upgrades::parse_upgrade_plan(&plan_value).expect("parse plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: plan_out,
            store: store.clone(),
            receipt_out: Some(dir.join("upgrade-create-receipt.preserves")),
        })
        .expect("create upgrade");
        run_upgrade_command(UpgradeCommand::SetName {
            store: store.clone(),
            name: "app/main".to_string(),
            artifact_ref: old,
            receipt_out: Some(dir.join("upgrade-set-name-receipt.preserves")),
        })
        .expect("set initial name");
        for task_id in ["compatibility-alias", "transcript-gate", "move-name", "cutover"] {
            run_upgrade_command(UpgradeCommand::RunTask {
                store: store.clone(),
                ledger: ledger_root.clone(),
                plan_ref: plan.plan_ref.clone(),
                task_id: task_id.to_string(),
                receipt_out: Some(dir.join(format!("upgrade-{task_id}-receipt.preserves"))),
            })
            .expect("run upgrade task");
        }
        run_upgrade_command(UpgradeCommand::Status {
            store: store.clone(),
            plan_ref: plan.plan_ref.clone(),
        })
        .expect("upgrade status");
        let pointer = upgrades::read_name_pointer(&store, "app/main")
            .expect("read name pointer")
            .expect("name pointer exists");
        assert_eq!(pointer.artifact_ref, new);
        run_upgrade_command(UpgradeCommand::CleanupCheck {
            store,
            ledger: ledger_root,
            registry: None,
            artifact_ref: pointer.previous_ref.expect("previous ref"),
            receipt_out: Some(dir.join("upgrade-cleanup-receipt.preserves")),
        })
        .expect("cleanup check emits denial receipt");
    }

    #[test]
    fn cli_chain_publish_fetch_commands_work() {
        let dir = temp_dir("chain-cli");
        let ledger = dir.join("ledger-source");
        let destination = dir.join("ledger-destination");
        let iroh_store = dir.join("chain-iroh");
        let chain = molten::evidence_chain::ChainScope::new("cli-chain", "artifact", "epoch");
        let payload_value =
            molten::preserves_rail::record("cli-chain-payload", vec![molten::preserves_rail::string("ok")]);
        let payload_ref = ledger::import_artifact(&ledger, &payload_value).expect("import chain payload").artifact_ref;
        let input = molten::evidence_chain::ChainLinkInput::genesis(
            chain.clone(),
            molten::evidence_chain::ChainPayload::new("cli-chain-payload", payload_ref, "molten.test.payload.v1"),
            Vec::new(),
            molten::evidence_chain::ChainProducer::new("node:cli", test_ref("producer-key")),
            test_ref("genesis-input"),
        );
        let link_value = molten::evidence_chain::chain_link_value(&input);
        let link = molten::evidence_chain::parse_chain_link(&link_value).expect("parse chain link");
        molten::evidence_chain::append_chain_link(&ledger, &link_value).expect("append chain link");

        run_chain_command(ChainCommand::Publish {
            ledger: ledger.clone(),
            iroh_store: iroh_store.clone(),
            scope: chain.scope.clone(),
            id: chain.id.clone(),
            epoch: chain.epoch.clone(),
            anchor: None,
            head: Some(link.link_ref.clone()),
            node: "node:cli".to_string(),
            fork_policy: "reject-unexpected-forks".to_string(),
            receipt_out: Some(dir.join("chain-publish.preserves")),
        })
        .expect("publish chain segment");
        let bundle_ref = only_blob_ref(&iroh_store);
        run_chain_command(ChainCommand::Fetch {
            ticket: format!("iroh-local-chain:{bundle_ref}"),
            ledger: destination.clone(),
            iroh_store,
            expected_bundle_ref: Some(bundle_ref),
            peer: "peer:cli".to_string(),
            fork_policy: "reject-unexpected-forks".to_string(),
            receipt_out: Some(dir.join("chain-fetch.preserves")),
        })
        .expect("fetch chain segment");
        let entries = ledger::list_artifacts(&destination).expect("list destination ledger");
        assert!(entries.iter().any(|entry| entry.artifact_kind == "chain-link"));
        assert!(entries.iter().any(|entry| entry.artifact_kind == "iroh-chain-exchange-receipt"));
    }

    #[test]
    fn cli_receipt_ledger_and_repro_exchange_commands_work() {
        let dir = temp_dir("receipt-ledger-iroh");
        let suite = dir.join("suite.preserves");
        let report = dir.join("report.preserves");
        let gate_receipt = dir.join("gate.preserves");
        write_file(
            &suite,
            r#"<harness-suite-v1 "molten.harness.suite.v1" "cli-evidence" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "assert" #f "ready">]>
              [<assert "a" "ready">]>"#,
        )
        .expect("write suite");
        run_test_command(TestCommand::Run {
            suite: suite.clone(),
            report_out: Some(report.clone()),
        })
        .expect("run suite");
        run_gate_command(GateCommand::Check {
            artifact: report.clone(),
            failure_out: None,
            receipt_out: Some(gate_receipt.clone()),
        })
        .expect("write gate receipt");

        let signed = dir.join("signed.preserves");
        run_receipt_command(ReceiptCommand::Sign {
            receipt: gate_receipt.clone(),
            out: signed.clone(),
            signer: "local-signer".to_string(),
            purpose: PASS_EVIDENCE_PURPOSE.to_string(),
            trust_root: "local-trust-root".to_string(),
            key: "local-dev-key".to_string(),
            parents: Vec::new(),
        })
        .expect("sign receipt");
        run_receipt_command(ReceiptCommand::Verify {
            signed_receipt: signed.clone(),
            purpose: PASS_EVIDENCE_PURPOSE.to_string(),
            trust_root: "local-trust-root".to_string(),
            key: "local-dev-key".to_string(),
            key_ledger: None,
            key_ref: None,
            key_id: None,
            signer: Some("local-signer".to_string()),
            subject_ref: None,
        })
        .expect("verify signed receipt");

        let ledger = dir.join("ledger");
        let ledger_import_receipt = dir.join("ledger-import.preserves");
        run_ledger_command(LedgerCommand::Import {
            artifact: report.clone(),
            ledger: ledger.clone(),
            receipt_out: Some(ledger_import_receipt),
        })
        .expect("ledger import");
        let report_value = read_preserves_file(&report).expect("read report");
        let report_ref = molten::preserves_rail::canonical_hash(&report_value).expect("report ref");
        run_ledger_command(LedgerCommand::Pin {
            artifact_ref: report_ref.clone(),
            ledger: ledger.clone(),
        })
        .expect("ledger pin");
        run_ledger_command(LedgerCommand::Gc {
            ledger: ledger.clone(),
            dry_run: false,
            apply_refs: Vec::new(),
            retention: retention_cli_args("ledger-gc"),
            receipt_out: Some(dir.join("ledger-gc.preserves")),
        })
        .expect("ledger gc");
        run_ledger_command(LedgerCommand::Export {
            artifact_ref: report_ref,
            ledger,
            out: dir.join("report-export.preserves"),
            receipt_out: Some(dir.join("ledger-export.preserves")),
        })
        .expect("ledger export");

        let repro = dir.join("repro");
        run_repro_command(ReproCommand::Export {
            report: report.clone(),
            out: repro.clone(),
            profile: "deny-sensitive".to_string(),
            failure_out: None,
        })
        .expect("export repro");
        let refs = repro.join("refs.preserves");
        let store = dir.join("iroh-store");
        let publish_receipt = dir.join("publish.preserves");
        run_repro_command(ReproCommand::Publish {
            bundle: refs.clone(),
            store: store.clone(),
            node: "node:local".to_string(),
            receipt_out: Some(publish_receipt),
            failure_out: None,
        })
        .expect("publish repro");
        let bundle_ref = molten::preserves_rail::canonical_hash(&read_preserves_file(&refs).expect("read bundle"))
            .expect("bundle ref");
        run_repro_command(ReproCommand::Fetch {
            ticket: format!("iroh-local:{bundle_ref}"),
            store,
            out: Some(dir.join("fetched.preserves")),
            ledger: None,
            expected_bundle_ref: Some(bundle_ref),
            peer: "peer:local".to_string(),
            receipt_out: Some(dir.join("fetch.preserves")),
            failure_out: None,
        })
        .expect("fetch repro");
    }

    fn retention_cli_args(label: &str) -> RetentionEvidenceArgs {
        RetentionEvidenceArgs {
            requester_ref: Some(test_ref(&format!("retention-requester-{label}"))),
            policy_refs: vec![test_ref(&format!("retention-policy-{label}"))],
            authority_refs: vec![test_ref(&format!("retention-authority-{label}"))],
            evidence_refs: vec![test_ref(&format!("retention-evidence-{label}"))],
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs: Vec::new(),
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    #[derive(Clone, Copy)]
    struct RetentionCliObject<'a> {
        root: &'a std::path::Path,
        label: &'a str,
        object_ref: &'a str,
        object_kind: &'a str,
        retention_class: &'a str,
        action: &'a str,
    }

    fn retention_cli_args_for_object(input: RetentionCliObject<'_>) -> RetentionEvidenceArgs {
        let requester_ref = test_ref(&format!("retention-requester-{}", input.label));
        let policy_refs = vec![store_cli_admission(
            input,
            retention::ADMISSION_KIND_POLICY,
            &requester_ref,
        )];
        let authority_refs = vec![store_cli_admission(
            input,
            retention::ADMISSION_KIND_AUTHORITY,
            &requester_ref,
        )];
        let evidence_refs = vec![store_cli_admission(
            input,
            retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
            &requester_ref,
        )];
        let reference_index_refs = vec![store_cli_admission(
            input,
            retention::ADMISSION_KIND_REFERENCE_INDEX,
            &requester_ref,
        )];
        RetentionEvidenceArgs {
            requester_ref: Some(requester_ref),
            policy_refs,
            authority_refs,
            evidence_refs,
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs,
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn retention_apply_ref(
        input: RetentionCliObject<'_>,
        subsystem: &str,
        retention_args: &RetentionEvidenceArgs,
    ) -> String {
        let evidence = retention_args.clone().into_retention_evidence();
        let plan = retention::store_retention_gc_plan(retention::RetentionGcPlanInput {
            root: input.root,
            subsystem,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            evidence: &evidence,
        })
        .expect("store CLI retention GC plan");
        retention::apply_retention_gc_plan(retention::RetentionGcApplyFromPlanInput {
            root: input.root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply CLI retention GC plan")
        .apply_ref
    }

    fn store_cli_admission(input: RetentionCliObject<'_>, kind: &str, requester_ref: &str) -> String {
        retention::store_retention_evidence_admission(input.root, &retention::RetentionEvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref: input.object_ref,
            object_kind: input.object_kind,
            retention_class: input.retention_class,
            action: input.action,
            bound_refs: &[test_ref(&format!("{kind}-{}", input.label))],
            retained_refs: &[],
            remote_refs: &[],
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store cli retention admission")
        .admission_ref
    }

    fn only_blob_ref(iroh_store: &std::path::Path) -> String {
        let mut refs = fs::read_dir(iroh_store.join("blobs"))
            .expect("read iroh blobs")
            .map(|entry| {
                let file_name = entry.expect("blob entry").file_name().to_string_lossy().into_owned();
                let hex = file_name
                    .strip_prefix("blake3_")
                    .and_then(|name| name.strip_suffix(".bin"))
                    .expect("blob file name");
                format!("blake3:{hex}")
            })
            .collect::<Vec<_>>();
        refs.sort();
        assert_eq!(refs.len(), 1);
        refs.remove(0)
    }

    fn test_ref(label: &str) -> String {
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("test-ref", vec![
            molten::preserves_rail::string(label),
        ]))
        .expect("test ref")
    }

    fn cleanup_stale_molten_temp_dirs() {
        static CLEAN_STALE_TEMP_DIRS: std::sync::Once = std::sync::Once::new();
        CLEAN_STALE_TEMP_DIRS.call_once(|| {
            let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
                return;
            };
            for entry_result in entries {
                let Ok(entry) = entry_result else {
                    continue;
                };
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    let file_name = entry.file_name();
                    let Some(name) = file_name.to_str() else {
                        continue;
                    };
                    if is_stale_molten_temp_dir(name) {
                        let remove_result = fs::remove_dir_all(entry.path());
                        if remove_result.is_err() {
                            continue;
                        }
                    }
                }
            }
        });
    }

    fn is_stale_molten_temp_dir(name: &str) -> bool {
        name.starts_with("molten-") && live_process_token_count(name) == 0
    }

    fn live_process_token_count(name: &str) -> usize {
        let current_pid = u64::from(std::process::id());
        name.split('-')
            .filter_map(|token| token.parse::<u64>().ok())
            .filter(|pid| *pid == current_pid || std::path::Path::new("/proc").join(pid.to_string()).exists())
            .count()
    }

    fn install_cli_stage_artifact(registry: &Path, operation: &str) -> String {
        let payload = job_dag::builtin_stage_operation_value(operation).expect("stage operation");
        let installed = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload,
            schema_refs: vec![cli_synthetic_ref("job-worker-stage-schema").expect("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![cli_synthetic_ref("job-worker-stage-policy").expect("policy")],
            evidence_refs: vec![cli_synthetic_ref("job-worker-stage-evidence").expect("evidence")],
            installer_ref: cli_synthetic_ref("job-worker-stage-installer").expect("installer"),
            capability_refs: vec![cli_synthetic_ref("job-worker-stage-capability").expect("capability")],
        })
        .expect("install stage artifact");
        assert_eq!(installed.decision, "pass");
        installed.artifact_ref
    }

    fn install_cli_clean_octet_gate(registry: &Path) -> String {
        let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean octet gate");
        let gate_ref = canonical_hash(&gate_value).expect("gate ref");
        let installed = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![cli_synthetic_ref("job-worker-octet-policy").expect("policy")],
            evidence_refs: vec![cli_synthetic_ref("job-worker-octet-evidence").expect("evidence")],
            installer_ref: cli_synthetic_ref("job-worker-octet-installer").expect("installer"),
            capability_refs: vec![cli_synthetic_ref("job-worker-octet-capability").expect("capability")],
        })
        .expect("install octet gate");
        assert_eq!(installed.decision, "pass");
        gate_ref
    }

    fn install_cli_job_execute_authority_context(registry: &Path, job_ref: &str) -> String {
        let subject_ref = cli_synthetic_ref("job-worker-target-subject").expect("subject");
        let context_value = authority::authority_context_value(authority::ContextValueInput {
            subject_ref: &subject_ref,
            capabilities: &[authority::AuthorityCapability {
                capability: "job:execute".to_string(),
                scope: job_ref.to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[cli_synthetic_ref("job-worker-authority-policy").expect("policy")],
            evidence_refs: &[cli_synthetic_ref("job-worker-authority-evidence").expect("evidence")],
        })
        .expect("authority context");
        let context_ref = canonical_hash(&context_value).expect("authority context ref");
        let installed = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![cli_synthetic_ref("job-worker-authority-install-policy").expect("policy")],
            evidence_refs: vec![cli_synthetic_ref("job-worker-authority-install-evidence").expect("evidence")],
            installer_ref: cli_synthetic_ref("job-worker-authority-installer").expect("installer"),
            capability_refs: vec![cli_synthetic_ref("job-worker-authority-install-capability").expect("capability")],
        })
        .expect("install authority context");
        assert_eq!(installed.decision, "pass");
        context_ref
    }

    fn temp_dir(label: &str) -> PathBuf {
        cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
