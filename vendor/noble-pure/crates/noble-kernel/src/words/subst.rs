//! Bounded post-order substitution from patterns to concrete types.
//! Nested programs use explicit segments; nonrecursive steps thread owned
//! stacks without early loop returns.

mod finish;
mod schedule;

/// Local bound for one substitution walk; beyond it substitution rejects.
const WORK_CAP: usize = 512;

/// One substitution step: a pattern, a constructor completion, a nested
/// program expansion, or a pre-built segment emission.
///
/// The tasks own their patterns: a reference into the scheme under
/// substitution cannot be carried across the owned segment stack Aeneas
/// interprets.
enum Task {
    Part(crate::shapes::Pattern),
    Finish(crate::shapes::Pattern),
    Expand(crate::shapes::Pattern),
    Emit(alloc::vec::Vec<crate::types::Ty>),
}

/// The state one substitution walk threads through.
struct Walk {
    /// Pending tasks.
    work: alloc::vec::Vec<Task>,
    /// Finished segments, oldest segment first.
    segments: alloc::vec::Vec<alloc::vec::Vec<crate::types::Ty>>,
}

/// One walk step's outcome.
enum StepResult {
    /// More tasks remain.
    Continue,
    /// Every task was processed.
    Done,
    /// The walk rejected the pattern.
    Failed(crate::words::InstError),
}

/// One step's returned state: the walk and the step outcome.
type StepState = (Walk, Result<(), crate::words::InstError>);

impl crate::words::Scheme {
    /// Substitute one type pattern to a concrete type.
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; every StepResult must explicitly map to its InstError or checked final segment; new substitution outcomes must force review rather than inherit a success fallback."
    )]
    pub fn subst_pattern(
        &self,
        pattern: &crate::shapes::Pattern,
        inst: &crate::words::Inst,
    ) -> Result<crate::types::Ty, crate::words::InstError> {
        let mut walk = Walk {
            work: alloc::vec::Vec::with_capacity(8),
            segments: alloc::vec::Vec::with_capacity(8),
        };
        walk.work.push(Task::Part(pattern.clone()));
        let mut outcome = StepResult::Continue;
        while matches!(outcome, StepResult::Continue) {
            let (next, step) = walk_step(self, inst, walk);
            walk = next;
            outcome = step;
        }
        match outcome {
            StepResult::Failed(problem) => Err(problem),
            StepResult::Continue | StepResult::Done => {
                let out = match walk.segments.pop() {
                    Some(segment) => segment,
                    None => return Err(crate::words::InstError::OversizedType),
                };
                if walk.segments.is_empty() {
                    super::segments::single(out)
                } else {
                    Err(crate::words::InstError::OversizedType)
                }
            }
        }
    }
}

/// Run one substitution step, returning the updated walk and its outcome.
///
/// The state is moved along exactly one path, so the translation never has to
/// join a branch that both moves and keeps the walk. The task is dispatched
/// through `run_task` so the destructuring `let` below binds a call: a
/// destructuring `let` whose right-hand side is itself a branch is the shape
/// Aeneas' let simplification rejects.
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; walk_step checks the work bound and returns terminal or typed failure outcomes rather than panicking. Owned task Vec operations, allocation and drop make this transition non-const."
)]
fn walk_step(
    scheme: &crate::words::Scheme,
    inst: &crate::words::Inst,
    walk: Walk,
) -> (Walk, StepResult) {
    let mut walk = walk;
    let task = walk.work.pop();
    let is_over_limit = walk.work.len() >= WORK_CAP;
    match task {
        None => (walk, StepResult::Done),
        Some(task) => {
            if is_over_limit {
                (
                    walk,
                    StepResult::Failed(crate::words::InstError::OversizedType),
                )
            } else {
                let (next, outcome) = run_task(scheme, inst, task, walk);
                match outcome {
                    Ok(()) => (next, StepResult::Continue),
                    Err(problem) => (next, StepResult::Failed(problem)),
                }
            }
        }
    }
}

/// Run one queued task, returning the updated walk and its outcome.
#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; each Task requires its distinct post-order handler, so new variants must force semantic review. These handlers grow owned segment Vecs and cannot be const."
)]
fn run_task(
    scheme: &crate::words::Scheme,
    inst: &crate::words::Inst,
    task: Task,
    mut walk: Walk,
) -> StepState {
    match task {
        Task::Emit(segment) => {
            walk.segments.push(segment);
            (walk, Ok(()))
        }
        Task::Finish(node) => finish::apply(node, walk),
        Task::Expand(node) => expand_task(scheme, node, inst, walk),
        Task::Part(node) => part_task(node, inst, walk),
    }
}

/// Expand one `program` pattern from the segments of its parts.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; expand_task checks the Program marker, required segment count and effect substitution before constructing ordered input/output stacks; malformed state returns InstError rather than assertion panics."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-kernel; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
fn expand_task(
    scheme: &crate::words::Scheme,
    node: crate::shapes::Pattern,
    inst: &crate::words::Inst,
    mut walk: Walk,
) -> StepState {
    let (parts_in, parts_out, slots, is_live_ref) = match node {
        crate::shapes::Pattern::Program(stack_in, stack_out, effects) => {
            let parts_in: alloc::vec::Vec<crate::shapes::Pattern> = *stack_in;
            let parts_out: alloc::vec::Vec<crate::shapes::Pattern> = *stack_out;
            let slots: alloc::vec::Vec<crate::shapes::EffectSlot> = *effects;
            (parts_in, parts_out, slots, false)
        }
        crate::shapes::Pattern::LiveRef(stack_in, stack_out, effects) => {
            let parts_in: alloc::vec::Vec<crate::shapes::Pattern> = *stack_in;
            let parts_out: alloc::vec::Vec<crate::shapes::Pattern> = *stack_out;
            let slots: alloc::vec::Vec<crate::shapes::EffectSlot> = *effects;
            (parts_in, parts_out, slots, true)
        }
        _ => return (walk, Err(crate::words::InstError::KindMismatch)),
    };
    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-maintainers; each Vec<Pattern> has nonzero-sized elements and at most isize::MAX entries, so the sum of its two program-part lengths fits usize; reassess if the representation changes."
    )]
    let wanted = parts_in.len() + parts_out.len();
    let mut collected: alloc::vec::Vec<alloc::vec::Vec<crate::types::Ty>> =
        alloc::vec::Vec::with_capacity(crate::capacity::at_least(wanted, 4));
    let mut popped = 0;
    let mut is_missing = false;
    while popped < wanted {
        match walk.segments.pop() {
            Some(segment) => {
                collected.push(segment);
                popped += 1;
            }
            None => {
                is_missing = true;
                break;
            }
        }
    }
    if is_missing {
        return (walk, Err(crate::words::InstError::OversizedType));
    }
    collected.reverse();
    let mut parts = collected.into_iter();
    let mut stack_in: alloc::vec::Vec<crate::types::Ty> =
        alloc::vec::Vec::with_capacity(crate::capacity::at_least(parts_in.len(), 4));
    let mut in_step = 0;
    while in_step < parts_in.len() {
        if let Some(segment) = parts.next() {
            stack_in.extend(segment);
        }
        in_step += 1;
    }
    let mut stack_out: alloc::vec::Vec<crate::types::Ty> =
        alloc::vec::Vec::with_capacity(crate::capacity::at_least(parts_out.len(), 4));
    let mut out_step = 0;
    while out_step < parts_out.len() {
        if let Some(segment) = parts.next() {
            stack_out.extend(segment);
        }
        out_step += 1;
    }
    let effects = match scheme.subst_effects(&slots, inst) {
        Ok(effects) => effects,
        Err(problem) => return (walk, Err(problem)),
    };
    let ty = if is_live_ref {
        crate::types::Ty::live_ref(stack_in, stack_out, effects)
    } else {
        crate::types::Ty::program(stack_in, stack_out, effects)
    };
    walk.segments.push(alloc::vec![ty]);
    (walk, Ok(()))
}

/// Emit the segment of one pattern, queueing its sub-patterns.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; part_task reports missing value/stack bindings as UnknownVariable and otherwise emits or queues the constructor's exact segment rule; arbitrary input patterns must not be asserted valid."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; part_task clones owned patterns and allocates emitted type segments and queued tasks, requiring runtime Vec and Box operations."
)]
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; each Pattern constructor defines an explicit emitted segment or post-order child schedule; new constructors must fail compilation until substitution semantics are supplied."
)]
fn part_task(node: crate::shapes::Pattern, inst: &crate::words::Inst, mut walk: Walk) -> StepState {
    let marker = node.clone();
    match node {
        crate::shapes::Pattern::Unit => walk.segments.push(alloc::vec![crate::types::Ty::Unit]),
        crate::shapes::Pattern::Bool => walk.segments.push(alloc::vec![crate::types::Ty::Bool]),
        crate::shapes::Pattern::I64 => walk.segments.push(alloc::vec![crate::types::Ty::I64]),
        crate::shapes::Pattern::Text => walk.segments.push(alloc::vec![crate::types::Ty::Text]),
        crate::shapes::Pattern::Syntax => walk.segments.push(alloc::vec![crate::types::Ty::Syntax]),
        crate::shapes::Pattern::Contract => {
            walk.segments.push(alloc::vec![crate::types::Ty::Contract])
        }
        crate::shapes::Pattern::Evidence => {
            walk.segments.push(alloc::vec![crate::types::Ty::Evidence])
        }
        crate::shapes::Pattern::Certified => {
            walk.segments.push(alloc::vec![crate::types::Ty::Certified])
        }
        crate::shapes::Pattern::Resource(kind) => walk
            .segments
            .push(alloc::vec![crate::types::Ty::Resource(kind)]),
        crate::shapes::Pattern::Nominal(id, shape) => walk
            .segments
            .push(alloc::vec![crate::types::Ty::Nominal(id, shape)]),
        crate::shapes::Pattern::GenericNominal(_, args, _) => {
            walk.work.push(Task::Finish(marker));
            let [first, second] = *args;
            walk.work.push(Task::Part(second));
            walk.work.push(Task::Part(first));
        }
        crate::shapes::Pattern::Var(var) => {
            let ty = match inst.value(var) {
                Some(ty) => ty.clone(),
                None => return (walk, Err(crate::words::InstError::UnknownVariable)),
            };
            walk.segments.push(alloc::vec![ty]);
        }
        crate::shapes::Pattern::StackVar(var) => match inst.stack(var) {
            Some(segment) => walk.work.push(Task::Emit(segment.to_vec())),
            None => return (walk, Err(crate::words::InstError::UnknownVariable)),
        },
        crate::shapes::Pattern::Pair(left, right) | crate::shapes::Pattern::Sum(left, right) => {
            walk.work.push(Task::Finish(marker));
            // Last-in-first-out: queue the right child first so the finish
            // step pops left below right, in documented order.
            walk.work.push(Task::Part(*right));
            walk.work.push(Task::Part(*left));
        }
        crate::shapes::Pattern::List(item) => {
            walk.work.push(Task::Finish(marker));
            walk.work.push(Task::Part(*item));
        }
        crate::shapes::Pattern::Program(stack_in, stack_out, _)
        | crate::shapes::Pattern::LiveRef(stack_in, stack_out, _) => {
            walk.work.push(Task::Expand(marker));
            walk = schedule::queue(&stack_in, &stack_out, walk);
        }
    }
    (walk, Ok(()))
}
