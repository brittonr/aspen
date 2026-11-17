//! Tie each accepted named specialization to its resolved original source occurrence.
//! Dynamic definition slots are checked graph edges, never lexical identities.

struct PendingUse {
    source_index: u32,
    definition: noble_kernel::contracts::Definition,
    caller_definition: Option<noble_kernel::contracts::Definition>,
    source_node: Option<u32>,
    source_span: Option<crate::Span>,
    candidate_node: noble_kernel::untrusted::NodeId,
}

fn mismatch(span: crate::Span, reason: &str) -> crate::source::Error {
    super::at(crate::source::Stage::Check, span, reason)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
/// Freeze only imported aliases whose original modules occur in this subject
/// graph. Rebinding a session alias later must not rewrite an earlier goal.
pub(super) fn imports(
    request: &super::Request<'_>,
    original_owner: u64,
    dependencies: &[crate::intrinsic::ResolvedSourceModule],
) -> Result<alloc::vec::Vec<crate::intrinsic::NamedImportBinding>, crate::source::Error> {
    let mut saved = alloc::vec::Vec::new();
    let mut retain = |alias: &str, owner: u64| {
        if let Some(module_index) = dependencies.iter().position(|module| module.owner == owner) {
            saved.push(crate::intrinsic::NamedImportBinding {
                alias: alloc::string::String::from(alias),
                owner,
                module_index,
            });
        }
    };
    if original_owner == request.identity {
        for alias in &request.session.aliases {
            if alias.pending {
                continue;
            }
            let imported = request.session.modules.get(alias.module).ok_or_else(|| {
                mismatch(
                    crate::Span { start: 0, end: 0 },
                    "original imported alias has no retained module",
                )
            })?;
            retain(&alias.spelling, imported.identity);
        }
    } else {
        let original = request
            .session
            .modules
            .iter()
            .find(|module| module.identity == original_owner)
            .ok_or_else(|| {
                mismatch(
                    crate::Span { start: 0, end: 0 },
                    "original named caller module is absent",
                )
            })?;
        for alias in &original.import_aliases {
            retain(&alias.alias, alias.owner);
        }
    }
    Ok(saved)
}

#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
pub(super) fn trace(
    request: &super::Request<'_>,
    root_source_index: u32,
    submission: &noble_kernel::execution::Submission,
    dependencies: &[crate::intrinsic::ResolvedSourceModule],
) -> Result<alloc::vec::Vec<crate::intrinsic::NamedUseOrigin>, crate::source::Error> {
    let span = usize::try_from(root_source_index)
        .ok()
        .and_then(|index| request.session.source.definitions.get(index))
        .map_or(crate::Span { start: 0, end: 0 }, |definition| {
            definition.tree.span
        });
    let candidate = &submission.body.candidate;
    let [root] = candidate.body.as_slice() else {
        return Err(mismatch(
            span,
            "named contract requires one selected subject invocation",
        ));
    };
    if candidate.nodes.len() != 1 || !submission.body.texts.is_empty() {
        return Err(mismatch(
            span,
            "named contract root is not a single source invocation",
        ));
    }
    let Some(noble_kernel::untrusted::Node::Invocation { def, .. }) = usize::try_from(root.0)
        .ok()
        .and_then(|index| candidate.nodes.get(index))
    else {
        return Err(mismatch(
            span,
            "named contract root does not invoke its subject",
        ));
    };
    let mut preceding = None;
    for body in &submission.definitions {
        if preceding.is_some_and(|prior| prior >= body.definition.0) {
            return Err(mismatch(
                span,
                "named specialization slots are not unique and ordered",
            ));
        }
        preceding = Some(body.definition.0);
    }
    let mut pending = alloc::vec![PendingUse {
        source_index: root_source_index,
        definition: *def,
        caller_definition: None,
        source_node: None,
        source_span: None,
        candidate_node: *root,
    }];
    let mut seen = alloc::vec![false; submission.definitions.len()];
    let mut result = alloc::vec::Vec::with_capacity(submission.definitions.len());
    while let Some(use_site) = pending.pop() {
        let Some(named) = usize::try_from(use_site.source_index)
            .ok()
            .and_then(|index| request.session.source.definitions.get(index))
        else {
            return Err(mismatch(
                span,
                "named invocation has no resolved source definition",
            ));
        };
        let original = attempt!(super::source_origin(request, named, span));
        let module_index = dependencies
            .iter()
            .position(|module| module.owner == original.owner)
            .ok_or_else(|| {
                mismatch(
                    original.definition.span,
                    "named invocation owner has no retained source module",
                )
            })?;
        let module = &dependencies[module_index];
        if module.module != original.module_name
            || module.version != original.version
            || module.full_source.as_slice() != original.module_source
        {
            return Err(mismatch(
                original.definition.span,
                "named invocation source module differs from immutable owner",
            ));
        }
        let position = submission
            .definitions
            .binary_search_by_key(&use_site.definition.0, |body| body.definition.0)
            .map_err(|_| {
                mismatch(
                    original.definition.span,
                    "named invocation has no accepted specialization",
                )
            })?;
        if seen[position] {
            return Err(mismatch(
                original.definition.span,
                "named invocation reuses an accepted specialization",
            ));
        }
        seen[position] = true;
        let body = &submission.definitions[position];
        let slot = usize::try_from(body.definition.0).map_err(|_| {
            mismatch(
                original.definition.span,
                "named specialization slot exceeds address space",
            )
        })?;
        if body.identity != named.identity
            || submission.environment.definition_owners.get(slot) != Some(&Some(original.owner))
            || submission.environment.kind(body.definition)
                != Some(noble_kernel::contracts::Behavior::Named)
        {
            return Err(mismatch(
                original.definition.span,
                "named specialization differs from resolved source owner or identity",
            ));
        }
        let tree = &named.tree;
        let candidate = &body.body.candidate;
        if tree.body.len() != candidate.body.len()
            || candidate.nodes.len() != candidate.body.len()
            || !body.body.texts.is_empty()
        {
            return Err(mismatch(
                original.definition.span,
                "named body is not a flat original source body",
            ));
        }
        result.push(crate::intrinsic::NamedUseOrigin {
            definition: use_site.definition,
            definition_identity: named.identity,
            definition_owner: original.owner,
            definition_name: named.name.clone(),
            definition_ordinal: original.ordinal,
            definition_span: original.definition.span,
            body_span: original.definition.body_span,
            module_index,
            caller_definition: use_site.caller_definition,
            source_node: use_site.source_node,
            source_span: use_site.source_span,
            candidate_node: use_site.candidate_node,
        });
        // Stack is LIFO: push later lexical calls first, then visit each use
        // once in the same order as the original, separately parsed body.
        for (source_id, accepted_id) in tree.body.iter().zip(&candidate.body).rev() {
            if source_id != &accepted_id.0 {
                return Err(mismatch(
                    original.definition.span,
                    "named candidate node order differs from original source body",
                ));
            }
            let source_node = attempt!(tree.node(*source_id).map_err(|problem| {
                crate::source::declared::diagnostic(crate::source::Stage::Check, problem)
            }));
            let Some(accepted) = usize::try_from(accepted_id.0)
                .ok()
                .and_then(|index| candidate.nodes.get(index))
            else {
                return Err(mismatch(
                    original.definition.span,
                    "named body has an invalid accepted node",
                ));
            };
            match (&source_node.kind, accepted) {
                (
                    crate::source::Kind::Literal(expected),
                    noble_kernel::untrusted::Node::Literal { lit, .. },
                ) if expected == lit => (),
                (
                    crate::source::Kind::Call(crate::source::Target::Builtin(expected)),
                    noble_kernel::untrusted::Node::Invocation { def, .. },
                ) if *expected == def.0 => (),
                (
                    crate::source::Kind::Call(crate::source::Target::Named(next)),
                    noble_kernel::untrusted::Node::Invocation { def, .. },
                ) => {
                    let start = original
                        .definition
                        .span
                        .start
                        .checked_add(source_node.span.start)
                        .ok_or_else(|| {
                            mismatch(
                                original.definition.span,
                                "named source occurrence span overflows",
                            )
                        })?;
                    let end = original
                        .definition
                        .span
                        .start
                        .checked_add(source_node.span.end)
                        .ok_or_else(|| {
                            mismatch(
                                original.definition.span,
                                "named source occurrence span overflows",
                            )
                        })?;
                    if start < original.definition.body_span.start
                        || end > original.definition.body_span.end
                        || usize::try_from(start)
                            .ok()
                            .zip(usize::try_from(end).ok())
                            .and_then(|(start, end)| original.module_source.get(start..end))
                            .is_none()
                    {
                        return Err(mismatch(
                            original.definition.span,
                            "named invocation is outside its original body",
                        ));
                    }
                    pending.push(PendingUse {
                        source_index: *next,
                        definition: *def,
                        caller_definition: Some(use_site.definition),
                        source_node: Some(*source_id),
                        source_span: Some(crate::Span { start, end }),
                        candidate_node: *accepted_id,
                    });
                }
                _ => {
                    return Err(mismatch(
                        original.definition.span,
                        "accepted named operation differs from original source node",
                    ))
                }
            }
        }
    }
    if seen.iter().any(|visited| !visited) {
        return Err(mismatch(
            span,
            "accepted named graph contains an untraced specialization",
        ));
    }
    Ok(result)
}
