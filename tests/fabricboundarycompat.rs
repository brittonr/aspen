//! Fabric boundary compatibility: canonical projections still match the accepted pre-migration
//! fixtures.
//!
//! `tests/fixtures/fabric-boundary/*.preserves` and `refs.tsv` were generated once at the
//! pre-migration commit `3de348149^` from the same explicit inputs in
//! `tests/fabricboundarycompat/`. A difference here means a boundary change altered a canonical
//! value or ref and needs a separate versioned change that approves it.

#[path = "fabricboundarycompat/cases.rs"]
mod cases;
#[path = "fabricboundarycompat/inputs.rs"]
mod inputs;
#[path = "fabricboundarycompat/ports.rs"]
mod ports;

use cases::OrFail;

const FIXTURE_REFS: &str = include_str!("fixtures/fabric-boundary/refs.tsv");
const FIXTURES: [(&str, &[u8]); 8] = [
    ("membership-profile", include_bytes!("fixtures/fabric-boundary/membership-profile.preserves")),
    ("membership-view", include_bytes!("fixtures/fabric-boundary/membership-view.preserves")),
    ("assignment-transition", include_bytes!("fixtures/fabric-boundary/assignment-transition.preserves")),
    ("time-profile", include_bytes!("fixtures/fabric-boundary/time-profile.preserves")),
    ("transport-profile", include_bytes!("fixtures/fabric-boundary/transport-profile.preserves")),
    ("transport-transition", include_bytes!("fixtures/fabric-boundary/transport-transition.preserves")),
    ("durable-profile", include_bytes!("fixtures/fabric-boundary/durable-profile.preserves")),
    ("durable-transition", include_bytes!("fixtures/fabric-boundary/durable-transition.preserves")),
];
const FIXTURE_BOUNDARY: &str = "fabric boundary fixture";

struct PinnedFixture {
    name: &'static str,
    bytes: &'static [u8],
    value_ref: &'static str,
}

/// Reads one `refs.tsv` row (name, ref, byte length) and checks it against the fixture bytes.
fn pinned_fixture(name: &'static str, bytes: &'static [u8]) -> cases::TestResult<PinnedFixture> {
    let row = FIXTURE_REFS.lines().find(|line| line.split('\t').next() == Some(name)).or_fail(name)?;
    let mut columns = row.split('\t').skip(1);
    let value_ref = columns.next().or_fail("refs.tsv ref column")?;
    let byte_len = columns
        .next()
        .or_fail("refs.tsv length column")?
        .parse::<usize>()
        .or_fail("refs.tsv length parses")?;
    assert!(columns.next().is_none(), "{name}: refs.tsv row has exactly three columns");
    assert_eq!(bytes.len(), byte_len, "{name}: fixture length matches refs.tsv");
    Ok(PinnedFixture { name, bytes, value_ref })
}

fn pinned_fixtures() -> cases::TestResult<Vec<PinnedFixture>> {
    assert_eq!(FIXTURE_REFS.lines().count(), FIXTURES.len(), "refs.tsv covers every fixture once");
    FIXTURES.iter().map(|(name, bytes)| pinned_fixture(name, bytes)).collect()
}

fn pinned_ref(name: &str) -> cases::TestResult<&'static str> {
    let fixtures = pinned_fixtures()?;
    let fixture = fixtures.iter().find(|fixture| fixture.name == name).or_fail(name)?;
    Ok(fixture.value_ref)
}

fn assert_ref_moves(name: &str, mutated_ref: &str) -> cases::TestResult<()> {
    assert_ne!(mutated_ref, pinned_ref(name)?, "{name}: a one-field input mutation must change the canonical ref");
    Ok(())
}

// r[verify molten.modularity.fabric_boundary.compatibility]
// r[verify molten.modularity.fabric_boundary.compatibility.fixtures]
#[test]
fn equal_explicit_inputs_reproduce_the_pre_migration_canonical_fixtures() -> cases::TestResult<()> {
    let fixtures = pinned_fixtures()?;
    let live = cases::canonical_projections(&molten::fabric_transport::TransportState::new())?;
    assert_eq!(live.len(), fixtures.len());
    for (fixture, case) in fixtures.iter().zip(live) {
        assert_eq!(case.name, fixture.name, "fixture order matches projection order");
        let live_bytes = molten::preserves_rail::canonical_bytes(&case.value).or_fail(fixture.name)?;
        assert_eq!(live_bytes.as_slice(), fixture.bytes, "{}: canonical bytes changed", fixture.name);
        assert_eq!(case.value_ref, fixture.value_ref, "{}: canonical ref changed", fixture.name);
        let decoded = molten::preserves_rail::strict_canonical_decode_with_ref(
            fixture.bytes,
            fixture.value_ref,
            FIXTURE_BOUNDARY,
        )
        .or_fail(fixture.name)?;
        assert_eq!(decoded.value, case.value, "{}: canonical value changed", fixture.name);
    }
    Ok(())
}

// r[verify molten.modularity.fabric_boundary.compatibility]
#[test]
fn one_field_membership_mutations_change_the_projection_refs() -> cases::TestResult<()> {
    let mut source = inputs::membership_source_profile();
    source.max_view_age_ticks += 1;
    let mutated_profile =
        molten::fabric_membership::canonical_membership_profile(&source).or_fail("mutated profile admits")?;
    assert_ref_moves("membership-profile", &mutated_profile.admission_ref)?;

    let profile = molten::fabric_membership::canonical_membership_profile(&inputs::membership_source_profile())
        .or_fail("profile admits")?;
    let descriptors = inputs::node_descriptors();
    let mut view = inputs::membership_view(&profile.profile, &descriptors);
    view.epoch += 1;
    let mutated_view = molten::fabric_membership::canonical_membership_view(
        &profile,
        &view,
        &descriptors,
        inputs::NOW_TICKS,
        &inputs::compatibility_ref(),
    )
    .or_fail("mutated view admits")?;
    assert_ref_moves("membership-view", &mutated_view.view_ref)?;

    let mut proposal = inputs::assignment_proposal();
    proposal.role_id = "replica-1".to_string();
    let mutated_assignment = cases::assignment_transition(&proposal)?;
    assert_ref_moves("assignment-transition", &mutated_assignment.transition_ref)
}

// r[verify molten.modularity.fabric_boundary.compatibility]
#[test]
fn one_field_time_and_transport_mutations_change_the_projection_refs() -> cases::TestResult<()> {
    let mut time = inputs::time_profile_descriptor();
    time.max_timers -= 1;
    let mutated_time = molten::fabric_time::canonical_admit_time_profile(&time).or_fail("mutated time admits")?;
    assert_ref_moves("time-profile", &mutated_time.profile_ref)?;

    let mut transport = ports::transport_profile();
    transport.limits.max_listeners -= 1;
    let mutated_transport =
        molten::fabric_transport::canonical_transport_profile(&transport).or_fail("mutated transport admits")?;
    assert_ref_moves("transport-profile", &mutated_transport.profile_ref)?;

    let transport = ports::transport_profile();
    let mut descriptor = ports::protocol_descriptor(&transport);
    descriptor.protocol_id = "echo-protocol-mutated".to_string();
    let empty_state = molten::fabric_transport::TransportState::new();
    let mutated_transition = cases::transport_transition(&transport, descriptor, &empty_state)?;
    assert_ref_moves("transport-transition", &mutated_transition.transition_ref)
}

// r[verify molten.modularity.fabric_boundary.compatibility]
#[test]
fn one_field_durability_mutations_change_the_projection_refs() -> cases::TestResult<()> {
    let mut durable = ports::durable_state_profile();
    durable.max_snapshots -= 1;
    let mutated_durable =
        molten::fabric_durability::canonical_durable_profile(&durable).or_fail("mutated durability admits")?;
    assert_ref_moves("durable-profile", &mutated_durable.profile_ref)?;

    let mut append = ports::durable_append_request();
    append.value = b"durable-record-mutated".to_vec();
    let mutated_append = cases::durable_transition(&ports::durable_state_profile(), &append)?;
    assert_ref_moves("durable-transition", &mutated_append.transition_ref)
}

// r[verify molten.modularity.fabric_boundary.compatibility.fixtures]
#[test]
fn tampered_or_truncated_fixtures_are_rejected() -> cases::TestResult<()> {
    for fixture in pinned_fixtures()? {
        let last = fixture.bytes.len().checked_sub(1).or_fail(fixture.name)?;
        let mut flipped = fixture.bytes.to_vec();
        let last_byte = flipped.get_mut(last).or_fail(fixture.name)?;
        *last_byte ^= 0x01;
        let flipped_decode =
            molten::preserves_rail::strict_canonical_decode_with_ref(&flipped, fixture.value_ref, FIXTURE_BOUNDARY);
        assert!(flipped_decode.is_err(), "{}: a flipped fixture byte must be rejected", fixture.name);

        let truncated = fixture.bytes.get(..last).or_fail(fixture.name)?;
        let truncated_decode =
            molten::preserves_rail::strict_canonical_decode_with_ref(truncated, fixture.value_ref, FIXTURE_BOUNDARY);
        assert!(truncated_decode.is_err(), "{}: a truncated fixture must be rejected", fixture.name);
    }
    Ok(())
}
