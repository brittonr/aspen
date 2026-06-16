use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
#[cfg(test)]
use molten::artifacts;
#[cfg(test)]
use molten::chunk_store;
use molten::coordination;
use molten::deterministic_replay;
use molten::error::MoltenError;
use molten::error::Result;
#[cfg(test)]
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
use molten::harness::failure_summary;
use molten::harness::failure_value;
use molten::harness::gate_check_value;
use molten::harness::gate_receipt_summary;
use molten::harness::gate_receipt_value;
use molten::harness::replay_report_value;
use molten::harness::report_failure_value;
use molten::harness::report_summary;
use molten::harness::repro_bundle_summary;
use molten::harness::repro_verify_receipt_summary;
use molten::harness::run_suite_value;
use molten::harness::suite_failure_value;
use molten::harness::validate_report_value;
use molten::iroh_exchange::FetchChainSegmentInput;
use molten::iroh_exchange::PublishChainSegmentInput;
use molten::iroh_exchange::fetch_chain_segment;
use molten::iroh_exchange::publish_chain_segment;
#[cfg(test)]
use molten::job_dag;
use molten::ledger;
#[cfg(test)]
use molten::octet_gate;
use molten::operator_dogfood;
#[cfg(test)]
use molten::plugin_host;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
#[cfg(test)]
use molten::provenance;
use molten::raft_control_plane;
use molten::remote_dataspace;
use molten::retention;
#[cfg(test)]
use molten::schema_identity;
use molten::secrets;
use molten::service_runtime;
use molten::service_supervision;
#[cfg(test)]
use molten::typed_storage;
#[cfg(test)]
use molten::upgrades;

mod cli_artifact;
mod cli_cache;
mod cli_catalog;
mod cli_chunk;
mod cli_delivery;
mod cli_job;
mod cli_node;
mod cli_octet;
mod cli_plugin;
mod cli_protocol;
mod cli_provenance;
mod cli_repro;
mod cli_retention;
mod cli_rewrite;
mod cli_schema;
mod cli_secrets;
mod cli_storage;
mod cli_transcript;
mod cli_upgrade;

const COORDINATION_CLI_BATCH_REF_LIMIT: usize = 4096;
const COORDINATION_CLI_BATCH_EVIDENCE_LIMIT: usize = 16384;
const PROVENANCE_CLI_EVIDENCE_LIMIT: usize = 64;
const SIGNED_KEYRING_CLI_ENTRY_LIMIT: usize = 4096;
const _: () = assert!(COORDINATION_CLI_BATCH_REF_LIMIT <= 100_000);
const _: () = assert!(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT <= 100_000);
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
        command: cli_node::NodeCommand,
    },
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RuntimeCommand {
    Config {
        #[arg(long)]
        config: PathBuf,
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
    ReplayFixture {
        #[command(subcommand)]
        command: ReplayFixtureCommand,
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
        command: cli_chunk::ChunkCommand,
    },
    Storage {
        #[command(subcommand)]
        command: cli_storage::StorageCommand,
    },
    Artifact {
        #[command(subcommand)]
        command: cli_artifact::ArtifactCommand,
    },
    Schema {
        #[command(subcommand)]
        command: cli_schema::SchemaCommand,
    },
    Cache {
        #[command(subcommand)]
        command: cli_cache::CacheCommand,
    },
    Upgrade {
        #[command(subcommand)]
        command: cli_upgrade::UpgradeCommand,
    },
    Transcript {
        #[command(subcommand)]
        command: cli_transcript::TranscriptCommand,
    },
    Rewrite {
        #[command(subcommand)]
        command: cli_rewrite::RewriteCommand,
    },
    Catalog {
        #[command(subcommand)]
        command: cli_catalog::CatalogCommand,
    },
    Job {
        #[command(subcommand)]
        command: cli_job::JobCommand,
    },
    Remote {
        #[command(subcommand)]
        command: RemoteCommand,
    },
    Delivery {
        #[command(subcommand)]
        command: cli_delivery::DeliveryCommand,
    },
    Retention {
        #[command(subcommand)]
        command: cli_retention::RetentionCommand,
    },
    Provenance {
        #[command(subcommand)]
        command: cli_provenance::ProvenanceCommand,
    },
    Protocol {
        #[command(subcommand)]
        command: cli_protocol::ProtocolCommand,
    },
    Raft {
        #[command(subcommand)]
        command: RaftCommand,
    },
    Plugin {
        #[command(subcommand)]
        command: cli_plugin::PluginCommand,
    },
    Coordination {
        #[command(subcommand)]
        command: CoordinationCommand,
    },
    Secrets {
        #[command(subcommand)]
        command: cli_secrets::SecretsCommand,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    Vat {
        #[command(subcommand)]
        command: VatCommand,
    },
    Octet {
        #[command(subcommand)]
        command: cli_octet::OctetCommand,
    },
    Node {
        #[command(subcommand)]
        command: cli_node::NodeCommand,
    },
    Repro {
        #[command(subcommand)]
        command: cli_repro::ReproCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ReplayFixtureCommand {
    Record {
        #[arg(long)]
        out: PathBuf,
    },
    Verify {
        fixture: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Tamper {
        fixture: PathBuf,
        #[arg(long, default_value = "effect-response")]
        kind: String,
        #[arg(long)]
        out: PathBuf,
    },
    Rollup {
        #[arg(long = "receipt")]
        receipts: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    Index {
        #[arg(long = "receipt")]
        receipts: Vec<PathBuf>,
        #[arg(long = "rollup")]
        rollups: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        report: PathBuf,
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
enum VatCommand {
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    SnapshotFixture {
        #[arg(long)]
        out: PathBuf,
    },
    RestoreFixture {
        #[arg(long)]
        out: PathBuf,
    },
    PromiseFixture {
        #[arg(long)]
        out: PathBuf,
    },
    AmbientAuthorityFixture {
        #[arg(long)]
        out: PathBuf,
    },
    RightsFixture {
        #[arg(long)]
        out: PathBuf,
    },
    DistributedRefFixture {
        #[arg(long)]
        out: PathBuf,
    },
    TimeTravelFixture {
        #[arg(long)]
        out: PathBuf,
    },
    ReplayFixture {
        #[arg(long)]
        out: PathBuf,
    },
    AuthorityGraphFixture {
        #[arg(long)]
        out: PathBuf,
    },
    PortableStorageFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        report: PathBuf,
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
        #[arg(long)]
        replay_verify_out: Option<PathBuf>,
        #[arg(long)]
        replay_index_out: Option<PathBuf>,
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
pub(crate) struct RetentionEvidenceArgs {
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
    pub(crate) fn into_retention_evidence(self) -> retention::DestructiveRetentionEvidence {
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
        Some(Command::Node { command }) => cli_node::run_node_command(command),
        Some(Command::Runtime { command }) => run_runtime_command(command),
    }
}

fn run_runtime_command(command: RuntimeCommand) -> Result<()> {
    match command {
        RuntimeCommand::Config { config } => {
            let source = fs::read_to_string(&config).map_err(MoltenError::from)?;
            let startup = molten::runtime::RuntimeStartupConfig::from_nickel_export_json(&source)?;
            println!(
                "runtime config ok source=nickel actors={} subscriptions={}",
                startup.actors.len(),
                startup.subscriptions.len()
            );
            Ok(())
        }
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
        TestCommand::ReplayFixture { command } => run_replay_fixture_command(command),
        TestCommand::Report { command } => run_report_command(command),
        TestCommand::Gate { command } => run_gate_command(command),
        TestCommand::Receipt { command } => run_receipt_command(command),
        TestCommand::Ledger { command } => run_ledger_command(command),
        TestCommand::Chain { command } => run_chain_command(command),
        TestCommand::Chunk { command } => cli_chunk::run_chunk_command(command),
        TestCommand::Storage { command } => cli_storage::run_storage_command(command),
        TestCommand::Artifact { command } => cli_artifact::run_artifact_command(command),
        TestCommand::Schema { command } => cli_schema::run_schema_command(command),
        TestCommand::Cache { command } => cli_cache::run_cache_command(command),
        TestCommand::Upgrade { command } => cli_upgrade::run_upgrade_command(command),
        TestCommand::Transcript { command } => cli_transcript::run_transcript_command(command),
        TestCommand::Rewrite { command } => cli_rewrite::run_rewrite_command(command),
        TestCommand::Catalog { command } => cli_catalog::run_catalog_command(command),
        TestCommand::Job { command } => cli_job::run_job_command(command),
        TestCommand::Remote { command } => run_remote_command(command),
        TestCommand::Delivery { command } => cli_delivery::run_delivery_command(command),
        TestCommand::Retention { command } => cli_retention::run_retention_command(command),
        TestCommand::Provenance { command } => cli_provenance::run_provenance_command(command),
        TestCommand::Protocol { command } => cli_protocol::run_protocol_command(command),
        TestCommand::Raft { command } => run_raft_command(command),
        TestCommand::Plugin { command } => cli_plugin::run_plugin_command(command),
        TestCommand::Coordination { command } => run_coordination_command(command),
        TestCommand::Secrets { command } => cli_secrets::run_secrets_command(command),
        TestCommand::Service { command } => run_service_command(command),
        TestCommand::Vat { command } => run_vat_command(command),
        TestCommand::Octet { command } => cli_octet::run_octet_command(command),
        TestCommand::Node { command } => cli_node::run_node_command(command),
        TestCommand::Repro { command } => cli_repro::run_repro_command(command),
    }
}

fn run_replay_fixture_command(command: ReplayFixtureCommand) -> Result<()> {
    match command {
        ReplayFixtureCommand::Record { out } => {
            let fixture = deterministic_replay::record_fixture_value()?;
            write_file(&out, &to_text(&fixture.value)?)?;
            println!(
                "deterministic replay fixture written to {} ref={} identity={} final_state={}",
                out.display(),
                fixture.record_ref,
                fixture.identity_ref,
                fixture.final_state_ref
            );
            Ok(())
        }
        ReplayFixtureCommand::Verify { fixture, receipt_out } => {
            read_preserves_file(&fixture)?;
            let receipt =
                deterministic_replay::verify_fixture_value(deterministic_replay::ReplayFixtureVariant::Baseline)?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "deterministic replay verify ref={} decision={} divergence={}",
                    receipt.receipt_ref,
                    receipt.decision,
                    receipt.divergence.as_str()
                ),
            );
            Ok(())
        }
        ReplayFixtureCommand::Tamper { fixture, kind, out } => {
            read_preserves_file(&fixture)?;
            let variant = replay_fixture_variant_from_kind(&kind)?;
            let receipt = deterministic_replay::verify_fixture_value(variant)?;
            write_file(&out, &to_text(&receipt.value)?)?;
            println!(
                "deterministic replay tamper receipt written to {} ref={} divergence={}",
                out.display(),
                receipt.receipt_ref,
                receipt.divergence.as_str()
            );
            Ok(())
        }
        ReplayFixtureCommand::Rollup { receipts, out } => {
            let mut inputs = Vec::with_capacity(receipts.len());
            for receipt in receipts {
                let value = read_preserves_file(&receipt)?;
                inputs.push(deterministic_replay::ReplayRollupInput {
                    expected_ref: Some(canonical_hash(&value)?),
                    value,
                });
            }
            let rollup = deterministic_replay::rollup_replay_receipts(&inputs)?;
            write_file(&out, &to_text(&rollup.value)?)?;
            println!(
                "deterministic replay rollup written to {} ref={} decision={} total={} pass={} deny={}",
                out.display(),
                rollup.rollup_ref,
                rollup.decision,
                rollup.total_count,
                rollup.pass_count,
                rollup.deny_count
            );
            Ok(())
        }
        ReplayFixtureCommand::Index { receipts, rollups, out } => {
            let mut inputs = Vec::with_capacity(receipts.len() + rollups.len());
            for receipt in receipts {
                let value = read_preserves_file(&receipt)?;
                inputs.push(deterministic_replay::ReplayIndexInput {
                    expected_ref: Some(canonical_hash(&value)?),
                    value,
                });
            }
            for rollup in rollups {
                let value = read_preserves_file(&rollup)?;
                inputs.push(deterministic_replay::ReplayIndexInput {
                    expected_ref: Some(canonical_hash(&value)?),
                    value,
                });
            }
            let index = deterministic_replay::index_replay_evidence(&inputs)?;
            write_file(&out, &to_text(&index.value)?)?;
            println!(
                "deterministic replay index written to {} ref={} decision={} total={} pass={} deny={} raw_receipts={} rollups={}",
                out.display(),
                index.index_ref,
                index.decision,
                index.total_count,
                index.pass_count,
                index.deny_count,
                index.raw_receipt_count,
                index.rollup_count
            );
            Ok(())
        }
        ReplayFixtureCommand::Show { report } => {
            let value = read_preserves_file(&report)?;
            let reference = canonical_hash(&value)?;
            println!("deterministic replay artifact ref={reference}");
            println!("{}", to_text(&value)?);
            Ok(())
        }
    }
}

fn replay_fixture_variant_from_kind(kind: &str) -> Result<deterministic_replay::ReplayFixtureVariant> {
    match kind {
        "identity" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedIdentity),
        "scheduler" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedScheduler),
        "input" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedInput),
        "effect-request" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedEffectRequest),
        "effect-response" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedEffectResponse),
        "policy" | "policy-decision" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedPolicyDecision),
        "action" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedAction),
        "receipt" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedReceipt),
        "output" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedOutput),
        "state" | "state-hash" => Ok(deterministic_replay::ReplayFixtureVariant::ChangedStateHash),
        "live-effect" | "missing-effect" => Ok(deterministic_replay::ReplayFixtureVariant::MissingRecordedEffect),
        _ => Err(MoltenError::invalid_harness(format!("unsupported replay fixture tamper kind {kind}"))),
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

fn run_vat_command(command: VatCommand) -> Result<()> {
    match command {
        VatCommand::RunFixture { out } => {
            let run = molten::runtime::run_vat_fixture()?;
            write_file(&out, &to_text(&run.value)?)?;
            println!("vat fixture run: {}", run.run_ref);
            Ok(())
        }
        VatCommand::SnapshotFixture { out } => {
            let snapshot = molten::runtime::run_vat_snapshot_fixture()?;
            write_file(&out, &to_text(&snapshot.value)?)?;
            println!("vat snapshot fixture: {}", snapshot.fixture_ref);
            Ok(())
        }
        VatCommand::RestoreFixture { out } => {
            let restore = molten::runtime::run_vat_restore_fixture()?;
            write_file(&out, &to_text(&restore.value)?)?;
            println!("vat restore fixture: {}", restore.fixture_ref);
            Ok(())
        }
        VatCommand::PromiseFixture { out } => {
            let promise = molten::runtime::run_vat_promise_fixture()?;
            write_file(&out, &to_text(&promise.value)?)?;
            println!("vat promise fixture: {}", promise.fixture_ref);
            Ok(())
        }
        VatCommand::AmbientAuthorityFixture { out } => {
            let authority = molten::runtime::run_vat_ambient_authority_fixture()?;
            write_file(&out, &to_text(&authority.value)?)?;
            println!("vat ambient authority fixture: {}", authority.fixture_ref);
            Ok(())
        }
        VatCommand::RightsFixture { out } => {
            let rights = molten::runtime::run_vat_rights_fixture()?;
            write_file(&out, &to_text(&rights.value)?)?;
            println!("vat rights fixture: {}", rights.fixture_ref);
            Ok(())
        }
        VatCommand::DistributedRefFixture { out } => {
            let distributed_ref = molten::runtime::run_vat_distributed_ref_fixture()?;
            write_file(&out, &to_text(&distributed_ref.value)?)?;
            println!("vat distributed ref fixture: {}", distributed_ref.fixture_ref);
            Ok(())
        }
        VatCommand::TimeTravelFixture { out } => {
            let debug = molten::runtime::run_vat_time_travel_fixture()?;
            write_file(&out, &to_text(&debug.value)?)?;
            println!("vat time travel fixture: {}", debug.fixture_ref);
            Ok(())
        }
        VatCommand::ReplayFixture { out } => {
            let replay = molten::runtime::run_vat_replay_fixture()?;
            write_file(&out, &to_text(&replay.value)?)?;
            println!("vat replay fixture: {}", replay.fixture_ref);
            Ok(())
        }
        VatCommand::AuthorityGraphFixture { out } => {
            let graph = molten::runtime::run_vat_authority_graph_fixture()?;
            write_file(&out, &to_text(&graph.value)?)?;
            println!("vat authority graph fixture: {}", graph.fixture_ref);
            Ok(())
        }
        VatCommand::PortableStorageFixture { out } => {
            let storage = molten::runtime::run_vat_portable_storage_fixture()?;
            write_file(&out, &to_text(&storage.value)?)?;
            println!("vat portable storage fixture: {}", storage.fixture_ref);
            Ok(())
        }
        VatCommand::Show { report } => {
            let value = read_preserves_file(&report)?;
            println!("{}", molten::runtime::vat_fixture_summary(&value)?);
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
            replay_verify_out,
            replay_index_out,
        } => {
            let run = operator_dogfood::run_local_node_dogfood(&operator_dogfood::LocalNodeDogfoodInput {
                state_root: &state_root,
            })?;
            write_file(&out, &to_text(&run.report_value)?)?;
            if let (Some(path), Some(value)) = (release_gate_out.as_ref(), run.release_gate_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            if let (Some(path), Some(value)) = (replay_verify_out.as_ref(), run.replay_verify_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            if let (Some(path), Some(value)) = (replay_index_out.as_ref(), run.replay_index_value.as_ref()) {
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
    use molten::catalog_mcp;
    use molten::delivery_idempotency;
    use molten::harness::parse_failure;
    use molten::harness::parse_repro_bundle;
    use molten::protocol_session;

    use super::*;
    use crate::cli_artifact::ArtifactCommand;
    use crate::cli_artifact::run_artifact_command;
    use crate::cli_cache::CacheCommand;
    use crate::cli_cache::run_cache_command;
    use crate::cli_catalog::CatalogCommand;
    use crate::cli_catalog::run_catalog_command;
    use crate::cli_chunk::ChunkCommand;
    use crate::cli_chunk::run_chunk_command;
    use crate::cli_delivery::DeliveryCommand;
    use crate::cli_delivery::run_delivery_command;
    use crate::cli_job::JobCommand;
    use crate::cli_job::run_job_command;
    use crate::cli_plugin::PluginCommand;
    use crate::cli_plugin::run_plugin_command;
    use crate::cli_protocol::ProtocolCommand;
    use crate::cli_protocol::run_protocol_command;
    use crate::cli_provenance::ProvenanceCommand;
    use crate::cli_provenance::run_provenance_command;
    use crate::cli_repro::ReproCommand;
    use crate::cli_repro::run_repro_command;
    use crate::cli_retention::RetentionCommand;
    use crate::cli_retention::run_retention_command;
    use crate::cli_rewrite::RewriteCommand;
    use crate::cli_rewrite::run_rewrite_command;
    use crate::cli_schema::SchemaCommand;
    use crate::cli_schema::run_schema_command;
    use crate::cli_secrets::SecretsCommand;
    use crate::cli_secrets::run_secrets_command;
    use crate::cli_storage::StorageCommand;
    use crate::cli_storage::run_storage_command;
    use crate::cli_transcript::TranscriptCommand;
    use crate::cli_transcript::run_transcript_command;
    use crate::cli_upgrade::UpgradeCommand;
    use crate::cli_upgrade::run_upgrade_command;

    #[test]
    fn runtime_config_command_accepts_typed_config_path() {
        let dir = temp_dir("runtime-config");
        let config = dir.join("runtime.json");
        write_file(
            &config,
            r#"{
                "source_language": "nickel",
                "actors": [{ "id": "actor:consumer", "kind": "native" }],
                "subscriptions": [{ "actor": "actor:consumer", "subject_preserves": "\"service.ready\"" }]
            }"#,
        )
        .expect("write config");

        run_runtime_command(RuntimeCommand::Config { config }).expect("runtime config command");
    }

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
            chunks: None,
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
        let replay_verify = dir.join("replay-verify.preserves");
        let replay_index = dir.join("replay-evidence-index.preserves");
        run_dogfood_command(DogfoodCommand::LocalNode {
            state_root: state_root.clone(),
            out: report.clone(),
            release_gate_out: Some(release_gate.clone()),
            replay_verify_out: Some(replay_verify.clone()),
            replay_index_out: Some(replay_index.clone()),
        })
        .expect("dogfood local node");
        let report_value = read_preserves_file(&report).expect("read dogfood report");
        let parsed = operator_dogfood::parse_dogfood_report(&report_value).expect("parse dogfood report");
        assert_eq!(parsed.decision, "pass");
        assert!(fs::read_to_string(&release_gate).expect("read release gate").contains("release-gate-receipt-v1"));
        assert!(
            fs::read_to_string(&replay_verify)
                .expect("read replay verify")
                .contains("deterministic-replay-verify-v1")
        );
        assert!(
            fs::read_to_string(&replay_index)
                .expect("read replay index")
                .contains("deterministic-replay-index-v1")
        );
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
    fn cli_upgrade_protocol_drain_task_gates_on_ledger_protocol_evidence() {
        let dir = temp_dir("upgrade-cli-protocol-drain");
        let ledger_root = dir.join("ledger");
        let store = dir.join("upgrades");
        let lifecycle = protocol_session::request_response_lifecycle().expect("protocol lifecycle");
        let gate = protocol_session::gate_protocol_session_lifecycle(protocol_session::ProtocolSessionGateInput {
            install_receipt: lifecycle.install.value.clone(),
            initial_states: lifecycle.initial_states.iter().map(|state| state.value.clone()).collect(),
            operation_receipts: lifecycle.operations.iter().map(|operation| operation.receipt.value.clone()).collect(),
            messages: lifecycle
                .operations
                .iter()
                .filter_map(|operation| operation.message.as_ref().map(|message| message.value.clone()))
                .collect(),
            next_states: lifecycle
                .operations
                .iter()
                .filter_map(|operation| operation.next_state.as_ref().map(|state| state.value.clone()))
                .collect(),
        })
        .expect("protocol gate");
        let gate_ref = ledger::import_artifact(&ledger_root, &gate.value).expect("import protocol gate").artifact_ref;
        let old_protocol_ref = gate.protocol_ref.clone();
        let new_protocol_ref = test_ref("cli-protocol-v2");
        let plan_value = upgrades::upgrade_plan_value(&upgrades::UpgradePlanInput {
            session_id: "cli-protocol-drain".to_string(),
            reason: "protocol drain".to_string(),
            summary: "drain protocol sessions before name cutover".to_string(),
            initiator_ref: test_ref("upgrade-initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old_protocol_ref.clone(), new_protocol_ref.clone()],
            impact_refs: vec![old_protocol_ref.clone()],
            tasks: vec![upgrades::UpgradeTaskInput {
                task_id: "drain-sessions".to_string(),
                kind: "drain-sessions".to_string(),
                subject: "request-response-protocol".to_string(),
                from_ref: Some(old_protocol_ref.clone()),
                to_ref: Some(new_protocol_ref.clone()),
                precondition_refs: vec![gate_ref],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: upgrades::UpgradeCompatibilityWindow {
                old_refs: vec![old_protocol_ref.clone()],
                new_refs: vec![new_protocol_ref.clone()],
                expires_at: Some(64),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![test_ref("rollback")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("upgrade-evidence")],
            source_gate_receipt_values: vec![
                octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"),
            ],
        })
        .expect("protocol drain plan");
        let plan_file = dir.join("protocol-drain-plan.preserves");
        write_file(&plan_file, &to_text(&plan_value).expect("plan text")).expect("write protocol drain plan");
        let plan = upgrades::parse_upgrade_plan(&plan_value).expect("parse plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: plan_file,
            store: store.clone(),
            receipt_out: Some(dir.join("protocol-drain-create.preserves")),
        })
        .expect("create protocol drain session");
        let receipt_out = dir.join("protocol-drain-task.preserves");
        run_upgrade_command(UpgradeCommand::RunTask {
            store,
            ledger: ledger_root,
            plan_ref: plan.plan_ref,
            task_id: "drain-sessions".to_string(),
            receipt_out: Some(receipt_out.clone()),
        })
        .expect("run protocol drain task");
        let receipt = upgrades::parse_upgrade_receipt(&read_preserves_file(&receipt_out).expect("read receipt"))
            .expect("parse receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(to_text(&receipt.value).expect("receipt text").contains("protocol-session-drain"));

        let missing_store = dir.join("missing-upgrades");
        let missing_gate_ref = test_ref("cli-missing-protocol-gate");
        let missing_plan = upgrades::upgrade_plan_value(&upgrades::UpgradePlanInput {
            session_id: "cli-protocol-drain-missing".to_string(),
            reason: "protocol drain".to_string(),
            summary: "missing protocol gate evidence denies".to_string(),
            initiator_ref: test_ref("upgrade-initiator"),
            capability_refs: vec![test_ref("upgrade-capability")],
            affected_refs: vec![old_protocol_ref.clone(), new_protocol_ref.clone()],
            impact_refs: vec![old_protocol_ref.clone()],
            tasks: vec![upgrades::UpgradeTaskInput {
                task_id: "drain-sessions".to_string(),
                kind: "drain-sessions".to_string(),
                subject: "request-response-protocol".to_string(),
                from_ref: Some(old_protocol_ref),
                to_ref: Some(new_protocol_ref),
                precondition_refs: vec![missing_gate_ref],
                postcondition_refs: Vec::new(),
                reversible: false,
            }],
            compatibility: upgrades::UpgradeCompatibilityWindow {
                old_refs: vec![test_ref("compat-old-protocol")],
                new_refs: vec![test_ref("compat-new-protocol")],
                expires_at: Some(64),
                policy_refs: vec![test_ref("compat-policy")],
            },
            rollback_refs: vec![test_ref("rollback")],
            policy_refs: vec![test_ref("upgrade-policy")],
            evidence_refs: vec![test_ref("upgrade-evidence")],
            source_gate_receipt_values: vec![
                octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("source gate fixture"),
            ],
        })
        .expect("missing protocol drain plan");
        let missing_plan_file = dir.join("protocol-drain-missing-plan.preserves");
        write_file(&missing_plan_file, &to_text(&missing_plan).expect("missing plan text"))
            .expect("write missing plan");
        let missing_plan = upgrades::parse_upgrade_plan(&missing_plan).expect("parse missing plan");
        run_upgrade_command(UpgradeCommand::Create {
            plan: missing_plan_file,
            store: missing_store.clone(),
            receipt_out: Some(dir.join("protocol-drain-missing-create.preserves")),
        })
        .expect("create missing protocol drain session");
        let missing_receipt_out = dir.join("protocol-drain-missing-task.preserves");
        run_upgrade_command(UpgradeCommand::RunTask {
            store: missing_store,
            ledger: dir.join("ledger"),
            plan_ref: missing_plan.plan_ref,
            task_id: "drain-sessions".to_string(),
            receipt_out: Some(missing_receipt_out.clone()),
        })
        .expect("run missing protocol drain task");
        let missing_receipt =
            upgrades::parse_upgrade_receipt(&read_preserves_file(&missing_receipt_out).expect("read missing receipt"))
                .expect("parse missing receipt");
        assert_eq!(missing_receipt.decision, "deny");
        assert!(to_text(&missing_receipt.value).expect("missing receipt text").contains("not readable from ledger"));
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
                molten::preserves_rail::content_ref_from_hex(hex).expect("canonical blob ref")
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
