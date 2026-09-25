// Canonical projections over the explicit fabric boundary inputs, in fixture order.
// Shared verbatim with the pre-migration generator; see `inputs.rs`.

pub type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Turns a failed fixture step into a test error that keeps its label and `Debug` form.
pub trait OrFail<T> {
    fn or_fail(self, label: &str) -> TestResult<T>;
}

impl<T, E: std::fmt::Debug> OrFail<T> for std::result::Result<T, E> {
    fn or_fail(self, label: &str) -> TestResult<T> {
        self.map_err(|error| format!("{label}: {error:?}").into())
    }
}

impl<T> OrFail<T> for Option<T> {
    fn or_fail(self, label: &str) -> TestResult<T> {
        self.ok_or_else(|| format!("{label}: value is absent").into())
    }
}

/// One pinned fixture: the canonical Preserves value its projection produced and that value's ref.
pub struct FabricBoundaryCase {
    pub name: &'static str,
    pub value: preserves::IOValue,
    pub value_ref: String,
}

pub fn assignment_transition(
    proposal: &molten::fabric_membership::AssignmentProposal,
) -> TestResult<molten::fabric_membership::CanonicalAssignmentTransition> {
    let proposed = molten::fabric_membership::propose_assignment(proposal).or_fail("assignment proposal admits")?;
    let command = super::inputs::reserve_command(proposal);
    let reserved = molten::fabric_membership::apply_assignment_command(&proposed, &command)
        .or_fail("reserve transition admits")?;
    molten::fabric_membership::canonical_assignment_transition(
        &reserved,
        &super::inputs::input_ref("assignment-intent"),
        None,
        &super::inputs::input_ref("assignment-persistence"),
    )
    .or_fail("assignment transition admits")
}

pub fn transport_transition(
    profile: &molten::fabric_transport::TransportProfile,
    descriptor: molten::fabric_transport::ProtocolDescriptor,
    empty_state: &molten::fabric_transport::TransportState,
) -> TestResult<molten::fabric_transport::CanonicalTransportTransition> {
    let canonical =
        molten::fabric_transport::canonical_transport_profile(profile).or_fail("transport profile admits")?;
    let command = super::ports::register_command(descriptor);
    let registered = molten::fabric_transport::apply_transport_command(profile, empty_state, &command)
        .or_fail("transport registration admits")?;
    molten::fabric_transport::canonical_transport_transition(&canonical, registered).or_fail("transport transition")
}

pub fn durable_transition(
    profile: &molten::fabric_durability::DurableStateProfile,
    request: &molten::fabric_durability::AppendRequest,
) -> TestResult<molten::fabric_durability::CanonicalDurableTransition> {
    let canonical =
        molten::fabric_durability::canonical_durable_profile(profile).or_fail("durability profile admits")?;
    let state = super::ports::durable_state(profile);
    let appended = molten::fabric_durability::append_log(profile, &state, request).or_fail("durable append admits")?;
    molten::fabric_durability::canonical_durable_transition(&canonical, &appended).or_fail("durable transition")
}

fn membership_projections() -> TestResult<[FabricBoundaryCase; 3]> {
    let profile = molten::fabric_membership::canonical_membership_profile(&super::inputs::membership_source_profile())
        .or_fail("membership profile admits")?;
    let descriptors = super::inputs::node_descriptors();
    let view = molten::fabric_membership::canonical_membership_view(
        &profile,
        &super::inputs::membership_view(&profile.profile, &descriptors),
        &descriptors,
        super::inputs::NOW_TICKS,
        &super::inputs::compatibility_ref(),
    )
    .or_fail("membership view admits")?;
    let assignment = assignment_transition(&super::inputs::assignment_proposal())?;
    Ok([
        FabricBoundaryCase {
            name: "membership-profile",
            value: profile.value,
            value_ref: profile.admission_ref,
        },
        FabricBoundaryCase {
            name: "membership-view",
            value: view.value,
            value_ref: view.view_ref,
        },
        FabricBoundaryCase {
            name: "assignment-transition",
            value: assignment.value,
            value_ref: assignment.transition_ref,
        },
    ])
}

fn time_and_transport_projections(
    empty_transport_state: &molten::fabric_transport::TransportState,
) -> TestResult<[FabricBoundaryCase; 3]> {
    let time = molten::fabric_time::canonical_admit_time_profile(&super::inputs::time_profile_descriptor())
        .or_fail("time profile admits")?;
    let declaration = super::ports::transport_profile();
    let transport =
        molten::fabric_transport::canonical_transport_profile(&declaration).or_fail("transport profile admits")?;
    let descriptor = super::ports::protocol_descriptor(&declaration);
    let transition = transport_transition(&declaration, descriptor, empty_transport_state)?;
    Ok([
        FabricBoundaryCase {
            name: "time-profile",
            value: time.value,
            value_ref: time.profile_ref,
        },
        FabricBoundaryCase {
            name: "transport-profile",
            value: transport.value,
            value_ref: transport.profile_ref,
        },
        FabricBoundaryCase {
            name: "transport-transition",
            value: transition.value,
            value_ref: transition.transition_ref,
        },
    ])
}

fn durability_projections() -> TestResult<[FabricBoundaryCase; 2]> {
    let declaration = super::ports::durable_state_profile();
    let durable =
        molten::fabric_durability::canonical_durable_profile(&declaration).or_fail("durability profile admits")?;
    let transition = durable_transition(&declaration, &super::ports::durable_append_request())?;
    Ok([
        FabricBoundaryCase {
            name: "durable-profile",
            value: durable.value,
            value_ref: durable.profile_ref,
        },
        FabricBoundaryCase {
            name: "durable-transition",
            value: transition.value,
            value_ref: transition.transition_ref,
        },
    ])
}

/// Runs every covered canonical projection over the explicit inputs, in fixture order.
pub fn canonical_projections(
    empty_transport_state: &molten::fabric_transport::TransportState,
) -> TestResult<Vec<FabricBoundaryCase>> {
    let [membership_profile, membership_view, assignment] = membership_projections()?;
    let [time, transport, transport_transition] = time_and_transport_projections(empty_transport_state)?;
    let [durable, durable_transition] = durability_projections()?;
    Ok(vec![
        membership_profile,
        membership_view,
        assignment,
        time,
        transport,
        transport_transition,
        durable,
        durable_transition,
    ])
}
