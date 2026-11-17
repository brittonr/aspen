struct StagedUnit {
    session: super::ModuleSession,
    kind: super::ModuleKind,
    submission: Option<noble_kernel::execution::Submission>,
    output: alloc::vec::Vec<noble_kernel::types::Ty>,
    pending: Option<crate::intrinsic::ProofBatch>,
}

const DECLARED_INPUT_SOURCE_BYTES: u32 = 65_536;

impl super::ModuleSession {
    pub fn prepare(
        &self,
        input_bytes: &[u8],
        inputs: &[noble_kernel::types::Ty],
        limits: crate::Limits,
    ) -> Result<super::ModulePrepared, crate::source::Error> {
        // DX-MODULE-05 caps ordinary declared inputs; the separately selected
        // calculator profile keeps its own per-input and clone budgets.
        if !self.source.calculator && limits.bytes > DECLARED_INPUT_SOURCE_BYTES {
            return Err(crate::source::declared::error(
                crate::source::Stage::Parse,
                "declared per-input source byte limit exceeds 65,536",
            ));
        }
        let limit_bytes = attempt!(usize::try_from(limits.bytes).map_err(|_| {
            crate::source::declared::error(
                crate::source::Stage::Parse,
                "source byte limit exceeds the host address space",
            )
        }));
        if input_bytes.len() > limit_bytes {
            // A budget refusal, so it is exhausted rather than invalid, exactly
            // like the Core lexer's identical check: a larger budget can admit it.
            let end = attempt!(
                crate::index(input_bytes.len(), crate::source::declared::SPAN).map_err(|problem| {
                    crate::source::declared::diagnostic(crate::source::Stage::Parse, problem)
                })
            );
            return Err(crate::source::declared::diagnostic(
                crate::source::Stage::Parse,
                crate::source::exhausted(
                    crate::Span { start: 0, end },
                    "source byte limit exceeded",
                ),
            ));
        }
        // Only modules retain the candidate source. If the aggregate would overflow,
        // classify the first lexical token before parsing or cloning prior sources.
        if let Err(error) = self.check_retained(input_bytes.len()) {
            let mut meter = crate::Meter::pooled(limits, self.source.request_work());
            let mut scanner = attempt!(crate::source::lexer::Scanner::new_declared(
                input_bytes,
                &mut meter,
            )
            .map_err(|problem| crate::source::declared::diagnostic(
                crate::source::Stage::Parse,
                problem
            )));
            let first = attempt!(scanner.next(&mut meter).map_err(|problem| {
                crate::source::declared::diagnostic(crate::source::Stage::Parse, problem)
            }));
            if matches!(first, Some(crate::source::lexer::Token {
                kind: crate::source::lexer::TokenKind::Word(ref spelling),
                ..
            }) if spelling == b"module")
            {
                return Err(error);
            }
        }
        let unit = attempt!(crate::source::declared::parsing::parse(
            input_bytes,
            limits,
            self.source.request_work()
        )
        .map_err(|problem| {
            crate::source::declared::diagnostic(crate::source::Stage::Parse, problem)
        }));
        if matches!(
            &unit,
            crate::source::declared::parsing::ParsedUnit::Definition
        ) {
            let extra = attempt!(input_bytes.len().checked_add(4).ok_or_else(|| {
                crate::source::declared::error(
                    crate::source::Stage::Check,
                    "retained definition source length overflow",
                )
            }));
            attempt!(self.check_retained(extra));
        }
        if self.source.calculator {
            let environment = attempt!(self.source.declared.as_ref().ok_or_else(|| {
                crate::source::declared::diagnostic(
                    crate::source::Stage::Check,
                    crate::source::exhausted(
                        super::super::SPAN,
                        "calculator source environment missing",
                    ),
                )
            }));
            if environment.environment.defs.len()
                > noble_kernel::contracts::CALCULATOR_ENVIRONMENT_ROW_LIMIT
            {
                return Err(crate::source::declared::diagnostic(
                    crate::source::Stage::Check,
                    crate::source::exhausted(
                        super::super::SPAN,
                        "calculator environment row limit exceeded",
                    ),
                ));
            }
            let budget = attempt!(environment
                .environment
                .calculator_clone_budget()
                .ok_or_else(|| crate::source::declared::diagnostic(
                    crate::source::Stage::Check,
                    crate::source::exhausted(
                        super::super::SPAN,
                        "calculator environment clone bound exceeded"
                    )
                )));
            let copies =
                attempt!(budget
                    .reserved_bytes
                    .checked_mul(2)
                    .filter(|total| *total
                        <= noble_kernel::contracts::CALCULATOR_ENV_CLONE_FANOUT_LIMIT)
                    .ok_or_else(|| crate::source::declared::diagnostic(
                        crate::source::Stage::Check,
                        crate::source::exhausted(
                            super::super::SPAN,
                            "calculator environment clone fanout exceeded"
                        )
                    )));
            if copies > u64::from(limits.work) {
                return Err(crate::source::declared::diagnostic(
                    crate::source::Stage::Check,
                    crate::source::exhausted(
                        super::super::SPAN,
                        "calculator environment clone work exceeded",
                    ),
                ));
            }
        }
        let StagedUnit {
            session: mut staged,
            kind,
            submission,
            output,
            pending,
        } = attempt!(self
            .snapshot()
            .stage_unit(unit, input_bytes, inputs, limits));
        if staged.source.calculator
            && staged.source.declared.as_ref().is_none_or(|context| {
                context.environment.defs.len()
                    > noble_kernel::contracts::CALCULATOR_ENVIRONMENT_ROW_LIMIT
            })
        {
            return Err(crate::source::declared::diagnostic(
                crate::source::Stage::Check,
                crate::source::exhausted(
                    super::super::SPAN,
                    "calculator environment row limit exceeded",
                ),
            ));
        }
        let rebuilt_import_definitions = if kind == super::ModuleKind::Import {
            attempt!(self.rebuild_import_alias(&mut staged, limits))
        } else {
            None
        };
        attempt!(staged.check_retained(0));
        let signed_expression = if kind == super::ModuleKind::Expression
            && (self
                .source
                .declared
                .as_ref()
                .is_some_and(|context| context.declared_text)
                || submission.as_ref().is_some_and(|program| {
                    program.definitions.iter().any(|definition| {
                        definition.body.candidate.nodes.iter().any(|node| {
                            matches!(node, noble_kernel::untrusted::Node::SignedSelf { .. })
                        })
                    })
                })) {
            input_bytes.to_vec()
        } else {
            alloc::vec::Vec::new()
        };
        let mut prepared = super::ModulePrepared {
            kind,
            generation: self.generation,
            limits,
            previous_source_generation: self.source.generation,
            previous_source_definitions_len: self.source.definitions.len(),
            previous_modules: self.modules.clone(),
            previous_aliases: self.aliases.clone(),
            previous_next_module_identity: self.next_module_identity,
            previous_history: self.source.history.clone(),
            staged,
            submission,
            output,
            pending,
            signed_expression,
            rebuilt_import_definitions,
        };
        if !prepared.signed_expression.is_empty() {
            attempt!(prepared.verify_signed_origins());
            let submission = prepared.submission.as_ref().ok_or_else(|| {
                crate::source::declared::error(
                    crate::source::Stage::Check,
                    "signed expression lacks its concrete checked submission",
                )
            })?;
            prepared.staged = attempt!(prepared.staged.record_signed_certified(submission));
            attempt!(prepared.staged.check_retained(0));
        }
        Ok(prepared)
    }

    fn stage_unit(
        mut self,
        unit: crate::source::declared::parsing::ParsedUnit,
        input_bytes: &[u8],
        inputs: &[noble_kernel::types::Ty],
        limits: crate::Limits,
    ) -> Result<StagedUnit, crate::source::Error> {
        let is_expression = matches!(
            &unit,
            crate::source::declared::parsing::ParsedUnit::Expression
        );
        let (kind, submission, output, pending) = match unit {
            crate::source::declared::parsing::ParsedUnit::Module {
                name,
                version,
                members,
            } => {
                let (next, pending) = attempt!(self.register(super::register::Registration {
                    name,
                    version,
                    members,
                    source: input_bytes,
                    inputs,
                    limits,
                }));
                self = next;
                (
                    super::ModuleKind::Module,
                    None,
                    alloc::vec::Vec::new(),
                    pending,
                )
            }
            crate::source::declared::parsing::ParsedUnit::Import {
                name,
                version,
                alias,
            } => {
                self = attempt!(self.import(&name, version, alias));
                (super::ModuleKind::Import, None, inputs.to_vec(), None)
            }
            crate::source::declared::parsing::ParsedUnit::Definition
            | crate::source::declared::parsing::ParsedUnit::Expression => {
                if is_expression {
                    self = attempt!(self.pending_signed_expression_words());
                }
                let mut prepared = attempt!(self.source.prepare_with_retention(
                    input_bytes,
                    inputs,
                    limits,
                    self.retained_source_limit,
                ));
                let kind = if prepared.is_definition() {
                    super::ModuleKind::Definition
                } else {
                    super::ModuleKind::Expression
                };
                if is_expression != (kind == super::ModuleKind::Expression) {
                    return Err(crate::source::declared::error(
                        crate::source::Stage::Check,
                        "source unit changed between module and expression parsing",
                    ));
                }
                if prepared
                    .definition
                    .as_ref()
                    .is_some_and(|definition| self.reserved_word(&definition.name))
                {
                    return Err(crate::source::declared::error(
                        crate::source::Stage::Resolve,
                        "definition conflicts with a qualified module export",
                    ));
                }
                let submission = prepared.submission.take();
                let output = core::mem::take(&mut prepared.output);
                attempt!(self.source.commit(prepared));
                if kind == super::ModuleKind::Definition || is_expression {
                    self = attempt!(self.outside_words());
                }
                (kind, submission, output, None)
            }
        };
        Ok(StagedUnit {
            session: self,
            kind,
            submission,
            output,
            pending,
        })
    }

    /// Publish the entire scanned declaration/import as one stale-checked transition.
    /// The returned session is the prior one on refusal and the staged one on success.
    pub fn commit(
        self,
        mut prepared: super::ModulePrepared,
    ) -> (Self, Result<(), crate::source::Error>) {
        if prepared.pending.is_some() {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "proof declarations require independent host recheck before module publication",
                )),
            );
        }
        if !self.current_snapshot(&prepared) {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "stale or foreign module namespace snapshot",
                )),
            );
        }
        if self.import_rebind_required(&prepared) && prepared.rebuilt_import_definitions.is_none() {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "import rebind lacks its checked transitive dependent rebuild",
                )),
            );
        }
        let Some(next) = self.generation.checked_add(1) else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "generation limit exceeded",
                )),
            );
        };
        prepared.staged.generation = next;
        (prepared.staged, Ok(()))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn current_snapshot(&self, prepared: &super::ModulePrepared) -> bool {
        self.generation == prepared.generation
            && self.retained_source_limit == prepared.staged.retained_source_limit
            && self.source.generation == prepared.previous_source_generation
            && self.source.calculator == prepared.staged.source.calculator
            && self.source.definitions.len() == prepared.previous_source_definitions_len
            && prepared
                .staged
                .source
                .definitions
                .get(..prepared.previous_source_definitions_len)
                == Some(self.source.definitions.as_slice())
            && self
                .source
                .declared
                .as_ref()
                .map(|context| (context.signed_self, context.declared_text))
                == prepared
                    .staged
                    .source
                    .declared
                    .as_ref()
                    .map(|context| (context.signed_self, context.declared_text))
            && self.bindings == prepared.staged.bindings
            && self.modules == prepared.previous_modules
            && self.aliases == prepared.previous_aliases
            && self.next_module_identity == prepared.previous_next_module_identity
            && self.source.history == prepared.previous_history
    }

    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    /// This API trusts the **host callback**, not any no_std proof marker.
    /// The callback must perform bounded source checking and a separate pinned
    /// independent exact-claim Lean check before returning checked results.
    pub fn commit_verified<F>(
        self,
        mut prepared: super::ModulePrepared,
        recheck: F,
    ) -> (Self, Result<(), crate::source::Error>)
    where
        F: FnOnce(
            &crate::intrinsic::ProofBatch,
        )
            -> Result<alloc::vec::Vec<crate::intrinsic::CheckedProof>, alloc::string::String>,
    {
        if !self.current_snapshot(&prepared) {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "stale or foreign module namespace snapshot",
                )),
            );
        }
        let Some(batch) = prepared.pending.as_ref() else {
            return self.commit(prepared);
        };
        let Some(module) = prepared.staged.modules.last() else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "pending proof has no staged module",
                )),
            );
        };
        if prepared.kind != super::ModuleKind::Module
            || batch.generation != prepared.generation
            || batch.module != module.name
            || batch.version != module.version
            || batch.source != module.source
        {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "proof batch is not bound to the staged module source",
                )),
            );
        }
        // A host callback supplies Lean evidence, but cannot substitute a
        // different source claim or paper over an invalid proof declaration.
        // Check the exact bounded source batch before invoking that callback;
        // the selected host still must recheck the resulting term in Lean.
        let source_checked = match crate::intrinsic::check_batch(batch, prepared.limits) {
            Ok(checked) => checked,
            Err(problem) => {
                return (
                    self,
                    Err(crate::source::declared::diagnostic(
                        crate::source::Stage::Acceptance,
                        problem,
                    )),
                );
            }
        };
        let checked = match recheck(batch) {
            Ok(checked) => checked,
            Err(message) => {
                return (
                    self,
                    Err(crate::source::declared::error(
                        crate::source::Stage::Acceptance,
                        &message,
                    )),
                );
            }
        };
        if checked.len() != batch.obligations.len()
            || checked.iter().zip(&source_checked).any(|(result, source)| {
                result.claim != source.claim || result.lean_term != source.lean_term
            })
            || checked
                .iter()
                .zip(&batch.obligations)
                .any(|(result, obligation)| {
                    result.name != obligation.name
                        || !revision(&result.model_revision)
                        || !revision(&result.checker_revision)
                        || result.kind
                            != match &obligation.goal {
                                crate::intrinsic::PendingGoal::Pure { .. } => {
                                    crate::intrinsic::ProofKind::Pure
                                }
                                crate::intrinsic::PendingGoal::Contract { contract }
                                    if contract.revision == 2 && obligation.revision == 2 =>
                                {
                                    crate::intrinsic::ProofKind::NamedContract
                                }
                                crate::intrinsic::PendingGoal::Contract { .. } => {
                                    crate::intrinsic::ProofKind::Contract
                                }
                            }
                })
        {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "host proof results do not match every staged declaration",
                )),
            );
        }
        // The namespace stores complete, immutable transitive evidence rather
        // than fetching a dependency by its current spelling on every use.
        // A fan-out of local uses can otherwise duplicate nested proofs
        // exponentially. Charge the exact source bytes and nodes that would
        // be copied before each copy is made.
        let (Ok(mut remaining_bytes), Ok(mut remaining_nodes)) = (
            usize::try_from(prepared.limits.bytes),
            usize::try_from(prepared.limits.nodes),
        ) else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "proof publication dependency budget exceeds address space",
                )),
            );
        };
        for obligation in &batch.obligations {
            if remaining_bytes.checked_sub(batch.source.len()).is_none() || remaining_nodes == 0 {
                return (
                    self,
                    Err(crate::source::declared::error(
                        crate::source::Stage::Acceptance,
                        "proof publication dependency budget exhausted",
                    )),
                );
            }
            remaining_bytes -= batch.source.len();
            remaining_nodes -= 1;
            for imported in &batch.dependencies {
                if uses(&obligation.term, &imported.name) {
                    let Some((bytes, nodes)) =
                        charge_dependency(imported, remaining_bytes, remaining_nodes)
                    else {
                        return (
                            self,
                            Err(crate::source::declared::error(
                                crate::source::Stage::Acceptance,
                                "proof publication dependency budget exhausted",
                            )),
                        );
                    };
                    remaining_bytes = bytes;
                    remaining_nodes = nodes;
                }
            }
            // Earlier obligations are budgeted when materialized below;
            // their recursive closure does not exist yet at this point.
        }
        let Some(module) = prepared.staged.modules.last_mut() else {
            return (
                self,
                Err(crate::source::declared::error(
                    crate::source::Stage::Acceptance,
                    "missing staged proof module",
                )),
            );
        };
        // Each published proof owns exactly the dependencies its term uses.
        // Select and charge all of them before copying any, then copy each
        // once into a list allocated at its exact length. A selection indexes
        // the imported proofs followed by the earlier local proofs; the first
        // `INLINE` stay on the stack and one spill buffer serves the batch.
        const INLINE: usize = 32;
        let mut inline = [0usize; INLINE];
        let mut spill = alloc::vec::Vec::new();
        #[expect(
            tigerstyle::numeric_units,
            reason = "Owner: noble-contracts; the constant is a count of source elements, and the name states its unit at every use site through the surrounding type and call"
        )]
        let imported_count = batch.dependencies.len();
        for (obligation, result) in batch.obligations.iter().zip(checked) {
            // At most one selection per imported or earlier local proof.
            let dependency_bound = imported_count.saturating_add(module.proofs.len());
            let candidates = batch
                .dependencies
                .iter()
                .chain(module.proofs.iter().map(|local| &local.reference));
            let mut selected = 0usize;
            spill.clear();
            for (index, candidate) in candidates.enumerate() {
                if !uses(&obligation.term, &candidate.name) {
                    continue;
                }
                // Imported closures were charged above; each local closure is
                // charged here, in candidate order, before anything is copied.
                if index >= imported_count {
                    let Some((bytes, nodes)) =
                        charge_dependency(candidate, remaining_bytes, remaining_nodes)
                    else {
                        return (
                            self,
                            Err(crate::source::declared::error(
                                crate::source::Stage::Acceptance,
                                "proof publication dependency budget exhausted",
                            )),
                        );
                    };
                    remaining_bytes = bytes;
                    remaining_nodes = nodes;
                }
                if selected >= dependency_bound {
                    return (
                        self,
                        Err(crate::source::declared::error(
                            crate::source::Stage::Acceptance,
                            "proof publication dependency budget exhausted",
                        )),
                    );
                }
                if selected < INLINE {
                    inline[selected] = index;
                } else {
                    // One entry per selection, so this bound is never reached.
                    if spill.len() >= dependency_bound {
                        return (
                            self,
                            Err(crate::source::declared::error(
                                crate::source::Stage::Acceptance,
                                "proof publication dependency budget exhausted",
                            )),
                        );
                    }
                    spill.push(index);
                }
                selected = selected.saturating_add(1);
            }
            let mut dependencies = alloc::vec::Vec::with_capacity(selected);
            for &index in inline[..selected.min(INLINE)].iter().chain(&spill) {
                let dependency = match index.checked_sub(imported_count) {
                    None => &batch.dependencies[index],
                    Some(local) => &module.proofs[local].reference,
                };
                dependencies.push(dependency.clone());
            }
            module.proofs.push(crate::source::declared::PublishedProof {
                reference: crate::intrinsic::ProofDependency {
                    name: obligation.name.clone(),
                    module: batch.module.clone(),
                    version: batch.version,
                    source: batch.source.clone(),
                    declaration: obligation.clone(),
                    exported: module.proof_exports.contains(&obligation.name),
                    dependencies,
                    model_revision: result.model_revision.clone(),
                    checker_revision: result.checker_revision.clone(),
                },
                checked: result,
            });
        }
        prepared.pending = None;
        self.commit(prepared)
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn revision(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
/// Charges a retained dependency and its transitive closure in the same
/// pre-order as the published copy, returning the remaining byte and node
/// budgets, or `None` once either is exhausted.
fn charge_dependency(
    dependency: &crate::intrinsic::ProofDependency,
    remaining_bytes: usize,
    remaining_nodes: usize,
) -> Option<(usize, usize)> {
    const INLINE: usize = 32;
    let mut inline: [Option<&crate::intrinsic::ProofDependency>; INLINE] = [None; INLINE];
    let mut inline_len = 0usize;
    let mut spill = alloc::vec::Vec::new();
    let (mut bytes, mut nodes) = (remaining_bytes, remaining_nodes);
    let mut current = Some(dependency);
    while let Some(next) = current {
        bytes = bytes.checked_sub(next.source.len())?;
        nodes = nodes.checked_sub(1)?;
        // Every queued proof costs a node. Refuse excessive fan-out before
        // putting any of its children into scratch storage.
        let waiting = inline_len.checked_add(spill.len())?;
        if waiting.checked_add(next.dependencies.len())? > nodes {
            return None;
        }
        // The first child runs next; the rest wait in reverse order. A leaf or
        // unary chain has no heap allocation, unlike a one-element Vec seed.
        for child in next.dependencies.iter().skip(1).rev() {
            if inline_len < INLINE && spill.is_empty() {
                inline[inline_len] = Some(child);
                inline_len = inline_len.saturating_add(1);
            } else {
                if spill.len() >= nodes {
                    return None;
                }
                spill.push(child);
            }
        }
        current = next
            .dependencies
            .first()
            .or_else(|| spill.pop())
            .or_else(|| {
                let top = inline_len.checked_sub(1)?;
                inline_len = top;
                inline[top].take()
            });
    }
    Some((bytes, nodes))
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
)]
fn uses(form: &crate::intrinsic::Form, name: &str) -> bool {
    use crate::intrinsic::FormKind;
    let FormKind::List(items) = &form.kind else {
        return false;
    };
    if items.len() == 2
        && matches!(&items[0].kind, FormKind::Atom(operation) if operation == "use")
        && matches!(&items[1].kind, FormKind::Atom(reference) if reference == name)
    {
        return true;
    }
    items.iter().any(|item| uses(item, name))
}

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
#[cfg(test)]
mod tests {
    use super::super::ModuleSession;
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use noble_kernel::contracts::Behavior;
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use noble_kernel::{shapes::Pattern, words::Scheme};

    #[expect(
        tigerstyle::path_segment_repetition,
        reason = "Owner: noble-contracts; the item name reads as the domain term at its use sites; renaming would churn the extracted subject"
    )]
    #[test]
    fn calculator_source_refuses_897_lightweight_rows_before_namespace_snapshot_clone() {
        let mut session = ModuleSession::new_calculator()
            .unwrap_or_else(|_| panic!("calculator environment unavailable"));
        let environment = &mut session
            .source
            .declared
            .as_mut()
            .unwrap_or_else(|| panic!("calculator declaration context unavailable"))
            .environment;
        while environment.defs.len() < 897 {
            environment.defs.push(Scheme {
                var_kinds: alloc::vec::Vec::new(),
                stack_in: alloc::vec::Vec::new(),
                stack_out: alloc::vec![Pattern::Unit],
                effects: alloc::vec::Vec::new(),
            });
            environment.kinds.push(Behavior::Named);
            environment.deps.push(alloc::vec::Vec::new());
            environment.definition_owners.push(None);
        }
        assert!(
            environment.calculator_clone_budget().is_some(),
            "lightweight row 897 must not be confused with clone-byte exhaustion"
        );
        let limits = crate::Limits {
            bytes: 262_144,
            nodes: 524_288,
            depth: 64,
            work: 512_000_000,
        };
        let refused = session
            .prepare(b"1", &[], limits)
            .err()
            .unwrap_or_else(|| panic!("oversized environment reached snapshot cloning"));
        assert_eq!(refused.stage(), crate::source::Stage::Check);
        assert_eq!(refused.diagnostic().kind, crate::DiagnosticKind::Exhausted);
    }

    #[expect(
        tigerstyle::path_segment_repetition,
        reason = "Owner: noble-contracts; the item name reads as the domain term at its use sites; renaming would churn the extracted subject"
    )]
    #[test]
    fn calculator_source_refuses_deep_environment_before_namespace_snapshot_clone() {
        let mut session = ModuleSession::new_calculator()
            .unwrap_or_else(|_| panic!("calculator environment unavailable"));
        let environment = &mut session
            .source
            .declared
            .as_mut()
            .unwrap_or_else(|| panic!("calculator declaration context unavailable"))
            .environment;
        let program = Pattern::program(
            alloc::vec![Pattern::Unit; 128],
            alloc::vec![Pattern::Unit; 128],
            alloc::vec::Vec::new(),
        );
        for _ in 0..8 {
            environment.defs.push(Scheme {
                var_kinds: alloc::vec::Vec::new(),
                stack_in: alloc::vec::Vec::new(),
                stack_out: alloc::vec![program.clone(); 128],
                effects: alloc::vec::Vec::new(),
            });
        }
        assert!(
            environment.calculator_clone_budget().is_none(),
            "nested program stacks must contribute every retained node to the clone bound"
        );
        let limits = crate::Limits {
            bytes: 262_144,
            nodes: 524_288,
            depth: 64,
            work: 512_000_000,
        };
        let refused = session
            .prepare(b"1", &[], limits)
            .err()
            .unwrap_or_else(|| panic!("oversized environment reached snapshot cloning"));
        assert_eq!(refused.stage(), crate::source::Stage::Check);
        assert_eq!(refused.diagnostic().kind, crate::DiagnosticKind::Exhausted);
    }
}
