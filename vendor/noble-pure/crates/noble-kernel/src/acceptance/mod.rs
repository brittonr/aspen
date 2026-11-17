//! The finite acceptance machine for the M2 fragment.
//!
//! The machine keeps an explicit frame stack (bounded by the declared depth
//! limit) instead of recursion, and threads its work meter and derivations as
//! returned state. Every conclusion is derived from the candidate's premises.

mod frames;
mod nodes;
mod parts;
mod preflight;
mod schemes;
mod validate;

enum Fail {
    Invalid(crate::untrusted::Diagnostic),
    Unsupported(crate::untrusted::UnsupportedKind),
    Exhausted(crate::untrusted::LimitKind),
    Internal,
}

/// One host-supplied borrowed input, ordered independently of Data values.
struct BorrowedInput {
    logical_position: usize,
    ty: crate::types::Ty,
}

/// One frame: an in-progress body fold.
struct Frame {
    depth: u32,
    origin: Option<crate::untrusted::NodeId>,
    body: alloc::vec::Vec<crate::untrusted::NodeId>,
    index: usize,
    stack: alloc::vec::Vec<crate::types::Ty>,
    borrowed: alloc::vec::Vec<BorrowedInput>,
    effects: crate::types::EffSet,
    claimed_out: alloc::vec::Vec<crate::types::Ty>,
    claimed_effects: crate::types::EffSet,
}

struct State {
    work: u32,
    derivations: alloc::vec::Vec<crate::untrusted::Derivation>,
    live_sites: alloc::vec::Vec<crate::untrusted::LiveSite>,
}

/// The machine between steps: its meter and frame stack, and, once the run
/// stops, its outcome.
struct Machine {
    state: State,
    frames: alloc::vec::Vec<Frame>,
    outcome: Option<Result<crate::untrusted::Interface, Fail>>,
}

/// Run the acceptance checker over one candidate and request.
pub fn check(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    candidate: &crate::untrusted::Candidate,
) -> crate::untrusted::Outcome {
    check_with_work(env, request, candidate).0
}

/// Run ordinary acceptance and return the work units it consumed.
///
/// The count is exact on acceptance and on body-check refusal. A refusal during
/// request preflight conservatively reports the full declared allowance because
/// the preflight error does not retain its local remaining counter.
pub fn check_with_work(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    candidate: &crate::untrusted::Candidate,
) -> (crate::untrusted::Outcome, u32) {
    let (result, work_used) = run(env, request, candidate, None);
    (outcome(result), work_used)
}

/// Type and effect derivation for a signed owner. This publicly callable
/// structural result does NOT authenticate source provenance or authorize
/// emission: only a separately source-bound compiler ingress may use it.
pub fn check_signed_structure(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    candidate: &crate::untrusted::Candidate,
    owner: crate::contracts::Definition,
    owner_inst: &crate::words::Inst,
) -> crate::untrusted::Outcome {
    check_signed_structure_with_work(env, request, candidate, owner, owner_inst).0
}

/// Run signed-owner structural checking and return the work units it consumed.
///
/// A request-preflight refusal conservatively reports the full declared
/// allowance, matching [`check_with_work`].
pub fn check_signed_structure_with_work(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    candidate: &crate::untrusted::Candidate,
    owner: crate::contracts::Definition,
    owner_inst: &crate::words::Inst,
) -> (crate::untrusted::Outcome, u32) {
    let (result, work_used) = run(env, request, candidate, Some((owner, owner_inst)));
    (outcome(result), work_used)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; the Result<Checked, Fail> argument has a destructor, which a const fn cannot evaluate (E0493 on the pinned compiler)."
)]
fn outcome(result: Result<crate::untrusted::Checked, Fail>) -> crate::untrusted::Outcome {
    match result {
        Ok(checked) => crate::untrusted::Outcome::Accepted(checked),
        Err(Fail::Invalid(diagnostic)) => crate::untrusted::Outcome::Invalid(diagnostic),
        Err(Fail::Unsupported(kind)) => crate::untrusted::Outcome::Unsupported(kind),
        Err(Fail::Exhausted(limit)) => crate::untrusted::Outcome::Exhausted(limit),
        Err(Fail::Internal) => crate::untrusted::Outcome::InternalFailure,
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; run rejects hostile requests through preflight and typed machine outcomes, and commits derivations only after successful work charging; assertion padding would add panic paths."
)]
fn run(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    candidate: &crate::untrusted::Candidate,
    signed_owner: Option<(crate::contracts::Definition, &crate::words::Inst)>,
) -> (Result<crate::untrusted::Checked, Fail>, u32) {
    let remaining = match preflight::check_request(env, request, candidate, signed_owner) {
        Ok(remaining) => remaining,
        Err(problem) => return (Err(problem), request.limits.work),
    };
    let context = parts::Ctx {
        request,
        env,
        signed_owner: signed_owner.map(|(owner, _)| owner),
    };
    let mut machine = Machine {
        state: State {
            work: remaining,
            derivations: alloc::vec::Vec::with_capacity(
                usize::try_from(request.limits.nodes.min(64)).unwrap_or(0),
            ),
            live_sites: alloc::vec::Vec::new(),
        },
        frames: alloc::vec::Vec::with_capacity(
            usize::try_from(request.limits.depth.min(64)).unwrap_or(0),
        ),
        outcome: None,
    };
    machine.frames.push(frames::entry_frame(request, candidate));
    while machine.outcome.is_none() {
        machine = step(machine, candidate, &context);
    }
    let work_used = request.limits.work.saturating_sub(machine.state.work);
    let result = match machine.outcome {
        Some(Ok(interface)) => Ok(crate::untrusted::Checked {
            interface,
            derivations: machine.state.derivations,
            live_sites: machine.state.live_sites,
        }),
        Some(Err(problem)) => Err(problem),
        None => Err(Fail::Internal),
    };
    (result, work_used)
}

/// Take one machine step: fold the current frame's next node.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; step checks frame and node lookup before dispatch and preserves typed Invalid/Internal outcomes rather than asserting on candidate-controlled state."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; step pops owned Vec frames and invokes runtime node cloning and folding; reassess if the machine becomes allocation-free."
)]
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; each Node constructor must explicitly select folding or quotation entry, so a new constructor must fail compilation until its acceptance semantics are implemented."
)]
fn step(
    mut machine: Machine,
    candidate: &crate::untrusted::Candidate,
    context: &parts::Ctx,
) -> Machine {
    let frame = match machine.frames.pop() {
        Some(frame) => frame,
        None => return halted(machine, Err(Fail::Internal)),
    };
    let node_id = match frame.body.get(frame.index) {
        Some(id) => *id,
        None => return close_frame(machine, frame, candidate, context),
    };
    let node = match nodes::node_of(candidate, node_id, context) {
        Ok(node) => node,
        Err(problem) => return halted(machine, Err(problem)),
    };
    match node {
        crate::untrusted::Node::Literal { .. }
        | crate::untrusted::Node::Invocation { .. }
        | crate::untrusted::Node::SignedSelf { .. }
        | crate::untrusted::Node::SlotInvoke { .. } => {
            fold_current(machine, frame, node_id, &node, context)
        }
        crate::untrusted::Node::Quotation { .. } => {
            open_current(machine, frame, node_id, &node, context)
        }
    }
}

/// Stop the machine with this outcome.
fn halted(mut machine: Machine, outcome: Result<crate::untrusted::Interface, Fail>) -> Machine {
    machine.outcome = Some(outcome);
    machine
}

/// Close one exhausted frame, then stop the machine or continue it.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; close_frame propagates completion failures and charges join work before committing the derivation and advanced parent; failures are outcomes, not panic conditions."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; close_frame pops and pushes Vec frames and clones the completed interface; these owned runtime transitions cannot be const."
)]
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; Completion distinguishes entry termination from a charged parent join; new completion states must require an explicit acceptance transition."
)]
fn close_frame(
    mut machine: Machine,
    frame: Frame,
    candidate: &crate::untrusted::Candidate,
    context: &parts::Ctx,
) -> Machine {
    let parent = machine.frames.pop();
    let fuel = machine.state.work;
    let completion = match frames::complete_frame(frame, parent, candidate, context, fuel) {
        Ok(completion) => completion,
        Err(problem) => return halted(machine, Err(problem)),
    };
    match completion {
        frames::Completion::Entry(effects) => halted(
            machine,
            Ok(crate::untrusted::Interface {
                stack_in: context.request.expected.stack_in.clone(),
                stack_out: context.request.expected.stack_out.clone(),
                effects,
            }),
        ),
        frames::Completion::Step {
            cost,
            node,
            interface,
            joined,
        } => {
            let remaining = match parts::charge(machine.state.work, cost) {
                Ok(remaining) => remaining,
                Err(problem) => return halted(machine, Err(problem)),
            };
            machine.state.work = remaining;
            machine
                .state
                .derivations
                .push(crate::untrusted::Derivation { node, interface });
            machine.frames.push(frames::advance(joined));
            machine
        }
    }
}

/// Fold one literal or invocation node into the machine's counters.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; fold_current returns node or work exhaustion failures before committing work, derivation and frame advancement; assertions would change the rejection contract."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; fold_current performs runtime substitution and Vec derivation/frame pushes; reassess only if those owned transitions become const-capable."
)]
fn fold_current(
    mut machine: Machine,
    frame: Frame,
    node_id: crate::untrusted::NodeId,
    node: &crate::untrusted::Node,
    context: &parts::Ctx,
) -> Machine {
    let fuel = machine.state.work;
    let (cost, joined, interface, live_site) =
        match nodes::fold_node(frame, node_id, node, context, fuel) {
            Ok(folded) => folded,
            Err(problem) => return halted(machine, Err(problem)),
        };
    let remaining = match parts::charge(machine.state.work, cost) {
        Ok(remaining) => remaining,
        Err(problem) => return halted(machine, Err(problem)),
    };
    machine.state.work = remaining;
    if let Some(site) = live_site {
        machine.state.live_sites.push(site);
    }
    machine
        .state
        .derivations
        .push(crate::untrusted::Derivation {
            node: node_id,
            interface,
        });
    machine.frames.push(frames::advance(joined));
    machine
}

/// Open one quotation node into its parent and child frames.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; open_current builds owned quotation frames and pushes them into a Vec after runtime validation and charging."
)]
fn open_current(
    mut machine: Machine,
    frame: Frame,
    node_id: crate::untrusted::NodeId,
    node: &crate::untrusted::Node,
    context: &parts::Ctx,
) -> Machine {
    let fuel = machine.state.work;
    let (cost, parent, child) = match nodes::open_quotation(frame, node_id, node, context, fuel) {
        Ok(opened) => opened,
        Err(problem) => return halted(machine, Err(problem)),
    };
    let remaining = match parts::charge(machine.state.work, cost) {
        Ok(remaining) => remaining,
        Err(problem) => return halted(machine, Err(problem)),
    };
    machine.state.work = remaining;
    machine.frames.push(parent);
    machine.frames.push(child);
    machine
}
