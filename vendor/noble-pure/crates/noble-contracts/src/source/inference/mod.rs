#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; inference mutates only preparation-owned arena/draft bodies, bounded frame/validation worklists, and the shared preparation meter; source trees, supplied types, the environment, and the committed namespace remain borrowed unchanged."
)]

mod completion;
mod operations;
mod traversal;
mod validation;

pub(super) enum DraftKind {
    Literal(noble_kernel::untrusted::Lit),
    Invocation(noble_kernel::contracts::Definition),
    SignedSelf(noble_kernel::contracts::Definition),
    Quotation(alloc::vec::Vec<noble_kernel::untrusted::NodeId>),
    SlotInvoke {
        ref_ordinal: u32,
        forwarded_ref_ordinals: alloc::vec::Vec<u32>,
    },
}

pub(super) struct Draft {
    pub kind: DraftKind,
    pub variables: alloc::vec::Vec<crate::inference::Variable>,
    pub text: Option<alloc::vec::Vec<u8>>,
    pub span: crate::Span,
}

pub(super) struct Body {
    pub nodes: alloc::vec::Vec<Draft>,
    pub root: alloc::vec::Vec<noble_kernel::untrusted::NodeId>,
    pub input: u32,
    pub output: u32,
    pub effect: u32,
    pub identity: Option<u64>,
    pub owner: Option<u64>,
    pub owner_slot: Option<u32>,
    pub owner_variables: Option<alloc::vec::Vec<crate::inference::Variable>>,
    pub span: crate::Span,
}

pub(super) struct State {
    pub arena: crate::inference::Arena,
    pub bodies: alloc::vec::Vec<Body>,
    pub span: crate::Span,
    text_bytes: u32,
    pub(super) definition_base: usize,
    pub(super) holes: alloc::vec::Vec<Hole>,
    pub(super) logical_inputs: Option<alloc::vec::Vec<noble_kernel::types::Ty>>,
    pub(super) borrows: alloc::vec::Vec<Borrowed>,
}

pub(super) struct Borrowed {
    pub ordinal: u32,
    pub logical_position: u32,
    pub ty: noble_kernel::types::Ty,
}

pub(super) struct Hole {
    pub input: u32,
    pub output: u32,
    pub effect: u32,
    pub span: crate::Span,
}

#[derive(Clone, Copy)]
enum Origin {
    Root,
    Quotation(u32),
    Named,
}

#[derive(Clone, Copy)]
enum TreeKey {
    Root,
    Named(u32),
}

#[derive(Clone, Copy)]
enum BodyKey {
    Root,
    Quotation(u32),
}

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Declaration,
    Submission,
    EditorAnalysis,
}

struct Frame {
    tree: TreeKey,
    items: BodyKey,
    at: usize,
    input: u32,
    stack: u32,
    effect: u32,
    body: usize,
    sequence: alloc::vec::Vec<noble_kernel::untrusted::NodeId>,
    origin: Origin,
    span: crate::Span,
    caller: Option<crate::Span>,
    signature: Option<(u32, u32, bool)>,
}

pub(super) struct Scope<'a> {
    pub(super) root: &'a super::Tree,
    pub(super) session: &'a super::Session,
    pub(super) environment: &'a noble_kernel::contracts::Env,
    pub(super) root_signature: Option<&'a noble_kernel::words::Scheme>,
    pub(super) prospective_owner: Option<noble_kernel::contracts::Definition>,
}

impl Scope<'_> {
    #[expect(
        tigerstyle::missing_const_fn,
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; each closed tree selector resolves through checked non-const namespace indexing, with an owned diagnostic on invalid IDs; new selector kinds must receive an explicit resolution rule."
    )]
    fn tree(
        &self,
        tree_key: TreeKey,
        span: crate::Span,
    ) -> Result<&super::Tree, crate::Diagnostic> {
        match tree_key {
            TreeKey::Root => Ok(self.root),
            TreeKey::Named(id) => {
                match self
                    .session
                    .definitions
                    .get(attempt!(crate::offset(id, span)))
                {
                    Some(definition) => Ok(&definition.tree),
                    None => Err(crate::internal(span)),
                }
            }
        }
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; each closed body selector resolves through checked non-const arena access without cloning; invalid quotation IDs return owned diagnostics and new selector kinds require an explicit rule."
)]
fn items(tree: &super::Tree, body_key: BodyKey) -> Result<&[u32], crate::Diagnostic> {
    match body_key {
        BodyKey::Root => Ok(&tree.body),
        BodyKey::Quotation(id) => {
            let node = attempt!(tree.node(id));
            match &node.kind {
                super::Kind::Quotation(items) => Ok(items),
                _ => Err(crate::internal(node.span)),
            }
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; frame traversal charges work and checks depth/node limits, then solves effects and validates every open interface without choosing values for quantified holes."
)]
pub(super) fn infer(
    scope: Scope<'_>,
    mode: Mode,
    inputs: &[noble_kernel::types::Ty],
    meter: &mut crate::Meter,
) -> Result<State, crate::Diagnostic> {
    let tree = scope.root;
    let mut state = attempt!(initial(tree, scope.session, mode, inputs, meter));
    let (signature, owner_variables) = match scope.root_signature {
        Some(scheme) => {
            let input = match state.bodies.first() {
                Some(body) => body.input,
                None => return Err(crate::internal(tree.span)),
            };
            let (output, effects, variables) = attempt!(apply_signature(
                &mut state.arena,
                scheme,
                input,
                true,
                tree.span,
                meter,
            ));
            (Some((output, effects, true)), Some(variables))
        }
        None => (None, None),
    };
    if let Some(body) = state.bodies.first_mut() {
        body.owner_variables = owner_variables;
    }
    state.definition_base = match scope.prospective_owner {
        Some(owner) => attempt!(crate::offset(owner.0, tree.span)),
        None => scope.environment.defs.len(),
    };
    attempt!(meter.node(tree.span));
    let mut frames = alloc::vec::Vec::with_capacity(1);
    let body = match state.bodies.first() {
        Some(body) => body,
        None => return Err(crate::internal(tree.span)),
    };
    frames.push(Frame {
        tree: TreeKey::Root,
        items: BodyKey::Root,
        at: 0,
        input: body.input,
        stack: body.input,
        effect: body.effect,
        body: 0,
        sequence: alloc::vec::Vec::new(),
        origin: Origin::Root,
        span: tree.span,
        caller: None,
        signature,
    });
    let mut failure = None;
    while let Some(frame) = frames.pop() {
        if let Err(problem) = traversal::visit(
            frame,
            &mut frames,
            &mut state,
            &scope,
            matches!(mode, Mode::EditorAnalysis),
            meter,
        ) {
            failure = Some(problem);
            break;
        }
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    attempt!(state.arena.solve_effects(tree.span, meter));
    attempt!(validation::open(&state, meter));
    Ok(state)
}

#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
fn apply_signature(
    arena: &mut crate::inference::Arena,
    scheme: &noble_kernel::words::Scheme,
    input: u32,
    rigid: bool,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(u32, u32, alloc::vec::Vec<crate::inference::Variable>), crate::Diagnostic> {
    let variables = if rigid {
        attempt!(arena.rigid_variables(&scheme.var_kinds, span, meter))
    } else {
        attempt!(arena.variables(&scheme.var_kinds, span, meter))
    };
    let expected_input = attempt!(arena.pattern_stack(&scheme.stack_in, &variables, span, meter));
    attempt!(arena.unify(input, expected_input, span, meter));
    let expected_output = attempt!(arena.pattern_stack(&scheme.stack_out, &variables, span, meter));
    let effects = attempt!(arena.effect_pattern(&scheme.effects, &variables, span, meter));
    Ok((expected_output, effects, variables))
}

#[expect(
    clippy::vec_init_then_push,
    reason = "Owner: noble-maintainers; explicit owned push avoids the pinned Aeneas erased-region failure in vec! array conversion; reassess when the translator supports it."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; input-stack construction, the fresh effect term, and the sole root body are charged before allocation; supplied inputs and declaration holes follow separate checked paths."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
fn initial(
    tree: &super::Tree,
    session: &super::Session,
    mode: Mode,
    inputs: &[noble_kernel::types::Ty],
    meter: &mut crate::Meter,
) -> Result<State, crate::Diagnostic> {
    let mut arena = crate::inference::Arena::source(
        session.effect_universe(),
        session.bindings.is_some() || session.declared.is_some() || session.live_slots.is_some(),
    );
    if let Some(bindings) = &session.bindings {
        arena.allow_program_type_effects(bindings.program_type_effects);
    }
    if let Some(context) = &session.declared {
        let mut nominals = alloc::vec::Vec::with_capacity(context.environment.nominals.len());
        let mut at = 0usize;
        while at < context.environment.nominals.len() {
            let declaration = &context.environment.nominals[at];
            nominals.push((declaration.id, declaration.shape.clone()));
            at += 1;
        }
        arena.nominals = nominals;
        arena.generic_nominals = context.environment.generic_variants.clone();
    } else if let Some(profile) = &session.live_slots {
        arena.nominals = profile
            .environment
            .nominals
            .iter()
            .map(|decl| (decl.id, decl.shape.clone()))
            .collect();
        arena.generic_nominals = profile.environment.generic_variants.clone();
    }
    let mut borrows = alloc::vec::Vec::new();
    let mut value_inputs = alloc::vec::Vec::new();
    if session.live_slots.is_some() {
        for (position, ty) in inputs.iter().enumerate() {
            if matches!(ty, noble_kernel::types::Ty::LiveRef(..)) {
                borrows.push(Borrowed {
                    ordinal: attempt!(crate::index(borrows.len(), tree.span)),
                    logical_position: attempt!(crate::index(position, tree.span)),
                    ty: ty.clone(),
                });
            } else {
                value_inputs.push(ty.clone());
            }
        }
    }
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; declarations must retain quantified stack holes while executable submissions use their supplied stack; any new inference mode must make this choice explicitly."
    )]
    let input = match mode {
        Mode::Submission => attempt!(arena.stack(
            if session.live_slots.is_some() {
                &value_inputs
            } else {
                inputs
            },
            tree.span,
            meter
        )),
        Mode::EditorAnalysis => {
            let mut stack = attempt!(arena.add(
                crate::inference::Term::Hole(crate::inference::Sort::Stack),
                tree.span,
                meter
            ));
            for ty in inputs {
                let item = attempt!(arena.ty(ty, tree.span, meter));
                stack = attempt!(arena.add(
                    crate::inference::Term::Push(stack, item),
                    tree.span,
                    meter
                ));
            }
            stack
        }
        Mode::Declaration => attempt!(arena.add(
            crate::inference::Term::Hole(crate::inference::Sort::Stack),
            tree.span,
            meter
        )),
    };
    let effect = attempt!(arena.effect_empty(tree.span, meter));
    attempt!(meter.node(tree.span));
    let mut bodies = alloc::vec::Vec::with_capacity(1);
    bodies.push(Body {
        nodes: alloc::vec::Vec::new(),
        root: alloc::vec::Vec::new(),
        input,
        output: input,
        effect,
        identity: None,
        owner: session.declared.as_ref().and_then(|context| context.owner),
        owner_slot: None,
        owner_variables: None,
        span: tree.span,
    });
    Ok(State {
        arena,
        bodies,
        span: tree.span,
        text_bytes: 0,
        definition_base: 0,
        holes: alloc::vec::Vec::new(),
        logical_inputs: session.live_slots.as_ref().map(|_| inputs.to_vec()),
        borrows,
    })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; appending an owned draft allocates after node charging and checked body/ID lookup, whose failure paths construct diagnostics."
)]
fn add(
    body: usize,
    draft: Draft,
    state: &mut State,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::untrusted::NodeId, crate::Diagnostic> {
    attempt!(meter.node(draft.span));
    let body = match state.bodies.get_mut(body) {
        Some(body) => body,
        None => return Err(crate::internal(draft.span)),
    };
    let id = noble_kernel::untrusted::NodeId(attempt!(crate::index(body.nodes.len(), draft.span)));
    body.nodes.push(draft);
    Ok(id)
}
