use std::collections::BTreeMap;

use molten_core::world_replay::*;

use super::super::*;
use super::fixture::*;
use super::ports::*;

#[test]
fn export_uses_exact_declared_members_without_transport_identity_claims() {
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let payloads = payloads(&fixture.request.capsule);
    let mut exchange = Exchange::default();
    let outcome = export_world_replay_capsule(&fixture.request, &payloads, &mut exchange).expect("capsule export");

    assert_eq!(outcome.observations.len(), fixture.request.capsule.members.len());
    assert_eq!(exchange.exported, fixture.request.capsule.members.len());
    assert!(outcome.observations.iter().all(|item| item.locator_hint.starts_with("detached:")));
    let locators = outcome
        .observations
        .iter()
        .map(|item| (item.object_ref.clone(), item.locator_hint.clone()))
        .collect::<BTreeMap<_, _>>();
    let fetched = fetch_world_replay_capsule(&fixture.request, &locators, &mut exchange).expect("capsule fetch");
    assert_eq!(fetched.len(), fixture.request.capsule.members.len());
}

#[test]
fn adapters_reuse_sealed_reproduction_and_content_manifest_boundaries() {
    // r[verify molten.world_replay.capsule]
    let suite = crate::preserves_rail::parse_text(
        r#"<harness-suite-v1 "molten.harness.suite.v1" "world-replay" 1
          <budget-v1 "molten.harness.budget.v1" <limits 8 2 32 65536>>
          <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
          <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "assert" #f "ready">]>
          [<assert "a" "ready">]>"#,
    )
    .expect("suite");
    let run = crate::harness::run_suite_value(&suite).expect("run suite");
    let bundle = crate::harness::sealed_repro_bundle_value_with_command(&run.report_value, &["molten".to_string()])
        .expect("sealed bundle");
    let (bundle_member, bundle_payload) =
        world_replay_sealed_reproduction_member(&bundle).expect("world replay bundle member");
    assert_eq!(bundle_member.object_ref, bundle_payload.object_ref);
    assert_eq!(bundle_member.codec, WorldReplayMemberCodec::SealedReproductionBundleV1);

    let root = temporary_root("world-replay-content-manifest");
    let put = crate::chunk_store::put_bytes(&root, "world-replay", b"abcdefgh", CHUNK_BYTES).expect("put");
    let manifest =
        crate::chunk_store::parse_manifest_value(&put.manifest_value, Some(&put.manifest_ref)).expect("manifest");
    let descriptor = crate::content_store_adapter::manifest_descriptor(&manifest);
    let manifest_bytes = crate::preserves_rail::canonical_bytes(&put.manifest_value).expect("manifest bytes");
    let (manifest_member, manifest_payload) =
        world_replay_content_manifest_member(&descriptor, &manifest_bytes).expect("manifest member");
    assert_eq!(manifest_member.object_ref, manifest_payload.object_ref);
    assert_eq!(manifest_member.codec, WorldReplayMemberCodec::ContentManifestV1);
    std::fs::remove_dir_all(root).expect("remove fixture root");
}

#[test]
fn the_trace_member_binds_the_trace_to_its_exact_record_bytes() {
    // r[verify molten.world_commit.replay_sealed_observation_and_full_comparison]
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let trace = &fixture.request.trace;
    let record = canonical_world_transition_trace(trace).expect("trace record");
    let (member, payload) = world_replay_trace_member(trace, &record).expect("bound trace member");
    assert_eq!(member.object_ref, trace.trace_ref);
    assert_eq!(member.roles, vec![WorldReplayCapsuleMemberRole::Trace]);
    assert_eq!(payload.bytes, record.bytes);

    let mut other = trace.clone();
    other.steps[0].input.input_ref = digest("another-transition-input");
    other.trace_ref = identify_world_transition_trace(&other).expect("other trace identity");
    let other_record = canonical_world_transition_trace(&other).expect("other trace record");
    let error = world_replay_trace_member(trace, &other_record).expect_err("another trace's record is denied");
    assert!(error.to_string().contains("do not encode the trace"), "{error}");

    let mut spliced = record.clone();
    spliced.bytes = other_record.bytes.clone();
    let error = world_replay_trace_member(trace, &spliced).expect_err("spliced bytes are denied");
    assert!(error.to_string().contains("do not encode the trace"), "{error}");

    let mut stale = other.clone();
    stale.trace_ref = trace.trace_ref.clone();
    world_replay_trace_member(&stale, &record).expect_err("a trace whose ref names another trace is denied");
}
