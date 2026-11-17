pub(super) fn interface(
    arena: &crate::inference::Arena,
    body: &crate::source::inference::Body,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::untrusted::Expected, crate::Diagnostic> {
    attempt!(meter.node(body.span));
    Ok(noble_kernel::untrusted::Expected {
        stack_in: attempt!(arena.stack_value(body.input, body.span, meter)),
        stack_out: attempt!(arena.stack_value(body.output, body.span, meter)),
        allowed_effects: attempt!(arena.effect_value(body.effect, body.span)),
    })
}

pub(super) fn dependencies(
    body: &crate::source::inference::Body,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::contracts::Definition>, crate::Diagnostic> {
    let mut dependencies = alloc::vec::Vec::new();
    let mut at = 0usize;
    let mut failure = None;
    while at < body.nodes.len() {
        if let Err(problem) = dependency(&body.nodes[at], &mut dependencies, meter) {
            failure = Some(problem);
            break;
        }
        at += 1;
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(dependencies),
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each node and every prior dependency comparison is charged, even after a match; insertion needs a growable owned Vec and consumes a node before reserving space."
)]
fn dependency(
    node: &crate::source::inference::Draft,
    dependencies: &mut alloc::vec::Vec<noble_kernel::contracts::Definition>,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, node.span));
    if let crate::source::inference::DraftKind::Invocation(definition) = node.kind {
        let mut is_present = false;
        let mut at = 0usize;
        let mut failure = None;
        while let Some(prior) = dependencies.get(at).copied() {
            if let Err(problem) = meter.charge(1, node.span) {
                failure = Some(problem);
                break;
            }
            is_present |= prior == definition;
            at += 1;
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        if !is_present {
            attempt!(meter.node(node.span));
            dependencies.reserve(1);
            dependencies.push(definition);
        }
    }
    Ok(())
}

struct Nodes {
    nodes: alloc::vec::Vec<noble_kernel::untrusted::Node>,
    texts: alloc::vec::Vec<noble_kernel::execution::TextLiteral>,
    spans: alloc::vec::Vec<crate::Span>,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each emitted node consumes node/work budget, its ID is checked, and all witness variables are materialized fallibly before matching node/span entries are appended."
)]
pub(super) fn body(
    mut draft: crate::source::inference::Body,
    arena: &crate::inference::Arena,
    meter: &mut crate::Meter,
) -> Result<(noble_kernel::execution::Body, alloc::vec::Vec<crate::Span>), crate::Diagnostic> {
    let mut output = Nodes {
        nodes: alloc::vec::Vec::with_capacity(draft.nodes.len()),
        texts: alloc::vec::Vec::new(),
        spans: alloc::vec::Vec::with_capacity(draft.nodes.len()),
    };
    let mut at = 0usize;
    let mut failure = None;
    while at < draft.nodes.len() {
        let result = match draft.nodes.get_mut(at) {
            Some(node) => output.push(node, arena, meter),
            None => Err(crate::internal(draft.span)),
        };
        if let Err(problem) = result {
            failure = Some(problem);
            break;
        }
        at += 1;
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    Ok((
        noble_kernel::execution::Body {
            candidate: noble_kernel::untrusted::Candidate {
                format: noble_kernel::untrusted::CANDIDATE_FORMAT,
                revision: noble_kernel::untrusted::SEMANTIC_REVISION,
                nodes: output.nodes,
                body: draft.root,
            },
            texts: output.texts,
        },
        output.spans,
    ))
}

impl Nodes {
    fn push(
        &mut self,
        draft: &mut crate::source::inference::Draft,
        arena: &crate::inference::Arena,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.node(draft.span));
        let node_id =
            noble_kernel::untrusted::NodeId(attempt!(crate::index(self.nodes.len(), draft.span)));
        let variables = core::mem::take(&mut draft.variables);
        let inst = if matches!(
            &draft.kind,
            crate::source::inference::DraftKind::SlotInvoke { .. }
        ) {
            None
        } else {
            Some(attempt!(arena.instantiation(&variables, draft.span, meter)))
        };
        let node = match &mut draft.kind {
            crate::source::inference::DraftKind::Literal(lit) => {
                noble_kernel::untrusted::Node::Literal {
                    lit: *lit,
                    inst: inst.ok_or_else(|| crate::internal(draft.span))?,
                }
            }
            crate::source::inference::DraftKind::Invocation(def) => {
                noble_kernel::untrusted::Node::Invocation {
                    def: *def,
                    inst: inst.ok_or_else(|| crate::internal(draft.span))?,
                }
            }
            crate::source::inference::DraftKind::SignedSelf(def) => {
                noble_kernel::untrusted::Node::SignedSelf {
                    def: *def,
                    inst: inst.ok_or_else(|| crate::internal(draft.span))?,
                }
            }
            crate::source::inference::DraftKind::Quotation(body) => {
                noble_kernel::untrusted::Node::Quotation {
                    body: core::mem::take(body),
                    inst: inst.ok_or_else(|| crate::internal(draft.span))?,
                }
            }
            crate::source::inference::DraftKind::SlotInvoke {
                ref_ordinal,
                forwarded_ref_ordinals,
            } => noble_kernel::untrusted::Node::SlotInvoke {
                ref_ordinal: *ref_ordinal,
                site_id: node_id.0,
                forwarded_ref_ordinals: core::mem::take(forwarded_ref_ordinals),
            },
        };
        if let Some(bytes) = draft.text.take() {
            self.texts.reserve(1);
            self.texts.push(noble_kernel::execution::TextLiteral {
                node: node_id,
                bytes,
            });
        }
        self.nodes.push(node);
        self.spans.push(draft.span);
        Ok(())
    }
}
