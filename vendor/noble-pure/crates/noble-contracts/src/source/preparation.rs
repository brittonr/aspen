#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; preparation's mutable parameters are only its request-local meter; draft trees and output buffers are newly owned, while source bytes, supplied types, and the committed session are borrowed unchanged until an explicit later commit."
)]

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
use alloc::borrow::Cow;
pub(in crate::source) mod symbolic;

#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
impl super::Session {
    pub fn prepare(
        &self,
        source_bytes: &[u8],
        inputs: &[noble_kernel::types::Ty],
        limits: crate::Limits,
    ) -> Result<super::Prepared, super::Error> {
        self.prepare_signed(source_bytes, inputs, None, limits, limits.bytes)
    }

    pub(super) fn prepare_with_retention(
        &self,
        source_bytes: &[u8],
        inputs: &[noble_kernel::types::Ty],
        limits: crate::Limits,
        retained_bytes: u32,
    ) -> Result<super::Prepared, super::Error> {
        self.prepare_signed(source_bytes, inputs, None, limits, retained_bytes)
    }

    pub(super) fn prepare_signed(
        &self,
        source_bytes: &[u8],
        inputs: &[noble_kernel::types::Ty],
        signature: Option<&noble_kernel::words::Scheme>,
        limits: crate::Limits,
        retained_bytes: u32,
    ) -> Result<super::Prepared, super::Error> {
        let mut meter = crate::Meter::pooled(limits, self.request_work());
        let mut result = self.prepare_signed_metered(
            source_bytes,
            inputs,
            signature,
            retained_bytes,
            &mut meter,
        );
        let work_used = meter.spent();
        match &mut result {
            Ok(prepared) => prepared.work_used = work_used,
            Err(failure) => failure.work_used = work_used,
        }
        result
    }

    /// Remaining work of the host-selected request this session shares
    /// across every checking stage, if the host opted in.
    pub(crate) fn request_work(&self) -> Option<&alloc::rc::Rc<core::cell::Cell<u32>>> {
        self.declared.as_ref()?.request_work.as_ref()
    }

    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; the operand is a u32 table index and the cast is a widening that cannot fail where usize is at least 32 bits; the portable form would add fallible plumbing to infallible lookups"
    )]
    pub(super) fn prepare_signed_metered(
        &self,
        source_bytes: &[u8],
        inputs: &[noble_kernel::types::Ty],
        signature: Option<&noble_kernel::words::Scheme>,
        retained_bytes: u32,
        meter: &mut crate::Meter,
    ) -> Result<super::Prepared, super::Error> {
        let parsed = if self.declared.is_some() {
            super::parsing::parse_declared(source_bytes, meter)
        } else {
            super::parsing::parse(source_bytes, meter)
        };
        let (mut tree, name) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => return Err(super::Error::at(super::Stage::Parse, error)),
        };
        if self.live_slots.is_some()
            || self
                .declared
                .as_ref()
                .is_some_and(|context| context.declared_text)
        {
            attempt!(super::locals::lower(&mut tree, meter)
                .map_err(|error| super::Error::at(super::Stage::Resolve, error)));
        }
        let alias_candidates = if name.is_some() {
            attempt!(alias_candidates(&tree, self, meter)
                .map_err(|error| super::Error::at(super::Stage::Resolve, error)))
        } else {
            alloc::vec::Vec::new()
        };
        let is_signed_owner = signature.is_some()
            && self
                .declared
                .as_ref()
                .is_some_and(|context| context.signed_self);
        if let Err(error) =
            super::resolution::resolve(&mut tree, name.as_deref(), self, is_signed_owner, meter)
        {
            return Err(super::Error::at(super::Stage::Resolve, error));
        }
        let alias_uses = if alias_candidates.is_empty() {
            alloc::vec::Vec::new()
        } else {
            attempt!(checked_alias_uses(&tree, self, &alias_candidates, meter)
                .map_err(|error| super::Error::at(super::Stage::Check, error)))
        };
        if name.is_some()
            && self.live_slots.is_some()
            && tree
                .nodes
                .iter()
                .any(|node| matches!(node.kind, super::Kind::Call(super::Target::SlotInvoke(_))))
        {
            return Err(super::Error::at(
                super::Stage::Check,
                crate::invalid(
                    tree.span,
                    "live-slot declarations cannot contain borrowed dispatch",
                ),
            ));
        }
        if let Err(error) = super::preflight::check(inputs, self, tree.span, meter) {
            return Err(super::Error::at(super::Stage::Check, error));
        }
        let selected_root = if tree.body.len() == 1 {
            tree.nodes
                .get(tree.body[0] as usize)
                .and_then(|node| match &node.kind {
                    super::Kind::Call(super::Target::Named(index)) => Some(*index),
                    _ => None,
                })
        } else {
            None
        };
        let (extra, mode) = if name.is_some() {
            (
                attempt!(source_bytes.len().checked_add(4).ok_or_else(|| {
                    super::Error::at(
                        super::Stage::Check,
                        super::exhausted(tree.span, "retained definition source length overflows"),
                    )
                })),
                super::inference::Mode::Declaration,
            )
        } else {
            (0, super::inference::Mode::Submission)
        };
        if let Err(error) = self.retained(extra, retained_bytes, tree.span, meter) {
            return Err(super::Error::at(super::Stage::Check, error));
        }
        let mut environment = match self.environment() {
            Ok(environment) => environment,
            Err(error) => return Err(super::Error::at(super::Stage::Check, error)),
        };
        // An actual unpublished owner interface occupies this transaction's
        // prospective Env row while its body is checked. It is never installed
        // into the committed namespace or exposed as a raw graph self edge.
        let prospective_owner = if is_signed_owner
            && tree
                .nodes
                .iter()
                .any(|node| matches!(node.kind, super::Kind::Call(super::Target::SelfOwner)))
        {
            let Some(scheme) = signature else {
                return Err(super::Error::at(
                    super::Stage::Check,
                    crate::invalid(tree.span, "signed owner has no signature"),
                ));
            };
            let Some(owner) = self.declared.as_ref().and_then(|context| context.owner) else {
                return Err(super::Error::at(
                    super::Stage::Check,
                    crate::invalid(tree.span, "signed owner has no module"),
                ));
            };
            let def = noble_kernel::contracts::Definition(attempt!(crate::index(
                environment.defs.len(),
                tree.span,
            )
            .map_err(|problem| super::Error::at(super::Stage::Check, problem))));
            attempt!(meter
                .node(tree.span)
                .map_err(|problem| super::Error::at(super::Stage::Check, problem)));
            // Copy-on-write: only a signed declaration pays for an owned
            // prospective environment row.
            let owned = environment.to_mut();
            owned.defs.push(scheme.clone());
            owned.kinds.push(noble_kernel::contracts::Behavior::Named);
            owned.deps.push(alloc::vec::Vec::new());
            owned.definition_owners.push(Some(owner));
            Some(def)
        } else {
            None
        };
        let scope = super::inference::Scope {
            root: &tree,
            session: self,
            environment: &environment,
            root_signature: signature,
            prospective_owner,
        };
        let state = match super::inference::infer(scope, mode, inputs, meter) {
            Ok(state) => state,
            Err(error) => return Err(super::Error::at(super::Stage::Check, error)),
        };
        let signed_relation = match prospective_owner {
            Some(owner) => {
                let scheme = signature.ok_or_else(|| {
                    super::Error::at(
                        super::Stage::Check,
                        crate::invalid(tree.span, "signed owner has no signature"),
                    )
                })?;
                Some(attempt!(symbolic::check(
                    &tree,
                    &state,
                    scheme,
                    source_bytes,
                    owner,
                    self,
                    &environment,
                    meter
                )
                .map_err(|problem| super::Error::at(
                    super::Stage::Check,
                    problem
                ))))
            }
            None => None,
        };
        let mut addition = alloc::vec::Vec::new();
        let (definition, submission, checked, output) = match name {
            Some(name) => {
                let definition = match declaration(
                    tree,
                    name,
                    source_bytes,
                    signature,
                    signed_relation,
                    alias_uses,
                    self,
                    meter,
                ) {
                    Ok((definition, bytes)) => {
                        addition = bytes;
                        definition
                    }
                    Err(error) => return Err(super::Error::at(super::Stage::Check, error)),
                };
                (Some(definition), None, None, inputs.to_vec())
            }
            None => {
                if let Cow::Borrowed(borrowed) = &environment {
                    if borrowed.defs.len()
                        > noble_kernel::contracts::CALCULATOR_ENVIRONMENT_ROW_LIMIT
                    {
                        return Err(super::Error::at(
                            super::Stage::Check,
                            super::exhausted(
                                tree.span,
                                "calculator environment row limit exceeded",
                            ),
                        ));
                    }
                    let budget = match borrowed.calculator_clone_budget() {
                        Some(budget) => budget,
                        None => {
                            return Err(super::Error::at(
                                super::Stage::Check,
                                super::exhausted(
                                    tree.span,
                                    "calculator environment clone bound exceeded",
                                ),
                            ))
                        }
                    };
                    if budget.reserved_bytes > u64::from(meter.limits.work) {
                        return Err(super::Error::at(
                            super::Stage::Check,
                            super::exhausted(
                                tree.span,
                                "calculator environment clone work exceeded",
                            ),
                        ));
                    }
                    let bytes = match u32::try_from(budget.bytes) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            return Err(super::Error::at(
                                super::Stage::Check,
                                super::exhausted(
                                    tree.span,
                                    "calculator environment clone work exceeded",
                                ),
                            ))
                        }
                    };
                    if let Err(error) = meter.charge(bytes, tree.span) {
                        return Err(super::Error::at(super::Stage::Check, error));
                    }
                }
                let (submission, checked, output) = attempt!(super::emission::emit(
                    state,
                    environment.into_owned(),
                    &self.definitions,
                    source_bytes.len(),
                    meter
                ));
                (None, Some(submission), Some(checked), output)
            }
        };
        Ok(super::Prepared {
            generation: self.generation,
            history: self.history.clone(),
            hosts: self.hosts,
            text_cursor: self.text_cursor,
            live_selected: self.live_selected.clone(),
            live_slots: self.live_slots.is_some(),
            live_slot_owner: self.live_slots.as_ref().map(|slots| slots.owner.clone()),
            limits: meter.limits,
            calculator: self.calculator,
            checked_i64: self.checked_i64,
            boundary: self.bindings.as_ref().map(|bindings| bindings.key.clone()),
            definition,
            selected_root,
            addition,
            submission,
            checked,
            work_used: 0,
            output,
        })
    }

    pub(super) fn environment(
        &self,
    ) -> Result<Cow<'_, noble_kernel::contracts::Env>, crate::Diagnostic> {
        if let Some(declared) = &self.declared {
            if self.calculator {
                return Ok(Cow::Borrowed(&declared.environment));
            }
            return Ok(Cow::Owned(declared.environment.clone()));
        }
        if let Some(profile) = &self.live_slots {
            return Ok(Cow::Owned(profile.environment.clone()));
        }
        match &self.bindings {
            Some(bindings) => Ok(Cow::Owned(bindings.environment.clone())),
            None if self.calculator => noble_kernel::contracts::calculator_environment()
                .map(Cow::Owned)
                .map_err(|_| crate::internal(crate::Span { start: 0, end: 0 })),
            None if self.checked_i64 => super::checked_i64_environment().map(Cow::Owned),
            None if self.text_cursor => noble_kernel::contracts::text_cursor_environment()
                .map(Cow::Owned)
                .map_err(|_| crate::internal(crate::Span { start: 0, end: 0 })),
            None if self.live_selected.is_some() => super::live_environment().map(Cow::Owned),
            None => super::environment().map(Cow::Owned),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; retained-byte admission uses checked conversions and metering whose rejection paths construct owned diagnostics."
    )]
    fn retained(
        &self,
        extra: usize,
        retained_bytes: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let history_bytes = attempt!(crate::index(self.history.len(), span));
        let limit_bytes = attempt!(crate::offset(retained_bytes, span));
        let total = attempt!(self.history.len().checked_add(extra).ok_or_else(|| {
            super::exhausted(span, "retained definition history byte count overflows")
        }));
        if total > limit_bytes {
            return Err(super::exhausted(
                span,
                "retained definition history byte limit exceeded",
            ));
        }
        meter.charge(history_bytes, span)
    }
}

/// Preserve exact source node ordinals before resolution erases spellings.
/// Only current outside aliases on a top-level named body are mutable;
/// module-local imports belong to that module's immutable source snapshot.
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; growth is bounded by the checked input or admission limits that drive the loop."
)]
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-contracts; an alias length is a String length below isize::MAX, so alias.len() + 1 cannot overflow usize."
)]
fn alias_candidates(
    tree: &super::Tree,
    session: &super::Session,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<(u32, usize)>, crate::Diagnostic> {
    let Some(context) = session.declared.as_ref() else {
        return Ok(alloc::vec::Vec::new());
    };
    if context.owner.is_some() || context.aliases.is_empty() {
        return Ok(alloc::vec::Vec::new());
    }
    let mut candidates = alloc::vec::Vec::new();
    for (node_index, node) in tree.nodes.iter().enumerate() {
        attempt!(meter.charge(1, node.span));
        let super::Kind::Word(word) = &node.kind else {
            continue;
        };
        for (alias_index, (alias, _)) in context.aliases.iter().enumerate() {
            attempt!(meter.charge(1, node.span));
            if word.len() > alias.len() + 1
                && word.starts_with(alias.as_bytes())
                && word[alias.len()] == b'.'
            {
                candidates.push((attempt!(crate::index(node_index, node.span)), alias_index));
                break;
            }
        }
    }
    Ok(candidates)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::fragile_exhaustive_enum_match,
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added; growth is bounded by the checked input or admission limits that drive the loop."
)]
fn checked_alias_uses(
    tree: &super::Tree,
    session: &super::Session,
    candidates: &[(u32, usize)],
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<super::AliasUse>, crate::Diagnostic> {
    let context = session
        .declared
        .as_ref()
        .ok_or_else(|| crate::internal(tree.span))?;
    let mut uses: alloc::vec::Vec<super::AliasUse> = alloc::vec::Vec::new();
    for &(node_index, alias_index) in candidates {
        let node = tree
            .nodes
            .get(attempt!(crate::offset(node_index, tree.span)))
            .ok_or_else(|| crate::internal(tree.span))?;
        let (alias, owner) = context
            .aliases
            .get(alias_index)
            .ok_or_else(|| crate::internal(node.span))?;
        let super::Kind::Call(target) = &node.kind else {
            return Err(crate::invalid(
                node.span,
                "checked import alias lost its source word",
            ));
        };
        match *target {
            super::Target::Named(index) => {
                let definition = session
                    .definitions
                    .get(attempt!(crate::offset(index, node.span)))
                    .ok_or_else(|| crate::internal(node.span))?;
                if definition.owner != Some(*owner) {
                    return Err(crate::invalid(
                        node.span,
                        "checked alias call has a foreign module owner",
                    ));
                }
            }
            super::Target::Builtin(_) => {}
            super::Target::SelfOwner | super::Target::SlotInvoke(_) => {
                return Err(crate::invalid(
                    node.span,
                    "checked alias call has an unsupported target",
                ));
            }
        }
        attempt!(meter.charge(1, node.span));
        if let Some(existing) = uses
            .iter_mut()
            .find(|entry| entry.spelling == alias.as_str() && entry.owner == *owner)
        {
            existing.nodes.push((node_index, *target));
        } else {
            attempt!(meter.node(node.span));
            uses.push(super::AliasUse {
                spelling: alias.clone(),
                owner: *owner,
                nodes: alloc::vec![(node_index, *target)],
            });
        }
    }
    Ok(uses)
}
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
fn declaration(
    tree: super::Tree,
    name: alloc::string::String,
    source_bytes: &[u8],
    signature: Option<&noble_kernel::words::Scheme>,
    signed: Option<super::SignedRelation>,
    alias_uses: alloc::vec::Vec<super::AliasUse>,
    session: &super::Session,
    meter: &mut crate::Meter,
) -> Result<(super::Named, alloc::vec::Vec<u8>), crate::Diagnostic> {
    // Inference has solved the open graph, including every dependency body.
    // No Unit instance decides generic validity.
    let identity = attempt!(super::resolution::comparison::identity(
        &tree, session, signature, meter
    ));
    let count = attempt!(crate::index(source_bytes.len(), tree.span));
    attempt!(meter.charge(count.saturating_add(4), tree.span));
    let mut addition = alloc::vec::Vec::with_capacity(source_bytes.len().saturating_add(4));
    addition.extend_from_slice(&count.to_le_bytes());
    addition.extend_from_slice(source_bytes);
    Ok((
        super::Named {
            name,
            identity,
            owner: session.declared.as_ref().and_then(|context| context.owner),
            tree,
            signature: signature.cloned(),
            signed,
            alias_uses,
            history_start: None,
        },
        addition,
    ))
}
