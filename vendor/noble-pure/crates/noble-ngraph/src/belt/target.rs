//! Bounded, versioned target scheduling for the checked sequential-belt graph fragment.
//!
//! The list scheduler is untrusted. [`check`] resolves the fixed machine descriptor and
//! independently simulates issue, readiness, completion, preservation and output legality.
//! The accepted timing model is not a hardware measurement or an ISA claim.

mod codec;
mod execution;
mod lower;
mod machine;
mod types;
mod validate;

pub use codec::{decode, encode, MAX_ARTIFACT_BYTES};
pub use execution::{execute, execute_checked};
pub use lower::lower;
pub use machine::{Machine, MachineDescriptor, UnitClass};
pub use types::{CheckedSchedule, DecodedSchedule, Issue, Lowered, Operation, OutputRead, Refusal};

/// Fixed machine-family revision.
pub const MACHINE_REVISION: &str = "ngraph-belt-machine/0";
/// Target schedule artifact revision.
pub const FORMAT_REVISION: &str = "ngraph-belt-target/0";
/// Deterministic bounded list scheduler revision.
pub const LOWERER_REVISION: &str = "ngraph-belt-target-lowerer/0";
/// Independent target-schedule legality checker revision.
pub const CHECKER_REVISION: &str = "ngraph-belt-target-checker/0";
/// Maximum number of events in a target schedule, including saves and loads.
pub const MAX_EVENTS: usize = 65_536;
/// Maximum number of declared graph outputs in a target schedule.
pub const MAX_OUTPUTS: usize = super::MAX_OUTPUTS;
/// Maximum simulated horizon, in cycles including the first cycle zero.
pub const MAX_HORIZON: u32 = 65_536;
/// Maximum number of pending event records the checker retains.
pub const MAX_PENDING: usize = 65_536;
/// Maximum checker work units: bounded issue events plus half the horizon.
pub const MAX_CHECK_WORK: usize = MAX_EVENTS + (MAX_HORIZON as usize / 2);

/// Independently validate the exact encoded schedule against `checked` and `machine`.
///
/// The returned token borrows the checked bytes; only this function can create one.
pub fn check<'a>(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    machine: Machine,
    bytes: &'a [u8],
) -> Result<CheckedSchedule<'a>, Refusal> {
    super::support::admit_graph(checked).map_err(|_| Refusal::UnsupportedGraph)?;
    let mut view = decode(ids, bytes)?;
    if view.machine != machine {
        return Err(Refusal::MachineMismatch);
    }
    if view.graph_id != checked.id() {
        return Err(Refusal::GraphIdentity);
    }
    validate::check_stream(checked, &mut view)?;
    Ok(CheckedSchedule {
        bytes,
        machine: view.machine,
        graph_id: view.graph_id,
        descriptor_digest: view.descriptor_digest,
        schedule_digest: view.schedule_digest,
        artifact_binding: view.artifact_binding,
        event_count: view.events.len(),
        output_count: view.outputs.len(),
    })
}
