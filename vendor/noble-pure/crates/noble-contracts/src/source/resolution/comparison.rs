#[derive(Clone, Copy)]
enum Body {
    Root,
    Quotation(u32),
}

#[derive(Clone, Copy)]
struct Entry {
    left: Body,
    right: Body,
    at: usize,
    depth: usize,
}

struct Context<'a> {
    left: &'a crate::source::Tree,
    right: &'a crate::source::Tree,
    session: &'a crate::source::Session,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; every structural comparison charges work, checks node IDs and quotation depth, and charges text comparisons before testing equality; the first mismatch or failure ends this bounded scan."
)]
pub(super) fn same_body(
    left: &crate::source::Tree,
    right: &crate::source::Tree,
    session: &crate::source::Session,
    meter: &mut crate::Meter,
) -> Result<bool, crate::Diagnostic> {
    let comparison = Context {
        left,
        right,
        session,
    };
    let mut pending = alloc::vec::Vec::with_capacity(1);
    pending.push(Entry {
        left: Body::Root,
        right: Body::Root,
        at: 0,
        depth: 0,
    });
    let mut is_same = true;
    let mut failure = None;
    while let Some(entry) = pending.pop() {
        match comparison.step(entry, &mut pending, meter) {
            Ok(true) => {}
            Ok(false) => {
                is_same = false;
                break;
            }
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(is_same),
    }
}

impl Context<'_> {
    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-maintainers; canonical comparison reads checked arena IDs, consumes work/depth budget, and grows an owned continuation stack, with allocating diagnostics on failure."
    )]
    fn step(
        &self,
        entry: Entry,
        pending: &mut alloc::vec::Vec<Entry>,
        meter: &mut crate::Meter,
    ) -> Result<bool, crate::Diagnostic> {
        attempt!(meter.charge(1, self.left.span));
        let a = attempt!(body(self.left, entry.left));
        let b = attempt!(body(self.right, entry.right));
        if a.len() != b.len() {
            return Ok(false);
        }
        if entry.at >= a.len() {
            return Ok(true);
        }
        let a_id = match a.get(entry.at) {
            Some(id) => *id,
            None => return Err(crate::internal(self.left.span)),
        };
        let b_id = match b.get(entry.at) {
            Some(id) => *id,
            None => return Err(crate::internal(self.left.span)),
        };
        pending.reserve(1);
        pending.push(Entry {
            at: entry.at.saturating_add(1),
            ..entry
        });
        let a = attempt!(self.left.node(a_id));
        let b = attempt!(self.right.node(b_id));
        match (&a.kind, &b.kind) {
            (crate::source::Kind::Literal(a), crate::source::Kind::Literal(b)) if a == b => {
                Ok(true)
            }
            (crate::source::Kind::Text(a), crate::source::Kind::Text(b)) => {
                attempt!(meter.charge(
                    attempt!(crate::index(
                        a.len().saturating_add(b.len()),
                        self.left.span
                    )),
                    self.left.span,
                ));
                Ok(a == b)
            }
            (crate::source::Kind::Call(a), crate::source::Kind::Call(b)) => {
                same_target(*a, *b, self.session, self.left.span)
            }
            (crate::source::Kind::Quotation(_), crate::source::Kind::Quotation(_)) => {
                // One sibling continuation remains for each enclosing body.
                let depth = entry.depth.saturating_add(1);
                attempt!(meter.depth(
                    attempt!(crate::index(depth, self.left.span)),
                    self.left.span,
                ));
                pending.reserve(1);
                pending.push(Entry {
                    left: Body::Quotation(a_id),
                    right: Body::Quotation(b_id),
                    at: 0,
                    depth,
                });
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; each closed body selector uses checked non-const node lookup and returns an owned diagnostic for an invalid quotation; newly added selector kinds require an explicit resolution rule."
)]
fn body(tree: &crate::source::Tree, body: Body) -> Result<&[u32], crate::Diagnostic> {
    match body {
        Body::Root => Ok(tree.body.as_slice()),
        Body::Quotation(id) => match &attempt!(tree.node(id)).kind {
            crate::source::Kind::Quotation(body) => Ok(body.as_slice()),
            _ => Err(crate::internal(tree.span)),
        },
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; named-target comparison validates both session offsets and creates an owned diagnostic if either immutable definition is missing."
)]
fn same_target(
    left: crate::source::Target,
    right: crate::source::Target,
    session: &crate::source::Session,
    span: crate::Span,
) -> Result<bool, crate::Diagnostic> {
    match (left, right) {
        (crate::source::Target::Builtin(a), crate::source::Target::Builtin(b)) => Ok(a == b),
        (crate::source::Target::SelfOwner, crate::source::Target::SelfOwner) => Ok(true),
        (crate::source::Target::SlotInvoke(a), crate::source::Target::SlotInvoke(b)) => Ok(a == b),
        (crate::source::Target::Named(a), crate::source::Target::Named(b)) => {
            let a = match session.definitions.get(attempt!(crate::offset(a, span))) {
                Some(a) => a,
                None => return Err(crate::internal(span)),
            };
            let b = match session.definitions.get(attempt!(crate::offset(b, span))) {
                Some(b) => b,
                None => return Err(crate::internal(span)),
            };
            Ok(a.identity == b.identity)
        }
        (
            crate::source::Target::Builtin(_),
            crate::source::Target::Named(_)
            | crate::source::Target::SlotInvoke(_)
            | crate::source::Target::SelfOwner,
        )
        | (
            crate::source::Target::Named(_),
            crate::source::Target::Builtin(_)
            | crate::source::Target::SlotInvoke(_)
            | crate::source::Target::SelfOwner,
        )
        | (
            crate::source::Target::SlotInvoke(_),
            crate::source::Target::Builtin(_)
            | crate::source::Target::Named(_)
            | crate::source::Target::SelfOwner,
        )
        | (
            crate::source::Target::SelfOwner,
            crate::source::Target::Builtin(_)
            | crate::source::Target::Named(_)
            | crate::source::Target::SlotInvoke(_),
        ) => Ok(false),
    }
}

/// Intern canonical resolved composition, not source spelling or formatting.
/// Identity is session-local: no portable hash/serialization claim is made.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; canonical comparisons consume work in installation order and preserve the first matching identity; new identities use checked conversion and increment rather than unchecked arithmetic."
)]
pub(crate) fn identity(
    tree: &crate::source::Tree,
    session: &crate::source::Session,
    signature: Option<&noble_kernel::words::Scheme>,
    meter: &mut crate::Meter,
) -> Result<u64, crate::Diagnostic> {
    let mut at = 0usize;
    let mut found = None;
    let mut failure = None;
    while at < session.definitions.len() {
        match same_definition(tree, signature, at, session, meter) {
            Ok(Some(identity)) => {
                found = Some(identity);
                break;
            }
            Ok(None) => {}
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
        at += 1;
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    if let Some(identity) = found {
        return Ok(identity);
    }
    match u64::try_from(session.definitions.len())
        .ok()
        .and_then(|id| id.checked_add(1))
    {
        Some(identity) => Ok(identity),
        None => Err(crate::source::exhausted(
            tree.span,
            "definition identity limit exceeded",
        )),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; definition identity comparison performs non-const metered structural traversal and returns owned diagnostics on invalid namespace entries."
)]
fn same_definition(
    tree: &crate::source::Tree,
    signature: Option<&noble_kernel::words::Scheme>,
    at: usize,
    session: &crate::source::Session,
    meter: &mut crate::Meter,
) -> Result<Option<u64>, crate::Diagnostic> {
    attempt!(meter.charge(1, tree.span));
    let definition = match session.definitions.get(at) {
        Some(definition) => definition,
        None => return Err(crate::internal(tree.span)),
    };
    // Namespace identities are assigned from the one-based definition index.
    debug_assert_ne!(definition.identity, 0);
    if definition.owner != session.declared.as_ref().and_then(|context| context.owner) {
        return Ok(None);
    }
    if !same_signature(definition.signature.as_ref(), signature) {
        return Ok(None);
    }
    let is_same = attempt!(same_body(tree, &definition.tree, session, meter));
    if is_same {
        Ok(Some(definition.identity))
    } else {
        Ok(None)
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn same_signature(
    previous: Option<&noble_kernel::words::Scheme>,
    next: Option<&noble_kernel::words::Scheme>,
) -> bool {
    match (previous, next) {
        (None, None) => true,
        (Some(previous), Some(next)) => {
            previous.var_kinds == next.var_kinds
                && previous.stack_in == next.stack_in
                && previous.stack_out == next.stack_out
                && previous.effects == next.effects
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}
