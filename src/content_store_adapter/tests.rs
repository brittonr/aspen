use std::collections::BTreeMap;
use std::time::Duration;

use super::*;

const GENERATION_ONE: u64 = 1;
const SUBMITTED_TICK: u64 = 10;
const DEADLINE_TICK: u64 = 20;
const MAX_TOTAL_BYTES: u64 = 4_096;
const MAX_CHUNK_COUNT: usize = 32;
const MAX_CHUNK_BYTES: u64 = 512;
const MAX_RANGE_BYTES: u64 = 1_024;
const MAX_CONCURRENT_OPERATIONS: usize = 4;
const MAX_QUEUED_BYTES: u64 = 8_192;
const MAX_MEMORY_BYTES: u64 = 4_096;
const MAX_DEADLINE_TICKS: u64 = 20;
const MAX_RETRIES: u32 = 2;
const MAX_EVENTS: usize = 64;
const MAX_STATUS_ENTRIES: usize = 16;
const FIXTURE_CHUNK_BYTES: u64 = 4;
const LIVE_TIMEOUT_SECONDS: u64 = 10;
const CONTENT_PORT_COUNT: usize = 2;
const SIMULATION_TIMEOUT_LATENCY: u64 = DEADLINE_TICK - SUBMITTED_TICK + 1;

fn test_ref(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn bounds() -> ContentResourceBounds {
    ContentResourceBounds {
        max_total_bytes: MAX_TOTAL_BYTES,
        max_chunk_count: MAX_CHUNK_COUNT,
        max_chunk_bytes: MAX_CHUNK_BYTES,
        max_range_bytes: MAX_RANGE_BYTES,
        max_concurrent_operations: MAX_CONCURRENT_OPERATIONS,
        max_queued_bytes: MAX_QUEUED_BYTES,
        max_memory_bytes: MAX_MEMORY_BYTES,
        max_deadline_ticks: MAX_DEADLINE_TICKS,
        max_retries: MAX_RETRIES,
        max_events: MAX_EVENTS,
        max_status_entries: MAX_STATUS_ENTRIES,
    }
}

fn profile(class: ContentAdapterClass) -> ContentAdapterProfile {
    content_adapter_profile(
        &format!("{}-v1", class.as_str()),
        test_ref(&format!("{}-profile", class.as_str())),
        class,
        bounds(),
        vec![test_ref("profile-evidence")],
    )
    .expect("content profile")
}

fn command(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    operation: ContentOperation,
    range: Option<ContentRange>,
) -> ContentCommand {
    content_command(profile, ContentCommandInput {
        operation_ref: test_ref(&format!("{}-operation", operation.as_str())),
        operation,
        manifest,
        range,
        submitted_tick: SUBMITTED_TICK,
        deadline_tick: DEADLINE_TICK,
        retry_count: 0,
        cancelled: false,
        policy_refs: vec![test_ref("content-operation-policy")],
    })
    .expect("content command")
}

fn fixture_store(
    name: &str,
) -> (std::path::PathBuf, crate::chunk_store::CapabilityChunkRoot, ContentManifestDescriptor) {
    let workspace = temp_dir(name);
    let put = crate::chunk_store::put_bytes(&workspace, "artifact", b"aaaabbbbcccc", FIXTURE_CHUNK_BYTES)
        .expect("put fixture content");
    let root = crate::chunk_store::open_capability_chunk_root(&workspace).expect("open fixture root");
    let source_manifest =
        crate::chunk_store::read_manifest_with_root(&root, &put.manifest_ref).expect("read fixture manifest");
    let manifest = manifest_descriptor(&source_manifest);
    (workspace, root, manifest)
}

fn fixture_chunks(manifest: &ContentManifestDescriptor) -> BTreeMap<String, Vec<u8>> {
    manifest
        .chunks
        .iter()
        .zip([b"aaaa".to_vec(), b"bbbb".to_vec(), b"cccc".to_vec()])
        .map(|(descriptor, bytes)| (descriptor.chunk_ref.clone(), bytes))
        .collect()
}

// r[verify molten.content_store_adapter.port_contract]
#[test]
fn content_store_and_exchange_ports_are_exact_versioned_and_backend_neutral() {
    let profile = profile(ContentAdapterClass::CapabilityLocal);
    let descriptors = content_store_port_descriptors(&profile.profile_ref);
    let registry = crate::fabric::build_fabric_port_registry(&descriptors).expect("content port registry");
    assert_eq!(registry.descriptors().len(), CONTENT_PORT_COUNT);
    let mut malformed = descriptors;
    malformed[0].conformance_refs.clear();
    assert!(crate::fabric::build_fabric_port_registry(&malformed).is_err());
}

// r[verify molten.content_store_adapter.port_contract]
// r[verify molten.content_store_adapter.verify_before_available]
// r[verify molten.content_store_adapter.streaming_bounds]
#[test]
fn capability_local_stream_and_range_expose_only_verified_bounded_bytes() {
    let (workspace, root, manifest) = fixture_store("content-adapter-local");
    let profile = profile(ContentAdapterClass::CapabilityLocal);
    let get = command(&profile, &manifest, ContentOperation::Get, None);
    let execution = execute_local_stream_get(&profile, &root, &get, GENERATION_ONE, None).expect("local stream get");
    assert_eq!(execution.state.artifact.terminal, ContentTerminal::Durable);
    assert_eq!(
        assemble_verified_content(&manifest, &execution.state.artifact, &execution.verified_chunks).unwrap(),
        b"aaaabbbbcccc"
    );
    assert!(!execution.backend_hint_ref.contains(workspace.to_string_lossy().as_ref()));

    let range = ContentRange {
        offset: FIXTURE_CHUNK_BYTES,
        length: FIXTURE_CHUNK_BYTES,
    };
    let range_command = command(&profile, &manifest, ContentOperation::RangeRead, Some(range));
    assert_eq!(execute_local_verified_range(&profile, &root, &range_command).expect("range"), b"bbbb");

    let repeated_workspace = temp_dir("content-adapter-repeated-chunk");
    let repeated_put = crate::chunk_store::put_bytes(&repeated_workspace, "artifact", b"aaaaaaaa", FIXTURE_CHUNK_BYTES)
        .expect("put repeated chunks");
    let repeated_root = crate::chunk_store::open_capability_chunk_root(&repeated_workspace).expect("repeated root");
    let repeated_manifest = manifest_descriptor(
        &crate::chunk_store::read_manifest_with_root(&repeated_root, &repeated_put.manifest_ref)
            .expect("repeated manifest"),
    );
    assert_eq!(repeated_manifest.chunks[0].chunk_ref, repeated_manifest.chunks[1].chunk_ref);
    let repeated_command = command(&profile, &repeated_manifest, ContentOperation::Get, None);
    let repeated = execute_local_stream_get(&profile, &repeated_root, &repeated_command, GENERATION_ONE, None)
        .expect("repeated stream");
    assert_eq!(
        assemble_verified_content(&repeated_manifest, &repeated.state.artifact, &repeated.verified_chunks,)
            .expect("repeated assembly"),
        b"aaaaaaaa"
    );

    let destination = temp_dir("content-adapter-local-put");
    let destination_root = crate::chunk_store::open_capability_chunk_root(&destination).expect("destination root");
    let put_command = command(&profile, &manifest, ContentOperation::Put, None);
    let put =
        execute_local_stream_put(&profile, &destination_root, &put_command, &manifest, "artifact", b"aaaabbbbcccc")
            .expect("local stream put");
    assert_eq!(put.manifest.manifest_ref, manifest.manifest_ref);
    assert!(
        execute_local_stream_put(&profile, &destination_root, &put_command, &manifest, "artifact", b"corrupt",)
            .is_err()
    );
    std::fs::remove_dir_all(workspace).expect("remove local fixture");
    std::fs::remove_dir_all(repeated_workspace).expect("remove repeated fixture");
    std::fs::remove_dir_all(destination).expect("remove put fixture");
}

// r[verify molten.content_store_adapter.partial_state]
// r[verify molten.content_store_adapter.live_sim_conformance]
#[test]
fn deterministic_simulation_matches_verified_trace_and_models_failure_without_exposure() {
    let (workspace, root, manifest) = fixture_store("content-adapter-simulation");
    let local_profile = profile(ContentAdapterClass::CapabilityLocal);
    let local_command = command(&local_profile, &manifest, ContentOperation::Get, None);
    let local = execute_local_stream_get(&local_profile, &root, &local_command, GENERATION_ONE, None).expect("local");

    let simulation_profile = profile(ContentAdapterClass::DeterministicSimulation);
    let simulation_command = command(&simulation_profile, &manifest, ContentOperation::Get, None);
    let chunks = fixture_chunks(&manifest);
    let simulated = execute_simulated_stream(
        &simulation_profile,
        &manifest,
        &simulation_command,
        GENERATION_ONE,
        None,
        &chunks,
        None,
    )
    .expect("simulation");
    assert_eq!(simulated.state.artifact.verified_chunk_refs, local.state.artifact.verified_chunk_refs);
    assert_eq!(
        assemble_verified_content(&manifest, &simulated.state.artifact, &simulated.verified_chunks).unwrap(),
        b"aaaabbbbcccc"
    );

    let corrupt = execute_simulated_stream(
        &simulation_profile,
        &manifest,
        &simulation_command,
        GENERATION_ONE,
        None,
        &chunks,
        Some(SimulationFault::CorruptAt(1)),
    )
    .expect("corrupt simulation outcome");
    assert_eq!(corrupt.state.artifact.terminal, ContentTerminal::Failed);
    assert_eq!(corrupt.verified_chunks.len(), 1);
    assert!(assemble_verified_content(&manifest, &corrupt.state.artifact, &corrupt.verified_chunks).is_err());

    let capacity = execute_simulated_stream(
        &simulation_profile,
        &manifest,
        &simulation_command,
        GENERATION_ONE,
        None,
        &chunks,
        Some(SimulationFault::CapacityExceeded),
    )
    .expect("capacity outcome");
    assert_eq!(capacity.state.artifact.terminal, ContentTerminal::Retryable);
    let delayed = execute_simulated_stream(
        &simulation_profile,
        &manifest,
        &simulation_command,
        GENERATION_ONE,
        None,
        &chunks,
        Some(SimulationFault::LatencyTicks(SIMULATION_TIMEOUT_LATENCY)),
    )
    .expect("latency outcome");
    assert_eq!(delayed.state.artifact.terminal, ContentTerminal::Uncertain);

    let cancelled = execute_simulated_stream(
        &simulation_profile,
        &manifest,
        &simulation_command,
        GENERATION_ONE,
        None,
        &chunks,
        Some(SimulationFault::CancelAt(1)),
    )
    .expect("cancelled outcome");
    assert_eq!(cancelled.state.artifact.terminal, ContentTerminal::Cancelled);
    let partial_workspace = temp_dir("content-adapter-partial-state");
    let partial_namespace = crate::node_state::NodeStateNamespace::open(
        crate::node_state::NodeStateNamespaceKind::Ledger,
        &partial_workspace,
    )
    .expect("partial-state namespace");
    persist_partial_state(&partial_namespace, &simulation_profile, &manifest, &cancelled.state.artifact)
        .expect("persist partial state");
    let loaded =
        load_partial_state(&partial_namespace, &simulation_profile, &manifest, &simulation_command.operation_ref)
            .expect("load partial state")
            .expect("persisted state");
    assert_eq!(loaded, cancelled.state.artifact);
    let mut invalid_state = loaded.clone();
    invalid_state.manifest_ref = test_ref("wrong-manifest");
    assert!(persist_partial_state(&partial_namespace, &simulation_profile, &manifest, &invalid_state).is_err());
    let resumed = execute_simulated_stream(
        &simulation_profile,
        &manifest,
        &simulation_command,
        GENERATION_ONE,
        Some(&loaded),
        &chunks,
        None,
    )
    .expect("resumed outcome");
    let mut resumed_chunks = cancelled.verified_chunks;
    resumed_chunks.extend(resumed.verified_chunks.clone());
    assert_eq!(
        assemble_verified_content(&manifest, &resumed.state.artifact, &resumed_chunks).expect("resumed content"),
        b"aaaabbbbcccc"
    );
    remove_partial_state(&partial_namespace, &simulation_command.operation_ref).expect("remove partial state");
    assert!(
        load_partial_state(&partial_namespace, &simulation_profile, &manifest, &simulation_command.operation_ref,)
            .expect("load removed state")
            .is_none()
    );
    std::fs::remove_dir_all(workspace).expect("remove simulation fixture");
    std::fs::remove_dir_all(partial_workspace).expect("remove partial-state fixture");
}

// r[verify molten.content_store_adapter.identity_boundary]
// r[verify molten.content_store_adapter.live_sim_conformance]
#[tokio::test]
async fn live_iroh_blobs_stream_preserves_molten_identity_and_uses_opaque_admitted_transport_key() {
    let (workspace, root, manifest) = fixture_store("content-adapter-live-iroh");
    let identity_workspace = temp_dir("content-adapter-live-identity");
    let namespace = crate::node_state::NodeStateNamespace::open(
        crate::node_state::NodeStateNamespaceKind::Identity,
        &identity_workspace,
    )
    .expect("identity namespace");
    let crypto_profile = crate::fabric_crypto_identity::canonical_crypto_profile(
        &crate::fabric_crypto_identity::production_ed25519_profile(
            test_ref("content-crypto-profile"),
            test_ref("content-crypto-entropy"),
        ),
    )
    .expect("crypto profile");
    let backend_ref = test_ref("content-identity-backend");
    let adapter =
        crate::fabric_crypto_identity::IrohEd25519FileAdapter::new(&namespace, crypto_profile, backend_ref.clone())
            .expect("crypto adapter");
    adapter
        .resolve_or_generate(
            crate::fabric_crypto_identity::KeyPurpose::TransportEndpoint,
            &test_ref("content-key-policy"),
            true,
        )
        .expect("transport identity");
    let key_path = crate::fabric_crypto_identity::transport_key_path().expect("transport key path");
    let key_record = namespace.read(&key_path, crate::node_state::MAX_NODE_SECRET_BYTES).expect("transport key record");
    let material = crate::fabric_crypto_identity::transport_endpoint_material(&key_record, &backend_ref)
        .expect("transport endpoint material");
    let endpoint_id = material.endpoint_id.clone();
    let profile = profile(ContentAdapterClass::IrohBlobs);
    assert!(
        publish_live_iroh_chunks(&profile, &root, &manifest.manifest_ref, LiveIrohIdentity {
            namespace: &namespace,
            endpoint_id: &endpoint_id,
            handle_ref: &test_ref("stale-handle"),
            backend_ref: &backend_ref,
        },)
        .await
        .is_err()
    );
    let mut publication = publish_live_iroh_chunks(&profile, &root, &manifest.manifest_ref, LiveIrohIdentity {
        namespace: &namespace,
        endpoint_id: &endpoint_id,
        handle_ref: &material.handle_ref,
        backend_ref: &backend_ref,
    })
    .await
    .expect("live publication");
    assert_eq!(publication.manifest().manifest_ref, manifest.manifest_ref);
    let get = command(&profile, publication.manifest(), ContentOperation::Get, None);
    let execution = execute_live_iroh_stream_get(
        &profile,
        &publication,
        &get,
        GENERATION_ONE,
        None,
        Duration::from_secs(LIVE_TIMEOUT_SECONDS),
    )
    .await
    .expect("live Iroh stream");
    assert_eq!(execution.state.artifact.terminal, ContentTerminal::Verified);
    assert_eq!(
        assemble_verified_content(publication.manifest(), &execution.state.artifact, &execution.verified_chunks)
            .unwrap(),
        b"aaaabbbbcccc"
    );
    assert!(!execution.backend_hint_ref.contains(&endpoint_id));

    publication.invalidate_first_locator();
    let stale = execute_live_iroh_stream_get(
        &profile,
        &publication,
        &get,
        GENERATION_ONE,
        None,
        Duration::from_secs(LIVE_TIMEOUT_SECONDS),
    )
    .await
    .expect("stale ticket outcome");
    assert_eq!(stale.state.artifact.terminal, ContentTerminal::Failed);
    assert_eq!(stale.state.artifact.failure, Some(ContentFailure::StaleTicket));
    assert!(stale.verified_chunks.is_empty());
    publication.shutdown().await.expect("shutdown publication");
    std::fs::remove_dir_all(workspace).expect("remove live fixture");
    std::fs::remove_dir_all(identity_workspace).expect("remove identity fixture");
}

// r[verify molten.content_store_adapter.retention_boundary]
// r[verify molten.content_store_adapter.final_validation]
#[test]
fn status_and_backend_protection_are_redacted_bounded_and_non_authoritative() {
    let profile = profile(ContentAdapterClass::CapabilityLocal);
    let backend_label = "/private/root?ticket=secret";
    let status = bounded_content_status(
        &profile,
        GENERATION_ONE,
        0,
        0,
        vec![(ContentTerminal::Verified, 1)],
        backend_label,
        Vec::new(),
    )
    .expect("status");
    let status_text = crate::preserves_rail::to_text(&status.value).expect("status text");
    assert!(!status_text.contains(backend_label));
    let protection =
        backend_protection_status(&profile, &test_ref("protected-manifest"), false).expect("unprotect status");
    assert_eq!(protection.artifact.terminal, ContentTerminal::Verified);
    assert_eq!(backend_protection_effect_grants_authority(), ContentAuthorityDecision::Deny);
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::cleanup_stale_molten_temp_dirs();
    static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("remove stale fixture");
    }
    std::fs::create_dir_all(&dir).expect("create fixture");
    dir
}
