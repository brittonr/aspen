//! Selected declared-text and live-slot lexical sugar. No local instruction reaches the kernel.
//!
//! Physical stacks place immutable bound values above the visible stack.
//! Checked `dip` executes ordinary words beneath them, while `dup` and
//! `swap` project a bound value into the visible stack. A quotation captures
//! its free values at construction with checked `quote` and `compose`; its
//! recipe therefore retains the original values, not source spellings.

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain items directly, and the qualified form at every use site would add noise without changing the dependency."
)]
use alloc::vec::Vec;

struct Bound {
    name: Vec<u8>,
    binder_node: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Generation {
    BindingSwap,
    ProjectionDup,
    ProjectionSwap,
    CaptureQuote,
    CaptureCompose,
    WrapQuotation,
    WrapDip,
    CleanupDrop,
}

impl Generation {
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added."
    )]
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::BindingSwap => "binding-swap",
            Self::ProjectionDup => "projection-dup",
            Self::ProjectionSwap => "projection-swap",
            Self::CaptureQuote => "capture-quote",
            Self::CaptureCompose => "capture-compose",
            Self::WrapQuotation => "wrap-quotation",
            Self::WrapDip => "wrap-dip",
            Self::CleanupDrop => "cleanup-drop",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GeneratedWitness {
    pub node: u32,
    pub generation: Generation,
    pub occurrence_node: Option<u32>,
    pub binder_node: Option<u32>,
}

struct Lower<'a> {
    nodes: &'a mut Vec<super::Node>,
    meter: &'a mut crate::Meter,
    witnesses: Option<&'a mut Vec<GeneratedWitness>>,
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
)]
pub(super) fn lower(
    tree: &mut super::Tree,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    lower_inner(tree, meter, None)
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
)]
#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-contracts; it forwards the growable witness vector to lower_inner, which appends generated witnesses with Vec::push."
)]
pub(super) fn lower_with_witness(
    tree: &mut super::Tree,
    meter: &mut crate::Meter,
    witnesses: &mut Vec<GeneratedWitness>,
) -> Result<(), crate::Diagnostic> {
    lower_inner(tree, meter, Some(witnesses))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
)]
fn lower_inner(
    tree: &mut super::Tree,
    meter: &mut crate::Meter,
    witnesses: Option<&mut Vec<GeneratedWitness>>,
) -> Result<(), crate::Diagnostic> {
    if !tree
        .nodes
        .iter()
        .any(|node| matches!(&node.kind, super::Kind::Word(word) if word == b"let"))
    {
        return Ok(());
    }
    // A declaration's final, unreferenced quotation is its syntactic `def`
    // envelope, not an executable node. Keep it last when the lowering adds
    // checked operations, so signed owner certificates can distinguish it
    // from every generated executable node without trusting source spans.
    let envelope = tree
        .nodes
        .len()
        .checked_sub(1)
        .filter(|index| {
            matches!(&tree.nodes[*index].kind,
            super::Kind::Quotation(children) if children.is_empty())
                && !tree
                    .body
                    .iter()
                    .any(|id| usize::try_from(*id).ok() == Some(*index))
        })
        .and_then(|_| tree.nodes.pop());
    let body = core::mem::take(&mut tree.body);
    let mut lower = Lower {
        nodes: &mut tree.nodes,
        meter,
        witnesses,
    };
    let lowered = lower.body(&body, Vec::new(), 0);
    if let Some(envelope) = envelope {
        tree.nodes.push(envelope);
    }
    tree.body = attempt!(lowered);
    Ok(())
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; iterator adapters and slice comparisons in the body are not const."
)]
fn name(bytes: &[u8]) -> bool {
    let Some((&first, rest)) = bytes.split_first() else {
        return false;
    };
    matches!(first, b'a'..=b'z' | b'A'..=b'Z' | b'_')
        && rest
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        && bytes != b"let"
}

#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the name matches the established vocabulary used by callers; renaming it would churn call sites without clarifying them."
)]
fn source_node(
    nodes: &[super::Node],
    id: u32,
    span: crate::Span,
) -> Result<&super::Node, crate::Diagnostic> {
    nodes
        .get(attempt!(crate::offset(id, span)))
        .ok_or_else(|| crate::internal(span))
}

/// Check lexical free occurrences before moving any quotation body. `shadow`
/// includes binders introduced inside the quoted body, not the outer locals.
#[expect(
    clippy::too_many_arguments,
    reason = "Owner: noble-contracts; the lexical free-occurrence walk threads its node table, body, outer binders, shadow stack and meter explicitly; bundling them would hide which state each recursive step reads or extends"
)]
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::mutating_input_in_pure,
    tigerstyle::no_recursion,
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; the recursion follows the structure of an already checked, size-bounded input; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
)]
#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-contracts; the shadow scope stack is pushed and popped while names are freed, so it needs the growable vector."
)]
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-contracts; at is an in-bounds body index, so at + 1 is at most the body length and cannot overflow usize."
)]
fn free(
    nodes: &[super::Node],
    body: &[u32],
    outer: &[Bound],
    shadow: &mut Vec<Vec<u8>>,
    used: &mut [bool],
    meter: &mut crate::Meter,
    depth: u32,
    span: crate::Span,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.depth(depth, span));
    let scope = shadow.len();
    let mut at = 0;
    while at < body.len() {
        attempt!(meter.charge(1, span));
        let node = attempt!(source_node(nodes, body[at], span));
        match &node.kind {
            super::Kind::Word(word) if word == b"let" => {
                let next = body
                    .get(at + 1)
                    .copied()
                    .ok_or_else(|| crate::invalid(node.span, "let requires a local name"))?;
                let bind = attempt!(source_node(nodes, next, node.span));
                let super::Kind::Word(name_bytes) = &bind.kind else {
                    return Err(crate::invalid(bind.span, "let requires a local name"));
                };
                if !name(name_bytes) || shadow[scope..].iter().any(|known| known == name_bytes) {
                    return Err(crate::invalid(bind.span, "duplicate or invalid local name"));
                }
                attempt!(meter.node(bind.span));
                shadow.push(name_bytes.clone());
                at += 2;
                continue;
            }
            super::Kind::Word(word) => {
                let mut is_hidden = false;
                for local in shadow.iter().rev() {
                    attempt!(meter.charge(1, node.span));
                    if local == word {
                        is_hidden = true;
                        break;
                    }
                }
                if !is_hidden {
                    for (index, local) in outer.iter().enumerate() {
                        attempt!(meter.charge(1, node.span));
                        if local.name == *word {
                            used[index] = true;
                            break;
                        }
                    }
                }
            }
            super::Kind::Quotation(nested) => {
                let deeper = depth
                    .checked_add(1)
                    .ok_or_else(|| crate::invalid(node.span, "lexical quotation depth overflow"))?;
                attempt!(free(
                    nodes, nested, outer, shadow, used, meter, deeper, node.span
                ));
            }
            _ => {}
        }
        at += 1;
    }
    shadow.truncate(scope);
    Ok(())
}

impl Lower<'_> {
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
    )]
    fn node(
        &mut self,
        kind: super::Kind,
        span: crate::Span,
        generation: Generation,
        occurrence_node: Option<u32>,
        binder_node: Option<u32>,
    ) -> Result<u32, crate::Diagnostic> {
        attempt!(self.meter.node(span));
        let id = attempt!(crate::index(self.nodes.len(), span));
        self.nodes.push(super::Node { kind, span });
        if let Some(witnesses) = &mut self.witnesses {
            witnesses.push(GeneratedWitness {
                node: id,
                generation,
                occurrence_node,
                binder_node,
            });
        }
        Ok(id)
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
    )]
    fn builtin(
        &mut self,
        definition: u32,
        span: crate::Span,
        generation: Generation,
        occurrence_node: Option<u32>,
        binder_node: Option<u32>,
    ) -> Result<u32, crate::Diagnostic> {
        self.node(
            super::Kind::Call(super::Target::Builtin(definition)),
            span,
            generation,
            occurrence_node,
            binder_node,
        )
    }

    #[expect(
        tigerstyle::ambiguous_params,
        tigerstyle::mutating_input_in_pure,
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
    )]
    #[expect(
        tigerstyle::sentinel_fallback,
        reason = "Owner: noble-contracts; an unrepresentable depth maps to usize::MAX, whose following checked_add fails closed."
    )]
    fn wrapped(
        &mut self,
        id: u32,
        locals: &[Bound],
        hidden: usize,
        depth: u32,
        span: crate::Span,
        occurrence_node: u32,
    ) -> Result<Vec<u32>, crate::Diagnostic> {
        let mut code = alloc::vec![id];
        for layer in 0..hidden {
            let count = usize::try_from(depth)
                .unwrap_or(usize::MAX)
                .checked_add(layer)
                .and_then(|value| value.checked_add(1))
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| crate::invalid(span, "lexical quotation depth overflow"))?;
            attempt!(self.meter.depth(count, span));
            let binder_node = locals
                .get(layer)
                .map(|bound| bound.binder_node)
                .ok_or_else(|| crate::internal(span))?;
            let quotation = attempt!(self.node(
                super::Kind::Quotation(code),
                span,
                Generation::WrapQuotation,
                Some(occurrence_node),
                Some(binder_node)
            ));
            let dip = attempt!(self.builtin(
                3,
                span,
                Generation::WrapDip,
                Some(occurrence_node),
                Some(binder_node)
            ));
            code = alloc::vec![quotation, dip];
        }
        Ok(code)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Owner: noble-contracts; one lexical lowering step names its output code, owner, locals and witness state separately; bundling them would change the extracted subject"
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        tigerstyle::mutating_input_in_pure,
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
    )]
    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-contracts; the emitted code buffer grows with Vec::push, so it needs the growable vector."
    )]
    fn operation(
        &mut self,
        code: &mut Vec<u32>,
        definition: u32,
        locals: &[Bound],
        hidden: usize,
        depth: u32,
        span: crate::Span,
        generation: Generation,
        occurrence_node: u32,
        binder_node: Option<u32>,
    ) -> Result<(), crate::Diagnostic> {
        let id = attempt!(self.builtin(
            definition,
            span,
            generation,
            Some(occurrence_node),
            binder_node
        ));
        code.extend(attempt!(self.wrapped(
            id,
            locals,
            hidden,
            depth,
            span,
            occurrence_node
        )));
        Ok(())
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable."
    )]
    fn lookup(
        meter: &mut crate::Meter,
        locals: &[Bound],
        word: &[u8],
        span: crate::Span,
    ) -> Result<Option<usize>, crate::Diagnostic> {
        for (index, local) in locals.iter().enumerate() {
            attempt!(meter.charge(1, span));
            if local.name == word {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }

    /// Project a local at physical index `which` below *all* hidden values.
    /// Each `dup` or `swap` is itself an ordinary kernel-checked builtin.
    #[expect(
        tigerstyle::ambiguous_params,
        tigerstyle::mutating_input_in_pure,
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
    )]
    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-contracts; it forwards the growable code buffer to operation, which appends with Vec::push."
    )]
    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-contracts; the preceding which >= locals.len() refusal guarantees locals.len() - which - 1 cannot underflow."
    )]
    fn read(
        &mut self,
        code: &mut Vec<u32>,
        which: usize,
        locals: &[Bound],
        depth: u32,
        span: crate::Span,
        occurrence_node: u32,
    ) -> Result<(), crate::Diagnostic> {
        if which >= locals.len() {
            return Err(crate::internal(span));
        }
        let binder_node = Some(locals[which].binder_node);
        attempt!(self.operation(
            code,
            0,
            locals,
            which,
            depth,
            span,
            Generation::ProjectionDup,
            occurrence_node,
            binder_node
        ));
        for older in 0..(locals.len() - which - 1) {
            let hidden = which
                .checked_add(1)
                .and_then(|count| count.checked_add(older))
                .ok_or_else(|| crate::internal(span))?;
            attempt!(self.operation(
                code,
                2,
                locals,
                hidden,
                depth,
                span,
                Generation::ProjectionSwap,
                occurrence_node,
                binder_node
            ));
        }
        Ok(())
    }

    #[expect(
        tigerstyle::ambiguous_params,
        tigerstyle::mutating_input_in_pure,
        tigerstyle::unbounded_collection_growth,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; growth is bounded by the checked input or admission limits that drive the loop."
    )]
    fn quote(
        &mut self,
        id: u32,
        outer: &[Bound],
        depth: u32,
        span: crate::Span,
    ) -> Result<Vec<u32>, crate::Diagnostic> {
        let nested = {
            let node = self
                .nodes
                .get_mut(attempt!(crate::offset(id, span)))
                .ok_or_else(|| crate::internal(span))?;
            let super::Kind::Quotation(body) = &mut node.kind else {
                return Err(crate::internal(span));
            };
            core::mem::take(body)
        };
        let mut used = alloc::vec![false; outer.len()];
        let deeper = depth
            .checked_add(1)
            .ok_or_else(|| crate::invalid(span, "lexical quotation depth overflow"))?;
        attempt!(free(
            self.nodes,
            &nested,
            outer,
            &mut Vec::new(),
            &mut used,
            self.meter,
            deeper,
            span
        ));
        let mut captured = Vec::new();
        for (index, local) in outer.iter().enumerate() {
            if used[index] {
                attempt!(self.meter.node(span));
                captured.push(Bound {
                    name: local.name.clone(),
                    binder_node: local.binder_node,
                });
            }
        }
        let mut lowered = Vec::new();
        let mut is_first = true;
        // `outer[0]` is the newest/top binder. Capture oldest first, so
        // replaying the composed programs reconstructs that same stack order.
        for index in (0..used.len()).rev() {
            if used[index] {
                let binder_node = Some(outer[index].binder_node);
                attempt!(self.read(&mut lowered, index, outer, depth, span, id));
                attempt!(self.operation(
                    &mut lowered,
                    8,
                    outer,
                    outer.len(),
                    depth,
                    span,
                    Generation::CaptureQuote,
                    id,
                    binder_node
                ));
                if !is_first {
                    attempt!(self.operation(
                        &mut lowered,
                        9,
                        outer,
                        outer.len(),
                        depth,
                        span,
                        Generation::CaptureCompose,
                        id,
                        binder_node
                    ));
                }
                is_first = false;
            }
        }
        let body = attempt!(self.body(&nested, captured, deeper));
        let node = self
            .nodes
            .get_mut(attempt!(crate::offset(id, span)))
            .ok_or_else(|| crate::internal(span))?;
        node.kind = super::Kind::Quotation(body);
        lowered.extend(attempt!(self.wrapped(
            id,
            outer,
            outer.len(),
            depth,
            span,
            id
        )));
        if !is_first {
            attempt!(self.operation(
                &mut lowered,
                9,
                outer,
                outer.len(),
                depth,
                span,
                Generation::CaptureCompose,
                id,
                None
            ));
        }
        Ok(lowered)
    }

    #[expect(
        tigerstyle::allocation_in_loop,
        tigerstyle::mutating_input_in_pure,
        tigerstyle::unbounded_collection_growth,
        reason = "Owner: noble-contracts; each iteration's buffer is bounded by the checked input that drives the loop; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; growth is bounded by the checked input or admission limits that drive the loop."
    )]
    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-contracts; at is an in-bounds source index, so at + 1 is at most the source length and cannot overflow usize; a Vec length is below isize::MAX, so locals.len() + 1 cannot overflow usize."
    )]
    fn body(
        &mut self,
        source: &[u32],
        mut locals: Vec<Bound>,
        depth: u32,
    ) -> Result<Vec<u32>, crate::Diagnostic> {
        let mut code = Vec::new();
        let mut introduced = 0usize;
        let mut at = 0usize;
        while at < source.len() {
            let id = source[at];
            let node = attempt!(source_node(
                self.nodes,
                id,
                crate::Span { start: 0, end: 0 }
            ));
            let span = node.span;
            let is_binding = matches!(&node.kind, super::Kind::Word(word) if word == b"let");
            if is_binding {
                let next = source
                    .get(at + 1)
                    .copied()
                    .ok_or_else(|| crate::invalid(span, "let requires a local name"))?;
                let target = attempt!(source_node(self.nodes, next, span));
                let super::Kind::Word(name_bytes) = &target.kind else {
                    return Err(crate::invalid(target.span, "let requires a local name"));
                };
                if !name(name_bytes)
                    || locals[..introduced]
                        .iter()
                        .any(|prior| prior.name == *name_bytes)
                {
                    return Err(crate::invalid(
                        target.span,
                        "duplicate or invalid local name",
                    ));
                }
                attempt!(self.meter.depth(
                    u32::try_from(locals.len() + 1)
                        .map_err(|_| crate::invalid(span, "lexical binding depth overflow"))?,
                    span
                ));
                // The new value is just below the previously hidden locals.
                // Move it above them before installing it as the newest
                // binder; `swap` moves ownership without copying or dropping.
                for hidden in (0..locals.len()).rev() {
                    attempt!(self.operation(
                        &mut code,
                        2,
                        &locals,
                        hidden,
                        depth,
                        span,
                        Generation::BindingSwap,
                        id,
                        Some(next)
                    ));
                }
                let bound = core::mem::replace(
                    &mut self.nodes[attempt!(crate::offset(next, span))].kind,
                    super::Kind::Quotation(Vec::new()),
                );
                let super::Kind::Word(name_bytes) = bound else {
                    return Err(crate::internal(span));
                };
                self.nodes[attempt!(crate::offset(id, span))].kind =
                    super::Kind::Quotation(Vec::new());
                locals.insert(
                    0,
                    Bound {
                        name: name_bytes,
                        binder_node: next,
                    },
                );
                introduced += 1;
                at += 2;
                continue;
            }
            let local = match &node.kind {
                super::Kind::Word(word) => attempt!(Self::lookup(self.meter, &locals, word, span)),
                _ => None,
            };
            if let Some(which) = local {
                self.nodes[attempt!(crate::offset(id, span))].kind =
                    super::Kind::Quotation(Vec::new());
                attempt!(self.read(&mut code, which, &locals, depth, span, id));
            } else if matches!(&node.kind, super::Kind::Quotation(_)) {
                code.extend(attempt!(self.quote(id, &locals, depth, span)));
            } else {
                code.extend(attempt!(self.wrapped(
                    id,
                    &locals,
                    locals.len(),
                    depth,
                    span,
                    id
                )));
            }
            at += 1;
        }
        for local in &locals {
            code.push(attempt!(self.builtin(
                1,
                crate::Span { start: 0, end: 0 },
                Generation::CleanupDrop,
                None,
                Some(local.binder_node)
            )));
        }
        Ok(code)
    }
}
