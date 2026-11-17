//! D7 stage 1: admission of an exact, independently kernel-accepted subject.

use crate::identity::IdentityScheme;
/// One admission request (D7 stage 1 inputs besides the subject).
#[derive(Clone, Debug)]
pub struct Request<'a> {
    /// Requested sub-profile name; only `ngraph-i64/0` has source extraction
    /// (approval P2, option B0(b)).
    pub profile: &'a str,
    /// The caller's claimed exact closure: every definition identity
    /// reachable from the root through named calls (any order, no repeats).
    pub claimed_closure: &'a [u64],
    /// Budgets; each may only be lowered from the `/0` defaults.
    pub budget: super::Budget,
}

/// One kernel-accepted body: the root (`identity == None`) or one definition
/// specialization.
#[derive(Debug)]
pub(super) struct Body<'s> {
    pub(super) candidate: &'s noble_kernel::untrusted::Candidate,
    pub(super) identity: Option<u64>,
    /// Per-node derived interface, or `None` when no consistent derivation
    /// exists (refused with `E-INTERFACE` if the node is ever processed).
    pub(super) interfaces: alloc::vec::Vec<Option<noble_kernel::untrusted::Interface>>,
}

/// An admitted subject. Only [`admit`] constructs one, after the kernel
/// accepted every body of the exact submission it borrows.
#[derive(Debug)]
pub struct Admitted<'s> {
    pub(super) submission: &'s noble_kernel::execution::Submission,
    /// `bodies[0]` is the root; `bodies[i + 1]` is `submission.definitions[i]`.
    pub(super) bodies: alloc::vec::Vec<Body<'s>>,
    pub(super) closure: alloc::vec::Vec<u64>,
    pub(super) subject: crate::ir::Subject,
    pub(super) budget: super::Budget,
    /// Number of opaque, untouched values below the extractable root suffix.
    pub(super) preserved_tail: usize,
}

impl Admitted<'_> {
    /// Recomputed exact closure (sorted definition identities).
    #[must_use]
    pub fn closure(&self) -> &[u64] {
        &self.closure
    }

    /// The subject bound into the graph identity.
    #[must_use]
    pub const fn subject(&self) -> &crate::ir::Subject {
        &self.subject
    }

    /// The root's ordered (bottom-first) input and output stack types.
    #[must_use]
    pub fn root_interface(&self) -> (&[noble_kernel::types::Ty], &[noble_kernel::types::Ty]) {
        let e = &self.submission.request.expected;
        (&e.stack_in, &e.stack_out)
    }

    /// Original root stack values that must pass through without being read.
    #[must_use]
    pub const fn preserved_tail(&self) -> usize {
        self.preserved_tail
    }

    /// Body index of a definition specialization.
    pub(super) fn definition_body(
        &self,
        def: noble_kernel::contracts::Definition,
    ) -> Option<usize> {
        self.submission
            .definitions
            .iter()
            .position(|d| d.definition == def)
            .map(|i| i + 1)
    }
}

fn refuse(
    code: crate::diag::Code,
    detail: impl Into<alloc::string::String>,
) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(code, crate::diag::Site::Graph, detail)
}

fn at(
    code: crate::diag::Code,
    body: Option<u64>,
    node: Option<u32>,
    detail: impl Into<alloc::string::String>,
) -> crate::diag::Diagnostic {
    let site = match node {
        Some(node) => crate::diag::Site::Source(crate::ir::SourceRef {
            definition: body,
            node,
        }),
        None => crate::diag::Site::Graph,
    };
    crate::diag::Diagnostic::new(code, site, detail)
}

/// D7 stage 1 (a)–(d).
///
/// # Errors
/// `E-PROFILE`, `E-SUBJECT`, `E-LIVE-SLOT`, `E-RECURSION`, `E-CLOSURE`, `E-RESOURCE`,
/// `E-DYNAMIC-RUN`, `E-PROGRAM-OUTPUT`, `E-AGGREGATE` or `E-EFFECT`, in that
/// stage order; nothing is extracted or evaluated.
pub fn admit<'s>(
    ids: &dyn IdentityScheme,
    submission: &'s noble_kernel::execution::Submission,
    request: &Request<'_>,
) -> Result<Admitted<'s>, crate::diag::Diagnostic> {
    admit_with_tail(ids, submission, request, 0)
}

/// Admit a checked root with an opaque stack prefix preserved in order.
/// The entire original body and its exact closure are independently checked
/// before the graph profile examines only the active suffix. Extraction will
/// refuse any attempt to read, copy or move a value of the preserved prefix.
///
/// # Errors
/// `E-INTERFACE` if the claimed prefix does not have the same input/output
/// types, in addition to [`admit`]'s refusal classes.
pub fn admit_with_tail<'s>(
    ids: &dyn IdentityScheme,
    submission: &'s noble_kernel::execution::Submission,
    request: &Request<'_>,
    preserved_tail: usize,
) -> Result<Admitted<'s>, crate::diag::Diagnostic> {
    match crate::ir::SubProfile::from_name(request.profile) {
        Some(crate::ir::SubProfile::I64) => {}
        Some(crate::ir::SubProfile::F32) => {
            return Err(refuse(
                crate::diag::Code::Profile,
                "ngraph-f32/0 graphs are authored directly in ngraph-ir/0; source extraction is I64-only (B0(b))",
            ))
        }
        None => return Err(refuse(crate::diag::Code::Profile, alloc::format!("unknown sub-profile {:?}", request.profile))),
    }
    let limit = super::Budget::default();
    if request.budget.work_units > limit.work_units
        || request.budget.expanded_nodes > limit.expanded_nodes
        || request.budget.inline_depth > limit.inline_depth
    {
        return Err(refuse(
            crate::diag::Code::Profile,
            "raising a /0 budget requires a new profile policy revision",
        ));
    }

    // The environment is producer data: extraction chooses every operation's
    // behaviour from it, and the kernel accepts a Core (non-declared-module)
    // environment without comparing its rows with the bootstrap table. Every
    // profile's first rows must therefore be the canonical ones here.
    if !bootstrap_rows(submission) {
        return Err(refuse(
            crate::diag::Code::Subject,
            "the environment's built-in rows differ from the bootstrap table",
        ));
    }

    // (a) Independent kernel acceptance of every body of the exact subject.
    let bodies = accept_all(submission)?;

    // (b) Exact closure, recomputed from resolved identities.
    let closure = recompute_closure(submission, &bodies)?;
    let mut claimed = request.claimed_closure.to_vec();
    claimed.sort_unstable();
    let claimed_len = claimed.len();
    claimed.dedup();
    if claimed.len() != claimed_len || claimed != closure {
        return Err(refuse(
            crate::diag::Code::Closure,
            alloc::format!(
                "claimed closure {claimed:?} differs from the recomputed exact closure {closure:?}"
            ),
        ));
    }

    // (c) The root interface: P scalars, Unit and pairs of them.
    let expected = &submission.request.expected;
    if expected.stack_in.len() < preserved_tail
        || expected.stack_out.len() < preserved_tail
        || expected.stack_in[..preserved_tail] != expected.stack_out[..preserved_tail]
    {
        return Err(refuse(
            crate::diag::Code::Interface,
            "the preserved stack tail is absent or its input/output types differ",
        ));
    }
    for (k, ty) in expected
        .stack_in
        .iter()
        .chain(&expected.stack_out)
        .enumerate()
    {
        if has_resource(ty) {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::Resource,
                u32::try_from(k).map_or(crate::diag::Site::Graph, crate::diag::Site::Slot),
                alloc::format!("Resource or LiveRef in the root interface: {ty:?}"),
            ));
        }
    }
    for ty in &expected.stack_in[preserved_tail..] {
        classify(ty, crate::diag::Code::DynamicRun)?;
    }
    for ty in &expected.stack_out[preserved_tail..] {
        classify(ty, crate::diag::Code::ProgramOutput)?;
    }

    // (d) The root effect bound must be empty.
    if !bodies[0].1.interface.effects.is_empty() {
        return Err(refuse(
            crate::diag::Code::Effect,
            alloc::format!(
                "root effect bound {:?} is not empty",
                bodies[0].1.interface.effects
            ),
        ));
    }

    let subject = subject_of(ids, submission, &closure);
    let bodies = bodies
        .into_iter()
        .map(|(body, checked)| index_interfaces(body, &checked))
        .collect();
    Ok(Admitted {
        submission,
        bodies,
        closure,
        subject,
        budget: request.budget,
        preserved_tail,
    })
}

type Accepted<'s> = (Body<'s>, noble_kernel::untrusted::Checked);

/// Whether `submission`'s environment carries the canonical bootstrap table's
/// behaviours as its prefix.
///
/// Extraction selects every operation from `Env::kind`, so the behaviours are
/// the rows it trusts; the schemes decide only what the kernel accepts, which
/// is the kernel's own check. Additional rows (declared modules, bound
/// adapters, a selected text or calculator profile) and the host-owned widening
/// of `test.emit` are legitimate, so only the canonical prefix is compared.
fn bootstrap_rows(submission: &noble_kernel::execution::Submission) -> bool {
    let Ok(canonical) = noble_kernel::contracts::environment() else {
        return false;
    };
    let env = &submission.environment;
    if canonical.kinds.is_empty() || env.kinds.len() < canonical.kinds.len() {
        return false;
    }
    env.kinds
        .iter()
        .zip(&canonical.kinds)
        .all(|(actual, expected)| actual == expected)
}

#[expect(
    tigerstyle::allocation_in_loop,
    reason = "Owner: noble-ngraph-maintainers; every accepted body owns a fresh empty interface list stored in the returned table, so a reused buffer cannot be moved into each row."
)]
fn accept_all(
    submission: &noble_kernel::execution::Submission,
) -> Result<alloc::vec::Vec<Accepted<'_>>, crate::diag::Diagnostic> {
    let env = &submission.environment;
    let mut out = alloc::vec::Vec::with_capacity(submission.definitions.len() + 1);
    out.push((
        Body {
            candidate: &submission.body.candidate,
            identity: None,
            interfaces: alloc::vec::Vec::new(),
        },
        accept_one(env, &submission.request, &submission.body.candidate, None)?,
    ));
    for (i, d) in submission.definitions.iter().enumerate() {
        if env.kind(d.definition) != Some(noble_kernel::contracts::Behavior::Named)
            || submission.definitions[..i]
                .iter()
                .any(|e| e.definition == d.definition)
        {
            return Err(refuse(
                crate::diag::Code::Subject,
                alloc::format!(
                    "definition {:?} is not one unique named specialization",
                    d.definition
                ),
            ));
        }
        let request = noble_kernel::untrusted::Request {
            input_bytes: submission.request.input_bytes,
            expected: d.expected.clone(),
            limits: submission.request.limits,
        };
        // As noble-wasm's definition admission: a module-owned body is
        // checked from its owner's module context.
        let owner = usize::try_from(d.definition.0)
            .ok()
            .and_then(|slot| env.definition_owners.get(slot).copied())
            .flatten();
        let checked = if owner == env.caller_module {
            accept_one(env, &request, &d.body.candidate, Some(d.identity))?
        } else {
            let mut owned = env.clone();
            owned.caller_module = owner;
            accept_one(&owned, &request, &d.body.candidate, Some(d.identity))?
        };
        out.push((
            Body {
                candidate: &d.body.candidate,
                identity: Some(d.identity),
                interfaces: alloc::vec::Vec::new(),
            },
            checked,
        ));
    }
    Ok(out)
}

fn accept_one(
    env: &noble_kernel::contracts::Env,
    request: &noble_kernel::untrusted::Request,
    candidate: &noble_kernel::untrusted::Candidate,
    identity: Option<u64>,
) -> Result<noble_kernel::untrusted::Checked, crate::diag::Diagnostic> {
    let checked = match noble_kernel::acceptance::check(env, request, candidate) {
        noble_kernel::untrusted::Outcome::Accepted(checked) => checked,
        other => {
            return Err(refuse(
                crate::diag::Code::Subject,
                alloc::format!("body of {identity:?} is not kernel-accepted: {other:?}"),
            ))
        }
    };
    // No node may hide outside the checked derivations.
    let mut covered = alloc::vec![false; candidate.nodes.len()];
    for d in &checked.derivations {
        match covered.get_mut(d.node.0 as usize) {
            Some(slot) => *slot = true,
            None => {
                return Err(refuse(
                    crate::diag::Code::Subject,
                    "derivation names a node outside the arena",
                ))
            }
        }
    }
    if covered.iter().any(|c| !c) {
        return Err(refuse(
            crate::diag::Code::Subject,
            alloc::format!("body of {identity:?} has an unchecked arena node"),
        ));
    }
    if checked.interface.stack_in != request.expected.stack_in
        || checked.interface.stack_out != request.expected.stack_out
        || !checked
            .interface
            .effects
            .is_subset_of(&request.expected.allowed_effects)
    {
        return Err(refuse(
            crate::diag::Code::Subject,
            alloc::format!("body of {identity:?} was not accepted at its expected interface"),
        ));
    }
    Ok(checked)
}

/// The Wasm planner's `derivation()` rule: every derivation recorded for a
/// node must agree on `stack_in`, `stack_out` and `effects`.
fn index_interfaces<'s>(
    mut body: Body<'s>,
    checked: &noble_kernel::untrusted::Checked,
) -> Body<'s> {
    let mut slots: alloc::vec::Vec<Option<Result<noble_kernel::untrusted::Interface, ()>>> =
        alloc::vec![None; body.candidate.nodes.len()];
    for d in &checked.derivations {
        let slot = &mut slots[d.node.0 as usize];
        *slot = match slot.take() {
            None => Some(Ok(d.interface.clone())),
            Some(Ok(prev))
                if prev.stack_in == d.interface.stack_in
                    && prev.stack_out == d.interface.stack_out
                    && prev.effects == d.interface.effects =>
            {
                Some(Ok(prev))
            }
            Some(_) => Some(Err(())),
        };
    }
    body.interfaces = slots.into_iter().map(|s| s.and_then(Result::ok)).collect();
    body
}

/// Named-call targets reachable from the root, by exact resolved identity.
/// A `Named` invocation that is no submission definition is a canonical
/// host row (`test.abort`, live operations); it is not part of the closure
/// and is refused during extraction as a host operation.
///
/// In order: live-slot dispatch has no static closure (`E-LIVE-SLOT`); a
/// definition identity reached again below itself is recursion
/// (`E-RECURSION`, direct or mutual); each reached definition row's supplied
/// dependency list (`Env::deps`) must match its body exactly, as in the Wasm
/// admission — every listed row is invoked and every invoked submission
/// definition is listed (`E-CLOSURE`).
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; both collections are bounded by the validated body's node and closure counts"
)]
#[expect(
    tigerstyle::allocation_in_loop,
    reason = "Owner: noble-ngraph-maintainers; each visited body's child list is moved into the closure table, so one reused buffer cannot be stored for every row."
)]
fn recompute_closure(
    submission: &noble_kernel::execution::Submission,
    bodies: &[Accepted<'_>],
) -> Result<alloc::vec::Vec<u64>, crate::diag::Diagnostic> {
    let env = &submission.environment;
    let target_of = |def: noble_kernel::contracts::Definition| {
        if env.kind(def) == Some(noble_kernel::contracts::Behavior::Named) {
            submission
                .definitions
                .iter()
                .position(|d| d.definition == def)
                .map(|t| t + 1)
        } else {
            None
        }
    };
    // Reachable bodies and their named-call children, in node order.
    let mut children: alloc::vec::Vec<Option<alloc::vec::Vec<usize>>> =
        alloc::vec![None; bodies.len()];
    let mut work = alloc::vec![0usize];
    while let Some(b) = work.pop() {
        if children[b].is_some() {
            continue;
        }
        let body = &bodies[b].0;
        let mut out = alloc::vec::Vec::new();
        for (n, node) in body.candidate.nodes.iter().enumerate() {
            match node {
                noble_kernel::untrusted::Node::SlotInvoke { .. } => return Err(at(
                    crate::diag::Code::LiveSlot,
                    body.identity,
                    u32::try_from(n).ok(),
                    "live-slot dispatch selects its target at run time; no static closure exists",
                )),
                noble_kernel::untrusted::Node::SignedSelf { .. } => {
                    return Err(at(
                        crate::diag::Code::Recursion,
                        body.identity,
                        u32::try_from(n).ok(),
                        "signed self recursion has no finite numeric-graph closure",
                    ))
                }
                noble_kernel::untrusted::Node::Invocation { def, .. } => {
                    if let Some(t) = target_of(*def) {
                        out.push(t);
                        work.push(t);
                    }
                }
                noble_kernel::untrusted::Node::Literal { .. }
                | noble_kernel::untrusted::Node::Quotation { .. } => {}
            }
        }
        children[b] = Some(out);
    }
    // Recursion by definition identity along any call path (three-colour
    // DFS over bodies; a gray ancestor with the same identity or a back edge
    // is recursion).
    let mut color = alloc::vec![0u8; bodies.len()];
    let mut path: alloc::vec::Vec<(usize, usize)> = alloc::vec![(0, 0)];
    color[0] = 1;
    while let Some(top) = path.last_mut() {
        let (b, next) = *top;
        let kids = children[b].as_deref().unwrap_or(&[]);
        if let Some(&c) = kids.get(next) {
            top.1 += 1;
            let identity = bodies[c].0.identity;
            if color[c] == 1 || path.iter().any(|&(p, _)| bodies[p].0.identity == identity) {
                return Err(refuse(
                    crate::diag::Code::Recursion,
                    alloc::format!(
                        "definition identity {identity:?} is reached again below itself"
                    ),
                ));
            }
            if color[c] == 0 {
                color[c] = 1;
                path.push((c, 0));
            }
        } else {
            color[b] = 2;
            path.pop();
        }
    }
    let mut closure = alloc::vec::Vec::new();
    for (b, kids) in children.iter().enumerate() {
        if b == 0 || kids.is_none() {
            continue;
        }
        closure.push(bodies[b].0.identity.unwrap_or_default());
        let row = submission.definitions[b - 1].definition;
        let listed = usize::try_from(row.0)
            .ok()
            .and_then(|r| env.deps.get(r))
            .map_or(&[][..], alloc::vec::Vec::as_slice);
        let invoked: alloc::vec::Vec<_> = bodies[b]
            .0
            .candidate
            .nodes
            .iter()
            .filter_map(|n| match n {
                noble_kernel::untrusted::Node::Invocation { def, .. } => Some(*def),
                _ => None,
            })
            .collect();
        let has_named_unlisted = invoked
            .iter()
            .any(|d| target_of(*d).is_some() && !listed.contains(d));
        if listed.iter().any(|l| !invoked.contains(l)) || has_named_unlisted {
            return Err(refuse(
                crate::diag::Code::Closure,
                alloc::format!(
                    "supplied dependency list {listed:?} of {:?} differs from its body's invocations",
                    bodies[b].0.identity
                ),
            ));
        }
    }
    closure.sort_unstable();
    closure.dedup();
    Ok(closure)
}

fn has_resource(ty: &noble_kernel::types::Ty) -> bool {
    let mut work = alloc::vec![ty];
    while let Some(t) = work.pop() {
        match t {
            noble_kernel::types::Ty::Resource(_) | noble_kernel::types::Ty::LiveRef(..) => {
                return true
            }
            noble_kernel::types::Ty::Pair(a, b) | noble_kernel::types::Ty::Sum(a, b) => {
                work.push(a);
                work.push(b);
            }
            noble_kernel::types::Ty::List(a) => work.push(a),
            noble_kernel::types::Ty::Program(i, o, _) => work.extend(i.iter().chain(o.iter())),
            noble_kernel::types::Ty::Nominal(_, shape) => push_shape(&mut work, shape),
            noble_kernel::types::Ty::GenericNominal(_, args, shape) => {
                work.extend(args.iter());
                push_shape(&mut work, shape);
            }
            noble_kernel::types::Ty::Unit
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Text
            | noble_kernel::types::Ty::Syntax
            | noble_kernel::types::Ty::Contract
            | noble_kernel::types::Ty::Evidence
            | noble_kernel::types::Ty::Certified => {}
        }
    }
    false
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the traversal worklist must grow while nested shapes are expanded"
)]
fn push_shape<'a>(
    work: &mut alloc::vec::Vec<&'a noble_kernel::types::Ty>,
    shape: &'a noble_kernel::types::NominalShape,
) {
    match shape {
        noble_kernel::types::NominalShape::Opaque(a) => work.push(a),
        noble_kernel::types::NominalShape::Variant(a, b) => {
            work.push(a);
            work.push(b);
        }
    }
}

/// Root interface item: `I64`/`Bool` scalars, `Unit` or pairs of them.
/// A Program input is never statically known (`E-DYNAMIC-RUN`); a Program
/// output exposes its recipe (`E-PROGRAM-OUTPUT`, V-OBS-01). An item that
/// nests more than [`crate::ir::MAX_SHAPE_DEPTH`] pairs has no `ngraph-ir/0` shape tree
/// (`E-AGGREGATE`); the Core frontend's 256-node type bound never produces
/// one, so only a forged submission reaches this refusal.
fn classify(
    ty: &noble_kernel::types::Ty,
    program: crate::diag::Code,
) -> Result<(), crate::diag::Diagnostic> {
    // Each entry carries the number of pairs enclosing it.
    let mut work = alloc::vec![(ty, 0u32)];
    while let Some((t, open)) = work.pop() {
        match t {
            noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::Unit => {}
            noble_kernel::types::Ty::Pair(a, b) => {
                if open >= crate::ir::MAX_SHAPE_DEPTH {
                    return Err(refuse(
                        crate::diag::Code::Aggregate,
                        alloc::format!(
                            "a root interface item nests more than {} pairs",
                            crate::ir::MAX_SHAPE_DEPTH
                        ),
                    ));
                }
                work.push((a, open + 1));
                work.push((b, open + 1));
            }
            noble_kernel::types::Ty::Program(..) => {
                return Err(refuse(
                    program,
                    alloc::format!("Program-valued root interface item {ty:?}"),
                ));
            }
            other => {
                return Err(refuse(
                    crate::diag::Code::Aggregate,
                    alloc::format!("{other:?} is not an ngraph-i64/0 interface type"),
                ))
            }
        }
    }
    Ok(())
}

/// `I64`/`Bool` scalar type of a kernel type.
pub(super) const fn scalar(ty: &noble_kernel::types::Ty) -> Option<crate::ir::ScalarType> {
    match ty {
        noble_kernel::types::Ty::I64 => Some(crate::ir::ScalarType::I64),
        noble_kernel::types::Ty::Bool => Some(crate::ir::ScalarType::Bool),
        _ => None,
    }
}

/// Subject identity: the exact root request, environment and body (including
/// its text-literal payloads, which the candidate's `Lit::Text` does not
/// carry) plus every closure definition specialization (identity, contract
/// entry, body, text literals, expected interface), encoded with the kernel's
/// derived `Debug` rendering under a domain tag. The encoding is
/// deterministic for one kernel revision; it is a producer digest, not
/// independent validation.
fn subject_of(
    ids: &dyn IdentityScheme,
    submission: &noble_kernel::execution::Submission,
    closure: &[u64],
) -> crate::ir::Subject {
    let mut parts: alloc::vec::Vec<alloc::vec::Vec<u8>> = alloc::vec![
        b"noble.ngraph.subject.kernel-debug.v0".to_vec(),
        alloc::format!("{:?}", submission.request.expected).into_bytes(),
        alloc::format!("{:?}", submission.environment).into_bytes(),
        alloc::format!("{:?}", submission.body.candidate).into_bytes(),
        alloc::format!("{:?}", submission.body.texts).into_bytes(),
    ];
    let mut defs: alloc::vec::Vec<_> = submission
        .definitions
        .iter()
        .filter(|d| closure.binary_search(&d.identity).is_ok())
        .collect();
    defs.sort_by_key(|d| (d.identity, d.definition.0));
    for d in defs {
        parts.push(d.identity.to_le_bytes().to_vec());
        parts.push(
            alloc::format!(
                "{:?}|{:?}|{:?}|{:?}",
                d.definition,
                d.expected,
                d.body.candidate,
                d.body.texts
            )
            .into_bytes(),
        );
    }
    let refs: alloc::vec::Vec<&[u8]> = parts.iter().map(alloc::vec::Vec::as_slice).collect();
    crate::ir::Subject::derive(
        ids,
        crate::ir::SubjectKind::Extracted,
        &refs,
        closure.to_vec(),
    )
}

#[cfg(test)]
mod tests {
    /// `I64` under `depth` nested pairs.
    fn nested(depth: u32) -> noble_kernel::types::Ty {
        let mut t = noble_kernel::types::Ty::I64;
        for _ in 0..depth {
            t = noble_kernel::types::Ty::Pair(
                alloc::boxed::Box::new(t),
                alloc::boxed::Box::new(noble_kernel::types::Ty::Unit),
            );
        }
        t
    }

    #[test]
    fn interface_items_nest_at_most_the_shape_bound() {
        crate::extract::admission::classify(
            &nested(crate::ir::MAX_SHAPE_DEPTH),
            crate::diag::Code::DynamicRun,
        )
        .expect("the deepest extractable item");
        let d = crate::extract::admission::classify(
            &nested(crate::ir::MAX_SHAPE_DEPTH + 1),
            crate::diag::Code::DynamicRun,
        )
        .expect_err("one more pair");
        assert_eq!(d.code, crate::diag::Code::Aggregate);
    }
}
