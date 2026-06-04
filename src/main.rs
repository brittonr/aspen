use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use molten::artifacts;
use molten::catalog;
use molten::catalog_mcp;
use molten::chunk_store;
use molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
use molten::coordination;
use molten::error::MoltenError;
use molten::error::Result;
use molten::eval_cache;
use molten::evidence::PASS_EVIDENCE_PURPOSE;
use molten::evidence::SignReceiptInput;
use molten::evidence::sign_receipt;
use molten::evidence::signed_receipt_summary;
use molten::evidence::verify_signed_receipt;
use molten::evidence_chain::ChainForkPolicy;
use molten::evidence_chain::ChainScope;
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
use molten::harness::repro_verify_receipt_summary;
use molten::harness::repro_verify_receipt_value;
use molten::harness::run_suite_value;
use molten::harness::sealed_repro_bundle_value_with_command;
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
use molten::protocol_session;
use molten::provenance;
use molten::raft_control_plane;
use molten::remote_dataspace;
use molten::rewrites;
use molten::schema_identity;
use molten::secrets;
use molten::service_runtime;
use molten::service_supervision;
use molten::transcripts;
use molten::typed_storage;
use molten::upgrades;

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
    Show {
        artifact: PathBuf,
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
    },
}

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
        Some(Command::Node { command }) => run_node_command(command),
    }
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
        } => {
            let signed_value = read_preserves_file(&signed_receipt)?;
            let verified = verify_signed_receipt(&signed_value, &purpose, &trust_root, &key)?;
            println!(
                "signed receipt verify ok envelope={} subject={} signer={} purpose={}",
                verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
            );
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
            receipt_out,
        } => {
            let gc = ledger::gc(&ledger, dry_run)?;
            emit_named_receipt(receipt_out.as_ref(), "ledger gc receipt", &gc.receipt_value)?;
            println!("ledger gc ok dry_run={} removed={}", gc.dry_run, gc.removed_refs.len());
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
            receipt_out,
        } => {
            let gc = chunk_store::gc(&store, dry_run)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &gc.receipt_value)?;
            println!(
                "chunk gc ok dry_run={} removed_manifests={} removed_chunks={}",
                gc.dry_run,
                gc.removed_manifests.len(),
                gc.removed_chunks.len()
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
            })?;
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &invalidated.receipt_value)?;
            for key_ref in &invalidated.invalidated_key_refs {
                println!("{key_ref}");
            }
            eprintln!("cache invalidate ok keys={} cache={}", invalidated.invalidated_key_refs.len(), cache.display());
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
            let request = job_sync_cli_request(&source_registry, &job, &stages, &target_peer)?;
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
            plan_out,
            receipt_out,
        } => {
            let request = job_sync_cli_request(&source_registry, &job, &stages, &target_peer)?;
            let synced = job_dag::sync_loopback(&source_registry, &target_registry, &request)?;
            emit_job_analysis(&synced.plan.value, plan_out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job sync receipt", &synced.receipt_value)?;
            eprintln!(
                "job sync-loopback ok job={} installed={} already_present={}",
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
        JobCommand::Status { ledger, job } => {
            for entry in ledger::list_artifacts(&ledger)? {
                if entry.artifact_kind != "job-dag-receipt" {
                    continue;
                }
                let value = ledger::read_artifact(&ledger, &entry.artifact_ref)?;
                let receipt = job_dag::parse_job_receipt(&value)?;
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
) -> Result<preserves::IOValue> {
    let dag = job_dag::read_job_dag_file_or_registry(source_registry, job)?;
    job_dag::job_sync_request_value(job_dag::SyncRequestValueInput {
        job_ref: &dag.job_ref,
        stage_ids: stages,
        target_peer,
        policy_refs: &[cli_job_ref("sync-policy", &dag.job_ref)?],
        capability_refs: &[cli_job_ref("sync-capability", &dag.job_ref)?],
        evidence_refs: &[cli_job_ref("sync-evidence", &dag.job_ref)?],
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

fn run_coordination_command(command: CoordinationCommand) -> Result<()> {
    match command {
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
            receipt_out,
        } => {
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
                    },
                ))?;
                if let Some(path) = service_receipt_out.as_ref() {
                    write_file(path, &to_text(&served.service.service_receipt_value)?)?;
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
            failure_out,
        } => {
            let artifact_value = read_preserves_file_with_failure(&report, failure_out.as_ref(), "export")?;
            let command = vec![
                "molten".to_string(),
                "test".to_string(),
                "repro".to_string(),
                "export".to_string(),
                report.display().to_string(),
                "--out".to_string(),
                out.display().to_string(),
            ];
            if parse_failure(&artifact_value).is_ok() {
                export_failure_repro(&artifact_value, &out, &command, failure_out.as_ref())
            } else {
                export_report_repro(&artifact_value, &out, &command, failure_out.as_ref())
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
            failure_out,
        } => {
            let bundle_value = read_preserves_file_with_failure(&bundle, failure_out.as_ref(), "unpack")?;
            unpack_report_repro(&bundle_value, &out, failure_out.as_ref())
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
    failure_out: Option<&PathBuf>,
) -> Result<()> {
    let bundle_value = match sealed_repro_bundle_value_with_command(report_value, command) {
        Ok(bundle_value) => bundle_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let gate_receipt_value = match parse_repro_bundle(&bundle_value).and_then(|bundle| {
        bundle
            .gate_receipt_value
            .ok_or_else(|| MoltenError::invalid_harness("sealed repro bundle missing embedded gate receipt"))
    }) {
        Ok(gate_receipt_value) => gate_receipt_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let suite_value = match report_suite_value(report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "export", &error, report_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("report.preserves"), &to_text(report_value)?)?;
        write_file(&out.join("suite.preserves"), &to_text(&suite_value)?)?;
        write_file(&out.join("summary.txt"), &report_summary(report_value)?)?;
        write_file(&out.join("commands.txt"), REPORT_REPRO_COMMANDS)?;
        write_file(&out.join("gate-receipt.preserves"), &to_text(&gate_receipt_value)?)?;
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

fn unpack_report_repro(bundle_value: &preserves::IOValue, out: &Path, failure_out: Option<&PathBuf>) -> Result<()> {
    let verify_receipt = match repro_verify_receipt_value(bundle_value) {
        Ok(receipt) => receipt,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let bundle = match parse_repro_bundle(bundle_value) {
        Ok(bundle) => bundle,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let report_value = match bundle.report_value {
        Some(report_value) => report_value,
        None => {
            let error = MoltenError::invalid_harness("sealed repro unpack requires an embedded report");
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let suite_value = match report_suite_value(&report_value) {
        Ok(suite_value) => suite_value,
        Err(error) => {
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let gate_receipt_value = match bundle.gate_receipt_value {
        Some(gate_receipt_value) => gate_receipt_value,
        None => {
            let error = MoltenError::invalid_harness("sealed repro unpack requires an embedded gate receipt");
            write_optional_artifact_failure(failure_out, "unpack", &error, bundle_value)?;
            return Err(error);
        }
    };
    let export = (|| -> Result<()> {
        fs::create_dir_all(out).map_err(MoltenError::from)?;
        write_file(&out.join("refs.preserves"), &to_text(bundle_value)?)?;
        write_file(&out.join("report.preserves"), &to_text(&report_value)?)?;
        write_file(&out.join("suite.preserves"), &to_text(&suite_value)?)?;
        write_file(&out.join("gate-receipt.preserves"), &to_text(&gate_receipt_value)?)?;
        write_file(&out.join("verify-receipt.preserves"), &to_text(&verify_receipt)?)?;
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

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
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
            failure_out: Some(unpack_failure.clone()),
        })
        .expect_err("failure repro unpack should fail");
        assert!(unpack_error.to_string().contains("diagnostic-only"));
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
        run_cache_command(CacheCommand::Invalidate {
            cache: cache.clone(),
            key_ref: None,
            dependency_ref: Some(dependency_ref),
            policy_ref: None,
            capability_ref: None,
            revocation_ref: None,
            operation: None,
            reason: "cli-test".to_string(),
            receipt_out: Some(dir.join("invalidate-receipt.preserves")),
        })
        .expect("cache invalidate");
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
        assert!(canonical_hash(&parsed_payload).expect("payload ref").starts_with("blake3:"));
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
        let source = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: None,
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
            stage_artifact_ref: None,
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
            stage_artifact_ref: None,
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
        let admit_plan_out = dir.join("job-admit-plan.preserves");
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
            plan_out: Some(dir.join("job-sync-loopback-plan.preserves")),
            receipt_out: Some(dir.join("job-sync-loopback-receipt.preserves")),
        })
        .expect("job sync loopback");
        assert!(fs::read_to_string(&sync_plan_out).expect("read sync plan").contains("job-sync-plan-v1"));
        assert!(
            !artifacts::list_artifacts(&target_registry, Some(job_dag::JOB_ARTIFACT_KIND))
                .expect("target jobs")
                .is_empty()
        );
        run_job_command(JobCommand::AdmitPlan {
            job: dag.job_ref.clone(),
            target_registry: target_registry.clone(),
            sync_ref: None,
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
            resource_refs: Vec::new(),
            out: Some(admit_plan_out.clone()),
            receipt_out: Some(dir.join("job-admit-plan-receipt.preserves")),
        })
        .expect("job admit plan");
        run_job_command(JobCommand::AdmitLoopback {
            job: dag.job_ref.clone(),
            target_registry: target_registry.clone(),
            sync_ref: None,
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
            resource_refs: Vec::new(),
            plan_out: Some(dir.join("job-admit-loopback-plan.preserves")),
            receipt_out: Some(dir.join("job-admit-loopback-receipt.preserves")),
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
            state_root,
            out: report.clone(),
            release_gate_out: Some(release_gate.clone()),
        })
        .expect("dogfood local node");
        let report_value = read_preserves_file(&report).expect("read dogfood report");
        let parsed = operator_dogfood::parse_dogfood_report(&report_value).expect("parse dogfood report");
        assert_eq!(parsed.decision, "pass");
        assert!(fs::read_to_string(&release_gate).expect("read release gate").contains("release-gate-receipt-v1"));
        run_dogfood_command(DogfoodCommand::Show { artifact: report }).expect("dogfood show report");
        run_dogfood_command(DogfoodCommand::Show { artifact: release_gate }).expect("dogfood show gate");
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
