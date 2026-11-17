#[expect(
    tigerstyle::renamed_imports,
    reason = "Owner: noble-contracts; the local alias matches the domain vocabulary used throughout this module"
)]
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
use crate::source::proof::{
    CheckedSelectedTarget, ProofContext, SourceCorrespondenceCandidate,
    SourceProofRefusal as Refusal,
};
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
use noble_kernel::{
    contracts::Definition,
    types::{EffSet, Ty},
    untrusted::{Node, NodeId},
};

impl super::ModuleSession {
    /// Compare the *full* checked source target to one host-retained verified
    /// source proof in this immutable namespace. This does not authenticate
    /// the host's Lean callback or attest an installed Wasm artifact; those
    /// independent host gates are prerequisites for any publication.
    #[expect(
        clippy::too_many_arguments,
        reason = "Owner: noble-maintainers; module, version, contract and proof names are independent namespace selectors that refuse separately; bundling them would change the public noble-contracts API and its reviewed source inventory."
    )]
    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    #[expect(
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
    )]
    pub fn correspond_checked_target<'a>(
        &'a self,
        module_name: &str,
        version: u32,
        contract_name: &str,
        proof_name: &str,
        target: &'a CheckedSelectedTarget<'_>,
        context: ProofContext<'_>,
        limits: crate::Limits,
    ) -> Result<SourceCorrespondenceCandidate<'a>, Refusal> {
        // MC1 proves only a capture-free, pure I64-to-I64 builtin program.
        // Resource/Account interfaces and effectful live targets have no
        // matching theorem; interface fit alone must never imply proof.
        let body = target.body();
        if body.interface.stack_in != [Ty::I64]
            || body.interface.stack_out != [Ty::I64]
            || body.interface.effects != EffSet::empty()
            || !body.live_sites.is_empty()
            || !target.root().live_sites.is_empty()
        {
            return Err(Refusal::UnsupportedProfile);
        }
        if context.module_generation != self.generation {
            return Err(Refusal::StaleContext);
        }
        let mut modules = self
            .modules
            .iter()
            .filter(|entry| entry.name == module_name && entry.version == version);
        let module = modules.next().ok_or(Refusal::MissingEvidence)?;
        if modules.next().is_some() {
            return Err(Refusal::MismatchedSource);
        }
        let mut contracts = module
            .contracts
            .iter()
            .filter(|entry| entry.name == contract_name);
        let contract = contracts.next().ok_or(Refusal::MissingEvidence)?;
        if contracts.next().is_some() {
            return Err(Refusal::MismatchedClaim);
        }
        let mut proofs = module
            .proofs
            .iter()
            .filter(|entry| entry.reference.name == proof_name);
        let proof = proofs.next().ok_or(Refusal::MissingEvidence)?;
        if proofs.next().is_some() {
            return Err(Refusal::MismatchedClaim);
        }
        let goal = &contract.goal;
        let crate::intrinsic::PendingGoal::Contract { contract: bound } =
            &proof.reference.declaration.goal
        else {
            return Err(Refusal::MismatchedClaim);
        };
        if goal.revision != 1
            || proof.reference.declaration.revision != 1
            || proof.checked.kind != crate::intrinsic::ProofKind::Contract
        {
            return Err(Refusal::UnsupportedProfile);
        }
        if goal.contract_name != bound.contract_name
            || goal.contract_span != bound.contract_span
            || goal.inputs != bound.inputs
            || goal.outputs != bound.outputs
            || goal.requires != bound.requires
            || goal.ensures != bound.ensures
            || goal.subject.module_source != bound.subject.module_source
            || goal.subject.definition_source != bound.subject.definition_source
            || goal.subject.definition_identity != bound.subject.definition_identity
            || goal.subject.definition_owner != bound.subject.definition_owner
            || goal.subject.source_span != bound.subject.source_span
            || goal.subject.definition_ordinal != bound.subject.definition_ordinal
            || goal.subject.input_types != bound.subject.input_types
            || goal.subject.output_types != bound.subject.output_types
            || goal.subject.effects != bound.subject.effects
            || proof.reference.module != module.name
            || proof.reference.version != module.version
            || proof.reference.source != module.source
            || goal.subject.module != module.name
            || goal.subject.version != module.version
            || goal.subject.module_source != module.source
        {
            return Err(Refusal::MismatchedClaim);
        }
        if !proof.reference.dependencies.is_empty() {
            return Err(Refusal::Assumptions);
        }
        let checked = &proof.checked;
        if checked.name != proof_name
            || checked.claim != "MC1Obligation.claim"
            || checked.model_revision != context.model_revision
            || checked.checker_revision != context.checker_revision
            || proof.reference.model_revision != context.model_revision
            || proof.reference.checker_revision != context.checker_revision
        {
            return Err(Refusal::MismatchedClaim);
        }
        // The contract was prepared and source-checked on staging and again
        // before commit_verified. Recheck it from its retained, not public,
        // goal to reject a changed kernel/MC1 interpretation at this boundary.
        crate::intrinsic::prepare_contract(goal, limits)
            .map_err(|_| Refusal::IndependentCheckFailed)?;
        if goal.subject.definition_source != target.source() {
            return Err(Refusal::MismatchedSource);
        }
        let accepted = &goal.subject.accepted_submission;
        let [defined] = accepted.definitions.as_slice() else {
            return Err(Refusal::MismatchedDefinition);
        };
        if defined.identity != goal.subject.definition_identity
            || goal.subject.input_types != body.interface.stack_in
            || goal.subject.output_types != body.interface.stack_out
            || goal.subject.effects != body.interface.effects
            || defined.expected.stack_in != target.definition().expected.stack_in
            || defined.expected.stack_out != target.definition().expected.stack_out
            || defined.expected.allowed_effects != target.definition().expected.allowed_effects
            || !crate::intrinsic::canonical_wrapping_add(&target.submission().environment)
            || !same_candidate(&defined.body.candidate, &target.definition().body.candidate)
            || !target.definition().body.texts.is_empty()
        {
            return Err(Refusal::MismatchedDefinition);
        }
        Ok(SourceCorrespondenceCandidate {
            target_id: target.definition_id(),
            target_generation: target.source_generation(),
            module_generation: self.generation,
            goal,
            retained: checked,
            interface: &body.interface,
        })
    }
}

#[expect(
    tigerstyle::compound_condition,
    reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn same_candidate(
    left: &noble_kernel::untrusted::Candidate,
    right: &noble_kernel::untrusted::Candidate,
) -> bool {
    if left.format != right.format
        || left.revision != right.revision
        || left.body.len() != 2
        || left.body != [NodeId(0), NodeId(1)]
        || right.body != left.body
        || left.nodes.len() != 2
        || right.nodes.len() != 2
    {
        return false;
    }
    let (
        Node::Literal {
            lit: noble_kernel::untrusted::Lit::I64(a),
            inst: ai,
        },
        Node::Literal {
            lit: noble_kernel::untrusted::Lit::I64(b),
            inst: bi,
        },
    ) = (&left.nodes[0], &right.nodes[0])
    else {
        return false;
    };
    if a != b || !crate::intrinsic::same_inst(ai, bi) {
        return false;
    }
    let (
        Node::Invocation {
            def: Definition(4),
            inst: ai,
        },
        Node::Invocation {
            def: Definition(4),
            inst: bi,
        },
    ) = (&left.nodes[1], &right.nodes[1])
    else {
        return false;
    };
    crate::intrinsic::same_inst(ai, bi)
}
