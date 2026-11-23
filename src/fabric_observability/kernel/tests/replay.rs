//! Offline conformance for a retained disposable-VM export, not provider admission.
use super::*;

const PROFILE_BYTES: u64 = 64 * 1024;
const BINDING_BYTES: u64 = 16 * 1024;

// r[verify linux_ebpf_observability.verification]
#[test]
#[ignore = "requires MOLTEN_TEST_EBPF_REPLAY_ROOT containing guarded OnixOS fixture output"]
fn retained_vm_epochs_match_recorded_projection() {
    let selected = std::env::var_os("MOLTEN_TEST_EBPF_REPLAY_ROOT").expect("explicit retained capture root");
    let selected = std::path::PathBuf::from(selected);
    let root = crate::node_state::Scope::open_existing(crate::node_state::NamespaceKind::Ledger, &selected).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&read(&root, "molten-positive.json", PROFILE_BYTES)).unwrap();
    for (epoch, expected) in [
        (1, AdapterOutcomeKind::Exported),
        (2, AdapterOutcomeKind::Exported),
        (3, AdapterOutcomeKind::Cancelled),
    ] {
        let epoch_root = crate::node_state::Scope::open_existing(
            crate::node_state::NamespaceKind::Ledger,
            &selected.join(format!("runtime/generation-{epoch}")),
        )
        .unwrap();
        let export = decode_ebpf_profile(&read(&epoch_root, "expected-profile.json", PROFILE_BYTES)).unwrap();
        let binding: EbpfBinding =
            serde_json::from_slice(&read(&epoch_root, "expected-binding.json", BINDING_BYTES)).unwrap();
        assert_eq!(export.binding, binding);
        let profile = ebpf_observation_profile(&export).unwrap();
        let descriptors = ebpf_metric_descriptors(&export).unwrap();
        let adapter = checked_ebpf_profile(export, &profile, &descriptors).unwrap();
        let bytes = read(&epoch_root, "final-window.json", adapter.max_bytes as u64);
        let window = decode_ebpf_window(&bytes, adapter.max_bytes as u64).unwrap();
        let observed = observe_retained_window(&epoch_root, epoch, window.end, &profile, &adapter);
        let projected =
            canonical_ebpf_window(&profile, &adapter, &window, &observed.health.artifact.context, window.end).unwrap();
        assert_eq!(sample_refs(&observed.samples), sample_refs(&projected.samples));
        assert_eq!(observed.health.artifact_ref, projected.health.artifact_ref);
        assert_eq!(observed.accounting, window.accounting);
        assert_eq!(observed.status.artifact.kind, expected);
        assert_eq!(observed.deny_required_readiness, expected != AdapterOutcomeKind::Exported);
        if epoch == 1 {
            assert_eq!(serde_json::json!(sample_refs(&observed.samples)), original["sample_refs"]);
            assert_eq!(observed.health.artifact_ref, original["health_ref"]);
            assert_eq!(observed.status.artifact_ref, original["status_ref"]);
            assert_eq!(observed.outcome.artifact_ref, original["outcome_ref"]);
            assert_eq!(observed.event.artifact_ref, original["event_ref"]);
            let replacement = crate::node_state::Scope::open_existing(
                crate::node_state::NamespaceKind::Ledger,
                &selected.join("runtime/generation-2"),
            )
            .unwrap();
            let stale = observe_retained_window(&replacement, epoch, window.end, &profile, &adapter);
            assert_eq!(stale.status.artifact.kind, AdapterOutcomeKind::Corrupt);
            assert!(stale.deny_required_readiness);
            assert!(stale.samples.is_empty());
        }
        assert_eq!(observed.health.artifact.context.generation, epoch);
        println!("epoch {epoch}: {}", observed.status.artifact.kind.as_str());
    }
}

fn read(root: &crate::node_state::Scope, leaf: &str, bound: u64) -> Vec<u8> {
    let leaf = crate::node_state::RelativePath::parse(leaf).unwrap();
    let crate::node_state::FileObservation::Regular(file) = root.observe_file(&leaf).unwrap() else {
        panic!("expected a regular retained artifact");
    };
    file.read_bounded(bound).unwrap()
}

fn sample_refs(samples: &[Artifact<MetricSample>]) -> Vec<&str> {
    samples.iter().map(|sample| sample.artifact_ref.as_str()).collect()
}

fn observe_retained_window(
    root: &crate::node_state::Scope,
    epoch: u64,
    tick: u64,
    profile: &ObservationProfile,
    adapter: &EbpfProfile,
) -> EbpfServiceObservation {
    observe_linux_ebpf(EbpfServiceInput {
        profile,
        adapter,
        capability: Some(EbpfReadCapability {
            root,
            leaf: crate::node_state::RelativePath::parse("final-window.json").unwrap(),
            content_ref: None,
        }),
        control: EbpfServiceControl {
            epoch,
            now: tick,
            deadline: tick,
            cancelled: false,
        },
        context: ObservationContext {
            source_id: "linux-ebpf".into(),
            source_ref: adapter.binding.adapter.clone(),
            profile_ref: profile.profile_ref.clone(),
            scope: ClaimScope::LocalComponent,
            generation: epoch,
            observed_tick: tick,
            valid_until_tick: tick.checked_add(adapter.freshness_ticks).expect("finite replay horizon"),
            resource_ref: adapter.binding.scope.clone(),
            evidence_refs: Vec::new(),
            non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
        },
    })
    .unwrap()
}
