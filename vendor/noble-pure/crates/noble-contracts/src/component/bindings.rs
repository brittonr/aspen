#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; reconstruction checks every definition, effect and scheme through typed failures before publishing an environment; assertions would replace the total rejection boundary with panics."
)]
pub(super) fn make(world: &super::World) -> Result<super::Bindings, super::Error> {
    let mut environment = match crate::source::environment() {
        Ok(environment) => environment,
        Err(diagnostic) => {
            return Err(super::Error {
                stage: super::Stage::Binding,
                diagnostic,
            })
        }
    };
    let count = world.imports.len();
    let extra_effect = usize::from(
        world
            .worker_program_effect
            .is_some_and(|effect| !environment.effects.contains(&effect)),
    );
    environment.defs.reserve(count);
    environment.kinds.reserve(count);
    environment
        .effects
        .reserve(count.saturating_add(extra_effect));
    environment.deps.reserve(count);
    let mut words = alloc::vec::Vec::with_capacity(count);
    let mut effects = 0u64;
    let mut at = 0usize;
    let mut failure = None;
    while at < count {
        match install(&world.imports[at], world, &mut environment, &mut words) {
            Ok(bit) => effects |= bit,
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
        at = at.saturating_add(1);
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    // The selected source Program's latent effect must be a known type
    // identity to the guest checker, but it is never an imported WIT effect
    // grant: only the three independently named imports contribute to the
    // callable-effect bitset above.
    if let Some(effect) = world.worker_program_effect {
        if !environment.effects.contains(&effect) {
            environment.effects.push(effect);
        }
    }
    let program_type_effects = match world.worker_program_effect {
        Some(effect) => 1u64.checked_shl(effect.0).ok_or_else(super::exhausted)?,
        None => 0,
    };
    environment.resource_kinds = resource_kinds(world);
    Ok(super::Bindings {
        environment,
        words,
        effects,
        program_type_effects,
        key: world.build_context(),
    })
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; this total bounded collection copies declared kinds and adds each live async kind once; every index is checked by its loop bound and caller data has no assertion preconditions."
)]
fn resource_kinds(world: &super::World) -> alloc::vec::Vec<noble_kernel::types::ResourceKind> {
    let live = gather_live(&world.imports, [None; 3]);
    let live = gather_live(&world.exports, live);
    let mut live_count = 0usize;
    let mut at = 0usize;
    while at < live.len() {
        if live[at].is_some() {
            live_count = live_count.saturating_add(1);
        }
        at = at.saturating_add(1);
    }
    let kind_count = world.resources.len().saturating_add(live_count);
    let mut resources = alloc::vec::Vec::with_capacity(kind_count);
    let mut at = 0usize;
    while at < world.resources.len() {
        resources.push(world.resources[at].kind);
        at = at.saturating_add(1);
    }
    at = 0;
    while at < live.len() {
        let kind = live[at];
        if let Some(kind) = kind {
            resources.push(kind);
        }
        at = at.saturating_add(1);
    }
    resources
}

fn gather_live(
    operations: &[super::Operation],
    mut live: [Option<noble_kernel::types::ResourceKind>; 3],
) -> [Option<noble_kernel::types::ResourceKind>; 3] {
    let mut at = 0;
    while at < operations.len() {
        live = gather_types(&operations[at].parameters, live);
        live = gather_types(&operations[at].results, live);
        at = at.saturating_add(1);
    }
    live
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; this total classification visits a bounded type slice and writes only three fixed array indices; all supported types are explicitly classified without caller preconditions or panic paths."
)]
const fn gather_types(
    types: &[super::Type],
    mut live: [Option<noble_kernel::types::ResourceKind>; 3],
) -> [Option<noble_kernel::types::ResourceKind>; 3] {
    let mut at = 0;
    while at < types.len() {
        match types[at] {
            super::Type::StreamU8 => live[0] = Some(super::STREAM_U8_KIND),
            super::Type::FutureS64 => live[1] = Some(super::FUTURE_S64_KIND),
            super::Type::FutureResultS64String => {
                live[2] = Some(super::FUTURE_RESULT_S64_STRING_KIND)
            }
            super::Type::Boolean
            | super::Type::S64
            | super::Type::CheckedU64
            | super::Type::String
            | super::Type::Bytes
            | super::Type::ResultS64String
            | super::Type::ResultBytesString
            | super::Type::Own(_)
            | super::Type::Borrow(_) => {}
        }
        at = at.saturating_add(1);
    }
    live
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; installation appends to make's newly owned environment and name Vec after validating this import's identity and contract, requiring growable storage rather than a slice; the World remains borrowed unchanged."
)]
fn install(
    operation: &super::Operation,
    world: &super::World,
    environment: &mut noble_kernel::contracts::Env,
    words: &mut alloc::vec::Vec<(alloc::string::String, noble_kernel::contracts::Definition)>,
) -> Result<u64, super::Error> {
    let definition = match u32::try_from(environment.defs.len()) {
        Ok(id) => noble_kernel::contracts::Definition(id),
        Err(_) => return Err(super::exhausted()),
    };
    let (scheme, effect, bit) = attempt!(contract(operation, definition, world));
    environment.defs.push(scheme);
    environment
        .kinds
        .push(noble_kernel::contracts::Behavior::Named);
    environment.effects.push(effect);
    environment.deps.push(alloc::vec::Vec::new());
    words.push((operation.word.clone(), definition));
    Ok(bit)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this independently rebuilt environment checks every definition identity, operation effect, scheme and primitive collision before publication; invalid generated contracts return diagnostics rather than assertions."
)]
fn contract(
    operation: &super::Operation,
    definition: noble_kernel::contracts::Definition,
    world: &super::World,
) -> Result<(noble_kernel::words::Scheme, noble_kernel::types::EffId, u64), super::Error> {
    let effect = match operation.effect {
        Some(effect) => effect,
        None => return Err(super::invalid("missing import effect")),
    };
    if operation.definition != Some(definition) {
        return Err(super::invalid("import definition identity mismatch"));
    }
    let scheme = attempt!(scheme(operation, world));
    if scheme.validate().is_err() {
        return Err(super::invalid("invalid generated WIT import contract"));
    }
    let bit = match 1u64.checked_shl(effect.0) {
        Some(bit) => bit,
        None => return Err(super::exhausted()),
    };
    if crate::program::bootstrap_word(operation.word.as_bytes()).is_some() {
        return Err(super::invalid("WIT import shadows a Noble primitive"));
    }
    Ok((scheme, effect, bit))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; this bounded typed conversion threads borrowed owners in order and rejects missing operation identity with an owned diagnostic; it has no assertion preconditions on caller data."
)]
fn scheme(
    operation: &super::Operation,
    world: &super::World,
) -> Result<noble_kernel::words::Scheme, super::Error> {
    let tail = noble_kernel::shapes::Pattern::StackVar(noble_kernel::words::Variable(0));
    let input_count = operation.parameters.len().saturating_add(1);
    let mut stack_in = alloc::vec::Vec::with_capacity(input_count);
    let mut stack_out =
        alloc::vec::Vec::with_capacity(input_count.saturating_add(operation.results.len()));
    stack_in.push(tail.clone());
    stack_out.push(tail);
    let mut at = 0usize;
    while at < operation.parameters.len() {
        parameter(
            operation.parameters[at],
            world,
            &mut stack_in,
            &mut stack_out,
        );
        at = at.saturating_add(1);
    }
    at = 0;
    while at < operation.results.len() {
        stack_out.push(pattern(operation.results[at], world));
        at = at.saturating_add(1);
    }
    let effect = match operation.effect {
        Some(effect) => effect,
        None => return Err(super::invalid("missing WIT operation identity")),
    };
    Ok(noble_kernel::words::Scheme {
        var_kinds: alloc::vec![noble_kernel::words::VariableKind::Stack],
        stack_in,
        stack_out,
        effects: alloc::vec![noble_kernel::shapes::EffectSlot::Effect(effect)],
    })
}

#[expect(
    tigerstyle::borrowed_argument_types,
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; parameter conversion appends descriptors to the two newly owned scheme vectors; growable Vec storage is required and no runtime owner is copied."
)]
fn parameter(
    ty: super::Type,
    world: &super::World,
    stack_in: &mut alloc::vec::Vec<noble_kernel::shapes::Pattern>,
    stack_out: &mut alloc::vec::Vec<noble_kernel::shapes::Pattern>,
) {
    stack_in.push(pattern(ty, world));
    if matches!(ty, super::Type::Borrow(_)) {
        stack_out.push(pattern(ty, world));
    }
}

fn pattern(ty: super::Type, world: &super::World) -> noble_kernel::shapes::Pattern {
    match ty {
        super::Type::Boolean => noble_kernel::shapes::Pattern::Bool,
        super::Type::S64 => noble_kernel::shapes::Pattern::I64,
        super::Type::CheckedU64 => noble_kernel::shapes::Pattern::I64,
        super::Type::String => noble_kernel::shapes::Pattern::Text,
        super::Type::Bytes => noble_kernel::shapes::Pattern::List(alloc::boxed::Box::new(
            noble_kernel::shapes::Pattern::I64,
        )),
        super::Type::ResultS64String => noble_kernel::shapes::Pattern::Sum(
            alloc::boxed::Box::new(noble_kernel::shapes::Pattern::I64),
            alloc::boxed::Box::new(noble_kernel::shapes::Pattern::Text),
        ),
        super::Type::ResultBytesString => noble_kernel::shapes::Pattern::Sum(
            alloc::boxed::Box::new(noble_kernel::shapes::Pattern::List(alloc::boxed::Box::new(
                noble_kernel::shapes::Pattern::I64,
            ))),
            alloc::boxed::Box::new(noble_kernel::shapes::Pattern::Text),
        ),
        super::Type::StreamU8 => noble_kernel::shapes::Pattern::Resource(super::STREAM_U8_KIND),
        super::Type::FutureS64 => noble_kernel::shapes::Pattern::Resource(super::FUTURE_S64_KIND),
        super::Type::FutureResultS64String => {
            noble_kernel::shapes::Pattern::Resource(super::FUTURE_RESULT_S64_STRING_KIND)
        }
        super::Type::Own(kind) if world.worker_program_kind() == Some(kind) => {
            noble_kernel::shapes::Pattern::program(
                alloc::vec![noble_kernel::shapes::Pattern::I64],
                alloc::vec![noble_kernel::shapes::Pattern::I64],
                match world.worker_program_effect {
                    Some(effect) => alloc::vec![noble_kernel::shapes::EffectSlot::Effect(effect)],
                    None => alloc::vec![],
                },
            )
        }
        super::Type::Own(kind) | super::Type::Borrow(kind) => {
            noble_kernel::shapes::Pattern::Resource(kind)
        }
    }
}
