pub(super) fn install(
    session: crate::source::declared::ModuleSession,
    name_and_version: (alloc::string::String, u32),
    identity: u64,
    requirements: alloc::vec::Vec<Requirement>,
    words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        alloc::string::String,
        Option<u32>,
    ),
    crate::source::Error,
> {
    let (name, version) = name_and_version;
    let step = BindingStep::new(session, name, (version, identity), requirements, words);
    let BindingStep {
        session,
        words,
        adapter_slot,
        name,
        ..
    } = attempt!(install_requirements(step));
    let session = attempt!(finish(session, identity, words));
    Ok((session, name, adapter_slot))
}

fn install_requirements(step: BindingStep) -> Result<BindingStep, crate::source::Error> {
    let mut remaining = step.requirements.len();
    let mut outcome = Ok(step);
    while remaining != 0 && outcome.is_ok() {
        outcome = match outcome {
            Ok(state) => apply_requirement_step(state),
            Err(problem) => Err(problem),
        };
        remaining -= 1;
    }
    outcome
}

type Requirement = (
    alloc::string::String,
    alloc::string::String,
    alloc::string::String,
);

enum Registration {
    Emit(noble_kernel::contracts::BoundEmitRegistration),
    Clock(noble_kernel::contracts::BoundClockRegistration),
    CodeExport(noble_kernel::contracts::BoundCodeExportRegistration),
    Send(noble_kernel::contracts::BoundSendRegistration),
}

struct BindingStep {
    session: crate::source::declared::ModuleSession,
    words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
    adapter_slot: Option<u32>,
    name: alloc::string::String,
    version: u32,
    identity: u64,
    requirements: alloc::vec::Vec<Requirement>,
    index: usize,
}

impl BindingStep {
    const fn new(
        session: crate::source::declared::ModuleSession,
        name: alloc::string::String,
        version_and_identity: (u32, u64),
        requirements: alloc::vec::Vec<Requirement>,
        words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
    ) -> Self {
        let (version, identity) = version_and_identity;
        Self {
            session,
            words,
            adapter_slot: None,
            name,
            version,
            identity,
            requirements,
            index: 0,
        }
    }
}

fn apply_requirement_step(state: BindingStep) -> Result<BindingStep, crate::source::Error> {
    let (registration, slot) = attempt!(bound_registration(
        &state.session,
        &state.name,
        state.version,
        state.identity,
        &state.requirements[state.index]
    ));
    install_bound_step(state, registration, slot)
}

fn install_bound_step(
    mut state: BindingStep,
    registration: Registration,
    slot: u32,
) -> Result<BindingStep, crate::source::Error> {
    let mut spelling = alloc::string::String::new();
    core::mem::swap(&mut spelling, &mut state.requirements[state.index].0);
    let (session, words) = attempt!(declare_word(
        state.session,
        registration,
        spelling,
        state.words
    ));
    state.session = session;
    state.words = words;
    state.adapter_slot = Some(slot);
    state.index += 1;
    Ok(state)
}

fn finish(
    mut session: crate::source::declared::ModuleSession,
    identity: u64,
    words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
) -> Result<crate::source::declared::ModuleSession, crate::source::Error> {
    let Some(context) = session.source.declared.as_mut() else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "missing declared context",
        ));
    };
    context.words = words;
    context.owner = Some(identity);
    context.environment.caller_module = Some(identity);
    Ok(session)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn bound_registration(
    session: &crate::source::declared::ModuleSession,
    name: &str,
    version: u32,
    identity: u64,
    requirement: &Requirement,
) -> Result<(Registration, u32), crate::source::Error> {
    let selected = attempt!(binding_index(session, name, version, requirement));
    let binding = &session.bindings[selected];
    let registration = if binding.operation == "test.clock" {
        Registration::Clock(noble_kernel::contracts::BoundClockRegistration {
            adapter_identity: binding.adapter_identity.clone(),
            adapter_slot: binding.adapter_slot,
            owner: identity,
            input: binding.input.clone(),
            output: binding.output.clone(),
            effects: noble_kernel::types::EffSet::from_ids(&binding.effects),
        })
    } else if binding.operation == "test.code-export" {
        Registration::CodeExport(noble_kernel::contracts::BoundCodeExportRegistration {
            adapter_identity: binding.adapter_identity.clone(),
            adapter_slot: binding.adapter_slot,
            owner: identity,
            input: binding.input.clone(),
            output: binding.output.clone(),
            effects: noble_kernel::types::EffSet::from_ids(&binding.effects),
        })
    } else if binding.operation == "test.send" {
        Registration::Send(noble_kernel::contracts::BoundSendRegistration {
            adapter_identity: binding.adapter_identity.clone(),
            adapter_slot: binding.adapter_slot,
            owner: identity,
            input: binding.input.clone(),
            output: binding.output.clone(),
            effects: noble_kernel::types::EffSet::from_ids(&binding.effects),
        })
    } else {
        Registration::Emit(noble_kernel::contracts::BoundEmitRegistration {
            adapter_identity: binding.adapter_identity.clone(),
            adapter_slot: binding.adapter_slot,
            owner: identity,
            input: binding.input.clone(),
            output: binding.output.clone(),
            effects: noble_kernel::types::EffSet::from_ids(&binding.effects),
        })
    };
    Ok((registration, binding.adapter_slot))
}

fn binding_index(
    session: &crate::source::declared::ModuleSession,
    name: &str,
    version: u32,
    requirement: &Requirement,
) -> Result<usize, crate::source::Error> {
    debug_assert!(!name.is_empty());
    debug_assert!(version != 0);
    let (_, input, operation) = requirement;
    if !((input == "Text" && operation == "test.emit")
        || (input.is_empty() && operation == "test.clock")
        || (input == "Text Text" && operation == "test.code-export")
        || (input == "Text" && operation == "test.send"))
    {
        return Err(crate::source::declared::error(
            crate::source::Stage::Link,
            "unsupported or mismatched required operation contract",
        ));
    }
    session
        .bindings
        .iter()
        .position(|binding| {
            binding.module_name == name
                && binding.module_version == version
                && binding.operation == operation.as_str()
        })
        .ok_or_else(|| {
            crate::source::declared::error(
                crate::source::Stage::Link,
                "missing bound adapter for required operation",
            )
        })
}

fn declare_word(
    session: crate::source::declared::ModuleSession,
    registration: Registration,
    spelling: alloc::string::String,
    mut words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
    ),
    crate::source::Error,
> {
    let (session, target) = attempt!(install_registration(session, registration));
    words.push((spelling, crate::source::Target::Builtin(target.0)));
    Ok((session, words))
}

#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn install_registration(
    mut session: crate::source::declared::ModuleSession,
    registration: Registration,
) -> Result<
    (
        crate::source::declared::ModuleSession,
        noble_kernel::contracts::Definition,
    ),
    crate::source::Error,
> {
    let Some(context) = session.source.declared.as_mut() else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "missing declared context",
        ));
    };
    let environment = core::mem::take(&mut context.environment);
    let result = match registration {
        Registration::Emit(binding) => environment.declare_bound_emit(binding),
        Registration::Clock(binding) => environment.declare_bound_clock(binding),
        Registration::CodeExport(binding) => environment.declare_bound_code_export(binding),
        Registration::Send(binding) => environment.declare_bound_send(binding),
    };
    let (environment, target) = attempt!(result.map_err(|_| {
        crate::source::declared::error(
            crate::source::Stage::Link,
            "bound adapter input, output or effect differs from required operation",
        )
    }));
    context.environment = environment;
    Ok((session, target))
}
