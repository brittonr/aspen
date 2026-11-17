//! Checked Program residualization over an untouched, opaque stack prefix.
//!
//! The original submission is checked in full before graph extraction. An
//! extractor marker refuses every attempt to consume or move a prefix value;
//! the graph sees only the supported active suffix. The emitted source is
//! independently prepared and admitted at the *full* dynamic interface,
//! leaving the prefix below its `let` bindings. The returned ModulePrepared is
//! the actual ordinary Declared-Text-v1 Program, not a graph proxy.
//!
//! This implementation and its finite differential tests do not prove
//! Rust/model, extraction/Program or backend/runtime correspondence.

use crate::identity::IdentityScheme;

/// Exact checked subject, binding map and bounded re-admission inputs.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    /// Untrusted producer bytes: the operation independently accepts the
    /// root and every reached definition before it uses them.
    pub original: &'a noble_kernel::execution::Submission,
    /// Independently checked exact, sorted definition closure.
    pub claimed_closure: &'a [u64],
    /// Optional externally retained original subject identity. A stale pin
    /// refuses; a caller must not treat a self-computed pin as authority.
    pub expected_original: Option<crate::ir::Digest>,
    /// Count of untouched stack values below the active suffix.
    pub preserved_tail: usize,
    /// One optional I64 value per scalar input of the active suffix, in
    /// bottom-first order; the preserved tail has no binding positions.
    pub static_inputs: &'a [Option<i64>],
    /// Extraction, specialization and checker caps.
    pub budgets: &'a crate::check::Budgets,
    /// Ordinary declared-text preparation limits.
    pub source_limits: noble_contracts::Limits,
}

/// A source-prepared residual Program and its checked derivation.
#[derive(Debug)]
pub struct Residual {
    /// Emitted source for the ordinary Declared-Text-v1 path.
    pub source: alloc::string::String,
    /// The source-prepared ordinary Program, checked on the full interface.
    pub prepared: noble_contracts::source::ModulePrepared,
    /// Original full-stack subject and its exact reached closure.
    pub original_subject: crate::ir::Subject,
    /// Fresh Program derivation identity; its closure is the independently
    /// recomputed residual closure (empty for the let-only emitter).
    pub residual_subject: crate::ir::Subject,
    /// Specialized and independently checked graph before source emission.
    pub specialized: super::Residual,
    /// Independently re-extracted graph from the full-interface Program.
    pub readmitted: crate::check::CheckedGraph,
    /// Number of unchanged bottom-most stack values on both interfaces.
    pub preserved_tail: usize,
}

fn refuse(
    code: crate::diag::Code,
    detail: impl Into<alloc::string::String>,
) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(code, crate::diag::Site::Graph, detail)
}

fn graph_budget(budgets: &crate::check::Budgets) -> crate::extract::Budget {
    crate::extract::Budget {
        work_units: budgets.work_units,
        expanded_nodes: budgets.expanded_nodes,
        inline_depth: budgets.inline_depth,
    }
}

fn output_type(ty: crate::ir::ScalarType) -> Option<noble_kernel::types::Ty> {
    match ty {
        crate::ir::ScalarType::I64 => Some(noble_kernel::types::Ty::I64),
        crate::ir::ScalarType::Bool => Some(noble_kernel::types::Ty::Bool),
        crate::ir::ScalarType::F32 => None,
    }
}

fn static_encoding(static_inputs: &[Option<i64>], preserved_tail: usize) -> alloc::vec::Vec<u8> {
    let mut encoded = alloc::vec::Vec::with_capacity(8 + static_inputs.len().saturating_mul(9));
    encoded.extend_from_slice(&(preserved_tail as u64).to_le_bytes());
    for entry in static_inputs {
        match entry {
            Some(value) => {
                encoded.push(1);
                encoded.extend_from_slice(&value.to_le_bytes());
            }
            None => encoded.push(0),
        }
    }
    encoded
}

/// Specialize an exact checked Program into a newly source-prepared,
/// independently checked residual Program. The observation domain is the
/// resource-free stack: `tail ++ merge(static_inputs, dynamic)` on the
/// original side and `tail ++ dynamic` on the residual side. No operation of
/// the original may inspect or move `tail`; unsupported subjects refuse.
/// Neither side is executed here, and the ordinary unspecialized path is
/// unchanged.
///
/// # Errors
/// Returns the original admission, extraction or graph-specialization
/// diagnostic, `E-SUBJECT` for a stale identity or refused residual source,
/// `E-CLOSURE` for a nonempty residual dependency closure, and `E-INTERFACE`
/// for a changed full stack interface. No partial Program is returned.
pub fn residualize(
    ids: &dyn IdentityScheme,
    request: &Request<'_>,
) -> Result<Residual, crate::diag::Diagnostic> {
    let original = crate::extract::admit_with_tail(
        ids,
        request.original,
        &crate::extract::Request {
            profile: "ngraph-i64/0",
            claimed_closure: request.claimed_closure,
            budget: graph_budget(request.budgets),
        },
        request.preserved_tail,
    )?;
    if request
        .expected_original
        .is_some_and(|expected| expected != original.subject().identity)
    {
        return Err(refuse(
            crate::diag::Code::Subject,
            "the checked original Program identity differs from the retained identity",
        ));
    }
    let extracted = crate::extract::extract(ids, &original, request.budgets)?;
    let specialized = super::specialize(
        ids,
        &extracted.checked,
        &super::Request {
            static_inputs: request.static_inputs,
            budgets: request.budgets,
        },
    )?;
    let source = super::emit_source(&specialized.checked)?;
    let mut dynamic_inputs =
        request.original.request.expected.stack_in[..request.preserved_tail].to_vec();
    dynamic_inputs.extend(super::interface_types(&specialized.checked)?);
    let retained = noble_contracts::source::RetainedSourceLimit::new(request.source_limits.bytes)
        .ok_or_else(|| {
        refuse(
            crate::diag::Code::Profile,
            "the source byte limit exceeds the retained-source cap",
        )
    })?;
    let session = noble_contracts::source::ModuleSession::new_declared_text(&[], false, retained)
        .map_err(super::source_refusal)?;
    let prepared = session
        .prepare(source.as_bytes(), &dynamic_inputs, request.source_limits)
        .map_err(super::source_refusal)?;
    if prepared.kind() != noble_contracts::source::ModuleKind::Expression {
        return Err(refuse(
            crate::diag::Code::Subject,
            "residual source is not an ordinary expression Program",
        ));
    }
    let submission = prepared.submission().ok_or_else(|| {
        refuse(
            crate::diag::Code::Subject,
            "residual source produced no checked Program submission",
        )
    })?;
    let expected_output = &request.original.request.expected.stack_out;
    if prepared.output() != expected_output {
        return Err(refuse(
            crate::diag::Code::Interface,
            "the full residual Program output differs from the original stack interface",
        ));
    }
    if prepared.output()[request.preserved_tail..]
        .iter()
        .zip(specialized.checked.graph().outputs.iter())
        .any(|(ty, node)| specialized.checked.ty(*node).and_then(output_type).as_ref() != Some(ty))
        || prepared.output().len() - request.preserved_tail
            != specialized.checked.graph().outputs.len()
    {
        return Err(refuse(
            crate::diag::Code::Interface,
            "residual Program outputs differ from the checked residual graph",
        ));
    }
    let readmitted = crate::extract::admit_with_tail(
        ids,
        submission,
        &crate::extract::Request {
            profile: "ngraph-i64/0",
            claimed_closure: &[],
            budget: graph_budget(request.budgets),
        },
        request.preserved_tail,
    )?;
    let residual_subject = crate::ir::Subject::derive(
        ids,
        crate::ir::SubjectKind::Extracted,
        &[
            b"noble.ngraph.residual-program.v0",
            super::ACCOUNTING_REVISION.as_bytes(),
            &original.subject().identity,
            &readmitted.subject().identity,
            &static_encoding(request.static_inputs, request.preserved_tail),
        ],
        readmitted.closure().to_vec(),
    );
    let readmitted_graph = crate::extract::extract(ids, &readmitted, request.budgets)?.checked;
    Ok(Residual {
        source,
        prepared,
        original_subject: original.subject().clone(),
        residual_subject,
        specialized,
        readmitted: readmitted_graph,
        preserved_tail: request.preserved_tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Hasher(u64);

    impl crate::identity::IdentityHasher for Hasher {
        fn update(&mut self, bytes: &[u8]) {
            for &byte in bytes {
                self.0 = self.0.wrapping_mul(31).wrapping_add(u64::from(byte));
            }
        }

        fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
            let mut out = [0; 32];
            out[..8].copy_from_slice(&self.0.to_le_bytes());
            out
        }
    }

    struct Ids;

    impl IdentityScheme for Ids {
        fn name(&self) -> &'static str {
            "test"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(Hasher(0))
        }
    }

    fn checked(
        definitions: &[&str],
        root: &str,
        input: &[noble_kernel::types::Ty],
    ) -> crate::extract::source::Prepared {
        crate::extract::source::prepare(
            noble_contracts::source::Session::new(),
            noble_contracts::Limits::default(),
            definitions,
            root,
            input,
        )
        .expect("original source prepares through the ordinary frontend")
    }

    fn prepare(
        original: &crate::extract::source::Prepared,
        tail: usize,
        static_inputs: &[Option<i64>],
        expected_original: Option<crate::ir::Digest>,
        claimed_closure: &[u64],
    ) -> Result<Residual, crate::diag::Diagnostic> {
        residualize(
            &Ids,
            &Request {
                original: &original.submission,
                claimed_closure,
                expected_original,
                preserved_tail: tail,
                static_inputs,
                budgets: &crate::check::Budgets::default(),
                source_limits: noble_contracts::Limits::default(),
            },
        )
    }

    #[test]
    fn checked_root_keeps_opaque_tail_and_exact_closures() {
        let pair = noble_kernel::types::Ty::Pair(
            alloc::boxed::Box::new(noble_kernel::types::Ty::I64),
            alloc::boxed::Box::new(noble_kernel::types::Ty::Bool),
        );
        let inputs = [
            noble_kernel::types::Ty::Text,
            noble_kernel::types::Ty::Unit,
            pair,
            noble_kernel::types::Ty::I64,
            noble_kernel::types::Ty::I64,
        ];
        let original = checked(&["def twice [ dup + ]"], "twice swap 3 +", &inputs);
        assert_eq!(&original.submission.request.expected.stack_in, &inputs);
        let residual = prepare(
            &original,
            3,
            &[Some(5), None],
            None,
            &original.claimed_closure,
        )
        .expect("checked Program specializes while preserving an opaque tail");
        assert_eq!(residual.preserved_tail, 3);
        assert_eq!(residual.original_subject.closure, original.claimed_closure);
        assert!(residual.residual_subject.closure.is_empty());
        assert_ne!(
            residual.original_subject.identity,
            residual.residual_subject.identity
        );
        assert_eq!(
            residual
                .prepared
                .submission()
                .expect("ordinary Program")
                .request
                .expected
                .stack_in,
            [
                inputs[..3].to_vec(),
                alloc::vec![noble_kernel::types::Ty::I64],
            ]
            .concat()
        );
        assert_eq!(
            residual.prepared.output(),
            original.submission.request.expected.stack_out
        );
        assert_eq!(
            residual.readmitted.graph().slots.len(),
            1,
            "the residual graph contains only the dynamic active suffix"
        );
    }

    #[test]
    fn stale_subject_and_missing_dependency_refuse_before_publication() {
        let original = checked(
            &["def twice [ dup + ]"],
            "twice",
            &[noble_kernel::types::Ty::Text, noble_kernel::types::Ty::I64],
        );
        assert!(!original.claimed_closure.is_empty());
        assert_eq!(
            prepare(&original, 1, &[None], None, &[])
                .expect_err("a missing named definition must refuse")
                .code,
            crate::diag::Code::Closure
        );
        let retained = prepare(&original, 1, &[None], None, &original.claimed_closure)
            .expect("the source-bound original identity")
            .original_subject
            .identity;
        let mut stale = original.clone();
        stale.submission.definitions[0].identity += 1;
        stale.claimed_closure[0] += 1;
        assert_eq!(
            prepare(&stale, 1, &[None], Some(retained), &stale.claimed_closure)
                .expect_err("a changed definition identity must not match the retained subject")
                .code,
            crate::diag::Code::Subject
        );
    }

    #[test]
    fn a_checked_root_may_not_inspect_or_move_its_tail() {
        let original = checked(
            &[],
            "swap",
            &[noble_kernel::types::Ty::I64, noble_kernel::types::Ty::I64],
        );
        assert_eq!(
            prepare(&original, 1, &[None], None, &[])
                .expect_err("the opaque tail is consumed")
                .code,
            crate::diag::Code::Interface
        );
        let effectful = checked(
            &[],
            "\"audit\" test.emit 1 +",
            &[noble_kernel::types::Ty::Text, noble_kernel::types::Ty::I64],
        );
        assert_eq!(
            prepare(&effectful, 1, &[None], None, &[])
                .expect_err("host effects are outside the pure Program slice")
                .code,
            crate::diag::Code::Effect
        );
    }
}
