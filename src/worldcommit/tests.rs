use molten_core::world_commit::CaptureRequest;
use molten_core::world_commit::ObservationStability;
use molten_core::world_commit::RevisionFence;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::RootObservation;
use molten_core::world_commit::SnapshotProfile;
use molten_core::world_commit::SnapshotProfileKind;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitVersion;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_commit::plan_capture;

use super::*;

const INITIAL_REVISION: u64 = 1;
const WORLD_COMMIT_FIELD_COUNT: usize = 6;
const WORLD_COMMIT_SCHEMA_ARTIFACT_COUNT: usize = 4;
const WORLD_COMMIT_SCHEMA_ARTIFACTS: [&str; WORLD_COMMIT_SCHEMA_ARTIFACT_COUNT] = [
    include_str!("../../schemas/preserves-boundaries/molten-world-commit-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-commit-capture-receipt-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-commit-closure-report-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-commit-restore-plan-v1.preserves"),
];

fn fixture_ref(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn profile() -> SnapshotProfile {
    SnapshotProfile {
        kind: SnapshotProfileKind::Logical,
        profile_ref: SnapshotProfileRef::new(fixture_ref("world-profile")).expect("profile ref"),
        cohort_ref: None,
    }
}

fn observation(kind: RootKind) -> RootObservation {
    let value = crate::preserves_rail::record("world-root-fixture-v1", vec![
        crate::preserves_rail::string(kind.as_str()),
        crate::preserves_rail::string(format!("{}-payload", kind.as_str())),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).expect("root bytes");
    RootObservation {
        root: WorldRootRef::parse(kind, crate::preserves_rail::content_ref_from_bytes(&bytes)).expect("typed root"),
        source_kind: kind,
        schema_validated: true,
        stability: ObservationStability::Mutable(
            RevisionFence::new(kind, format!("{}-source", kind.as_str()), INITIAL_REVISION).expect("revision fence"),
        ),
        durable: false,
        inventory_complete: true,
    }
}

fn canonical_fixture() -> CanonicalWorldCommit {
    let profile = profile();
    let request = CaptureRequest {
        version: WorldCommitVersion::V1,
        profile: profile.clone(),
        parents: Vec::new(),
        observations: profile.kind.required_roots().iter().map(|kind| observation(*kind)).collect(),
        bounds: explicit_bounds(),
    };
    let plan = plan_capture(&request).expect("capture plan");
    canonical_world_commit(&plan.core, &request.bounds).expect("canonical commit")
}

fn explicit_bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: molten_core::world_commit::MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

#[test]
fn port_failures_exclude_sensitive_adapter_messages_from_receipt_codes() {
    // r[verify molten.world_commit.verification]
    let error = WorldCommitPortError::new("root-observation-failed", "secret=do-not-record");

    let issue = super::shell::port_issue(&error);

    assert_eq!(issue, "port:root-observation-failed");
    assert!(!issue.contains("do-not-record"));
}

#[test]
fn checked_schema_artifacts_match_the_rust_boundary_specs() {
    // r[verify molten.world_commit.verification]
    for (source, spec) in WORLD_COMMIT_SCHEMA_ARTIFACTS.iter().zip(WORLD_COMMIT_BOUNDARY_SCHEMAS) {
        let value = crate::preserves_rail::parse_text(source).expect("schema artifact");
        let fields = crate::preserves_rail::simple_record_fields(
            &value,
            "preserves-boundary-schema-artifact-v1",
            WORLD_COMMIT_FIELD_COUNT,
        )
        .expect("schema artifact fields");
        assert_eq!(crate::preserves_rail::record_string_field(&fields[0], "family", "family").unwrap(), spec.family);
        assert_eq!(crate::preserves_rail::record_string_field(&fields[1], "version", "version").unwrap(), spec.version);
        assert_eq!(
            crate::preserves_rail::record_string_field(
                &fields[2],
                "preserves-schema-version",
                "Preserves schema version",
            )
            .unwrap(),
            preserves_schema::PRESERVES_SCHEMA_SPEC_VERSION
        );
        assert_eq!(
            crate::preserves_rail::record_string_field(&fields[3], "record-label", "record label").unwrap(),
            spec.record_label
        );
        assert_eq!(
            crate::preserves_rail::record_string_field(&fields[4], "schema-id", "schema id").unwrap(),
            spec.schema_id
        );
    }
}

#[test]
fn versioned_boundary_schemas_have_distinct_canonical_identities() {
    // r[verify molten.world_commit.verification]
    let refs = WORLD_COMMIT_BOUNDARY_SCHEMAS
        .iter()
        .map(crate::preserves_rail::boundary_schema_ref)
        .collect::<crate::error::Result<Vec<_>>>()
        .expect("world commit schema refs");
    let unique = refs.iter().map(|reference| reference.as_str()).collect::<std::collections::BTreeSet<_>>();

    assert_eq!(refs.len(), WORLD_COMMIT_BOUNDARY_SCHEMAS.len());
    assert_eq!(unique.len(), refs.len());
    assert!(WORLD_COMMIT_BOUNDARY_SCHEMAS.iter().all(|schema| schema.version == "v1"));
}

#[test]
fn canonical_codec_roundtrips_fixture_and_binds_every_behavior_input() {
    // r[verify molten.world_commit.core]
    let first = canonical_fixture();
    let fixture =
        crate::preserves_rail::parse_text(include_str!("../../tests/fixtures/world-commit/logical-v1.preserves"))
            .expect("logical fixture");
    let fixture_bytes = crate::preserves_rail::canonical_bytes(&fixture).expect("logical fixture bytes");
    assert_eq!(fixture_bytes, first.bytes);
    let parsed = parse_canonical_world_commit_with_ref(&first.bytes, &first.commit_ref, &explicit_bounds())
        .expect("roundtrip world commit");
    assert_eq!(parsed, first);

    let mut changed = first.core.clone();
    let replacement =
        WorldRootRef::parse(RootKind::Artifact, fixture_ref("changed-artifact-root")).expect("changed artifact root");
    changed.roots.retain(|root| root.kind() != RootKind::Artifact);
    changed.roots.push(replacement);
    let changed = canonical_world_commit(&changed, &explicit_bounds()).expect("changed commit");
    assert_ne!(changed.commit_ref, first.commit_ref);
}

#[test]
fn codec_rejects_stale_schema_embedded_evidence_and_non_normalized_roots() {
    // r[verify molten.world_commit.verification]
    assert!(parse_canonical_world_commit(b"malformed-packed-preserves", &explicit_bounds()).is_err());
    let canonical = canonical_fixture();
    let fields = preserves::ValueImpl::collect_simple_record(
        &canonical.value,
        WORLD_COMMIT_RECORD,
        Some(WORLD_COMMIT_FIELD_COUNT),
    )
    .expect("world commit fields");

    let mut stale = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
    stale[0] = crate::preserves_rail::string("molten.world-commit.v2");
    let stale_bytes =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(WORLD_COMMIT_RECORD, stale))
            .expect("stale bytes");
    assert!(parse_canonical_world_commit(&stale_bytes, &explicit_bounds()).is_err());

    let mut embedded = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect::<Vec<_>>();
    embedded.push(crate::preserves_rail::record("attestation", vec![crate::preserves_rail::string(fixture_ref(
        "forbidden-attestation",
    ))]));
    let embedded_bytes =
        crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(WORLD_COMMIT_RECORD, embedded))
            .expect("embedded evidence bytes");
    assert!(parse_canonical_world_commit(&embedded_bytes, &explicit_bounds()).is_err());

    let mut reordered = canonical.core.clone();
    reordered.roots.reverse();
    let reordered_bytes =
        crate::preserves_rail::canonical_bytes(&world_commit_value(&reordered)).expect("reordered bytes");
    assert!(parse_canonical_world_commit(&reordered_bytes, &explicit_bounds()).is_err());
}

#[test]
fn detached_valence_and_artifact_auth_projections_leave_commit_identity_unchanged() {
    // r[verify molten.world_commit.detached_evidence]
    let commit = canonical_fixture();
    let identity_before = commit.commit_ref.clone();
    let valence = project_world_commit_to_valence(&commit).expect("Valence projection");
    let statement = project_world_commit_artifact_auth_statement(&commit, WorldCommitArtifactAuthInput {
        producer_id: "molten",
        key_id: "world-commit-evidence-key",
        key_identity_ref: &fixture_ref("public-key"),
        verifier_context_ref: &fixture_ref("verifier-context"),
    })
    .expect("artifact-auth statement");
    artifact_auth_core::canonical_statement_bytes(&statement).expect("canonical artifact-auth statement");

    assert!(valence.report.valid);
    assert_eq!(commit.commit_ref, identity_before);
    assert_eq!(
        statement.scope.subject.digest_hex,
        crate::preserves_rail::content_ref_hex(commit.commit_ref.as_str()).expect("commit digest")
    );
}
