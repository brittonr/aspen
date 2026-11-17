//! Source-bound, kernel-rechecked selection. A source target is not a proof of
//! its behavior or of any compiled artifact.

/// A selected definition's identity within one retained source session. Neither
/// field is portable across sessions; the borrowed target also carries its full
/// source and accepted recipe, which must be compared before publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedDefinitionId {
    index: u32,
    identity: u64,
}

impl CheckedDefinitionId {
    pub const fn index(self) -> u32 {
        self.index
    }
    pub const fn identity(self) -> u64 {
        self.identity
    }
}

/// Refusal of source selection or correspondence. `UnsupportedProfile` means
/// that the checked target lies outside the existing pure unary I64 theorem;
/// in particular this is not a generic Account/effect/capture proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the type name is the domain term at its use sites and renaming would churn the extracted subject"
)]
pub enum SourceProofRefusal {
    UnsupportedProfile,
    StaleContext,
    MissingTarget,
    MissingEvidence,
    MismatchedSource,
    MismatchedDefinition,
    MismatchedInterface,
    MismatchedClaim,
    Assumptions,
    IndependentCheckFailed,
}

/// One source-owned named definition selected by a complete, independently
/// kernel-accepted root invocation. `submission` and `definition` retain the
/// actual checked body, instantiations, environment and source session identity;
/// no producer name or digest alone is an executable target.
pub struct CheckedSelectedTarget<'a> {
    id: CheckedDefinitionId,
    generation: u64,
    source: &'a [u8],
    submission: &'a noble_kernel::execution::Submission,
    definition: &'a noble_kernel::execution::Definition,
    root: noble_kernel::untrusted::Checked,
    body: noble_kernel::untrusted::Checked,
    tree: &'a super::Tree,
    name: &'a str,
    /// Candidate NodeIds are compacted during inference; the source replay
    /// retains authored and generated lexical NodeIds in a separate arena.
    candidate_to_lexical: alloc::vec::Vec<u32>,
    generated: core::ops::Range<u32>,
    witnesses: alloc::vec::Vec<super::locals::GeneratedWitness>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedGeneratedWitness {
    generation: &'static str,
    occurrence_node: Option<u32>,
    binder_node: Option<u32>,
}

impl CheckedGeneratedWitness {
    pub const fn generation(&self) -> &'static str {
        self.generation
    }
    pub const fn occurrence_node(&self) -> Option<u32> {
        self.occurrence_node
    }
    pub const fn binder_node(&self) -> Option<u32> {
        self.binder_node
    }
}

pub enum CheckedLexicalKind<'a> {
    I64(i64),
    Builtin(u32),
    Quotation(&'a [u32]),
}

pub struct CheckedLexicalNode<'a> {
    pub kind: CheckedLexicalKind<'a>,
    pub authored_span: Option<crate::Span>,
    pub generated_witness: Option<CheckedGeneratedWitness>,
}

pub struct CheckedOperationSite {
    pub lexical_node: u32,
    pub authored_span: Option<crate::Span>,
    pub generated_witness: Option<CheckedGeneratedWitness>,
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::mutating_input_in_pure,
    tigerstyle::platform_dependent_cast,
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; it mutates only exclusively borrowed, caller-owned work state for this bounded pass; checked inputs stay immutable; usize is at least 32 bits on every supported target, so the u32-to-usize widening is lossless; growth is bounded by the checked input or admission limits that drive the loop."
)]
fn candidate_to_lexical(
    candidate: &noble_kernel::untrusted::Candidate,
    tree: &super::Tree,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<u32>, SourceProofRefusal> {
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain items directly, and the qualified form at every use site would add noise without changing the dependency."
    )]
    use noble_kernel::untrusted::Node;
    #[expect(
        tigerstyle::non_trait_imports,
        tigerstyle::renamed_imports,
        reason = "Owner: noble-contracts; the module imports these domain items directly, and the qualified form at every use site would add noise without changing the dependency; the short local alias keeps the refusal constructors readable within this function."
    )]
    use SourceProofRefusal as Refusal;
    let mut mapping = alloc::vec![u32::MAX; candidate.nodes.len()];
    let mut pairs = alloc::vec::Vec::new();
    if candidate.body.len() != tree.body.len() {
        return Err(Refusal::MismatchedSource);
    }
    pairs.extend(
        candidate
            .body
            .iter()
            .zip(&tree.body)
            .map(|(checked, source)| (checked.0, *source)),
    );
    while let Some((checked, lexical)) = pairs.pop() {
        let slot = mapping
            .get_mut(usize::try_from(checked).map_err(|_| Refusal::MismatchedSource)?)
            .ok_or(Refusal::MismatchedSource)?;
        if *slot != u32::MAX {
            return Err(Refusal::MismatchedSource);
        }
        *slot = lexical;
        let source = tree.node(lexical).map_err(|_| Refusal::MismatchedSource)?;
        meter
            .charge(1, source.span)
            .map_err(|_| Refusal::MismatchedSource)?;
        let checked = candidate
            .nodes
            .get(checked as usize)
            .ok_or(Refusal::MismatchedSource)?;
        match (&source.kind, checked) {
            (super::Kind::Literal(a), Node::Literal { lit: b, .. }) if a == b => {}
            (
                super::Kind::Text(_),
                Node::Literal {
                    lit: noble_kernel::untrusted::Lit::Text,
                    ..
                },
            ) => {}
            (super::Kind::Call(super::Target::Builtin(a)), Node::Invocation { def, .. })
                if *a == def.0 => {}
            (super::Kind::Call(super::Target::Named(_)), Node::Invocation { .. }) => {}
            (super::Kind::Call(super::Target::SelfOwner), Node::SignedSelf { .. }) => {}
            (super::Kind::Quotation(source_body), Node::Quotation { body, .. })
                if source_body.len() == body.len() =>
            {
                pairs.extend(
                    body.iter()
                        .zip(source_body)
                        .map(|(checked, source)| (checked.0, *source)),
                );
            }
            _ => return Err(Refusal::MismatchedSource),
        }
    }
    if mapping.contains(&u32::MAX) {
        return Err(Refusal::MismatchedSource);
    }
    Ok(mapping)
}

/// The replayed lexical span, its generated witnesses and their node map.
type LexicalReplay = (
    core::ops::Range<u32>,
    alloc::vec::Vec<super::locals::GeneratedWitness>,
    alloc::vec::Vec<u32>,
);

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::path_segment_repetition,
    tigerstyle::platform_dependent_cast,
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; the name matches the established vocabulary used by callers; renaming it would churn call sites without clarifying them; usize is at least 32 bits on every supported target, so the u32-to-usize widening is lossless; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
)]
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-contracts; the is_empty refusal precedes nodes.len() - 1, and witness ordinals are compared only after witnesses.len() equals end - original, so original + ordinal stays below end."
)]
fn replay_lexical_source(
    bytes: &[u8],
    retained: &super::Named,
    session: &super::Session,
    definition_prefix: usize,
    limits: crate::Limits,
    candidate: &noble_kernel::untrusted::Candidate,
) -> Result<LexicalReplay, SourceProofRefusal> {
    #[expect(
        tigerstyle::non_trait_imports,
        tigerstyle::renamed_imports,
        reason = "Owner: noble-contracts; the module imports these domain items directly, and the qualified form at every use site would add noise without changing the dependency; the short local alias keeps the refusal constructors readable within this function."
    )]
    use SourceProofRefusal as Refusal;
    let mut meter = crate::Meter::new(limits);
    let (mut replay, name) =
        super::parsing::parse(bytes, &mut meter).map_err(|_| Refusal::MismatchedSource)?;
    if name.as_deref() != Some(retained.name.as_str())
        || replay.span != retained.tree.span
        || replay.nodes.is_empty()
    {
        return Err(Refusal::MismatchedSource);
    }
    let original = u32::try_from(replay.nodes.len() - 1).map_err(|_| Refusal::MismatchedSource)?;
    let mut witnesses = alloc::vec::Vec::new();
    super::locals::lower_with_witness(&mut replay, &mut meter, &mut witnesses)
        .map_err(|_| Refusal::MismatchedSource)?;
    super::resolution::resolve_prefix(
        &mut replay,
        name.as_deref(),
        session,
        false,
        definition_prefix,
        &mut meter,
    )
    .map_err(|_| Refusal::MismatchedSource)?;
    let last = replay
        .nodes
        .len()
        .checked_sub(1)
        .ok_or(Refusal::MismatchedSource)?;
    let end = u32::try_from(last).map_err(|_| Refusal::MismatchedSource)?;
    if replay != retained.tree {
        return Err(Refusal::MismatchedSource);
    }
    let generated_count = end.checked_sub(original).ok_or(Refusal::MismatchedSource)?;
    if witnesses.len() != generated_count as usize
        || witnesses
            .iter()
            .enumerate()
            .any(|(ordinal, witness)| witness.node != original + ordinal as u32)
    {
        return Err(Refusal::MismatchedSource);
    }
    let mapping = candidate_to_lexical(candidate, &replay, &mut meter)?;
    Ok((original..end, witnesses, mapping))
}

impl CheckedSelectedTarget<'_> {
    pub const fn definition_id(&self) -> CheckedDefinitionId {
        self.id
    }
    pub const fn source_generation(&self) -> u64 {
        self.generation
    }
    pub const fn source(&self) -> &[u8] {
        self.source
    }
    pub const fn submission(&self) -> &noble_kernel::execution::Submission {
        self.submission
    }
    pub const fn definition(&self) -> &noble_kernel::execution::Definition {
        self.definition
    }
    pub const fn root(&self) -> &noble_kernel::untrusted::Checked {
        &self.root
    }
    pub const fn body(&self) -> &noble_kernel::untrusted::Checked {
        &self.body
    }
    pub const fn definition_name(&self) -> &str {
        self.name
    }
    pub fn candidate_to_lexical(&self) -> &[u32] {
        &self.candidate_to_lexical
    }
    pub fn lexical_body(&self) -> &[u32] {
        &self.tree.body
    }
    #[expect(
        tigerstyle::usize_in_public_api,
        reason = "Owner: noble-contracts; the value indexes in-memory tables already typed usize by every caller."
    )]
    pub fn lexical_nodes_len(&self) -> usize {
        self.tree.nodes.len()
    }
    pub const fn original_lexical_nodes(&self) -> u32 {
        self.generated.start
    }
    pub const fn generated_lexical_end(&self) -> u32 {
        self.generated.end
    }
    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; usize is at least 32 bits on every supported target, so the u32-to-usize widening is lossless."
    )]
    pub fn lexical_source_node(&self, candidate: noble_kernel::untrusted::NodeId) -> Option<u32> {
        self.candidate_to_lexical.get(candidate.0 as usize).copied()
    }
    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; usize is at least 32 bits on every supported target, so the u32-to-usize widening is lossless."
    )]
    fn lexical_witness(&self, node: u32) -> Option<CheckedGeneratedWitness> {
        let at = node.checked_sub(self.generated.start)?;
        let witness = self.witnesses.get(at as usize)?;
        (witness.node == node).then_some(CheckedGeneratedWitness {
            generation: witness.generation.as_str(),
            occurrence_node: witness.occurrence_node,
            binder_node: witness.binder_node,
        })
    }
    pub fn checked_lexical_node(
        &self,
        lexical: u32,
    ) -> Result<CheckedLexicalNode<'_>, SourceProofRefusal> {
        let source = self
            .tree
            .node(lexical)
            .map_err(|_| SourceProofRefusal::MismatchedSource)?;
        let kind = match &source.kind {
            super::Kind::Literal(noble_kernel::untrusted::Lit::I64(value)) => {
                CheckedLexicalKind::I64(*value)
            }
            super::Kind::Call(super::Target::Builtin(definition)) => {
                CheckedLexicalKind::Builtin(*definition)
            }
            super::Kind::Quotation(body) => CheckedLexicalKind::Quotation(body),
            _ => return Err(SourceProofRefusal::UnsupportedProfile),
        };
        Ok(CheckedLexicalNode {
            kind,
            authored_span: (!self.generated.contains(&lexical)).then_some(source.span),
            generated_witness: self.lexical_witness(lexical),
        })
    }
    /// Generated nodes have no authored instruction token: their authority is
    /// the complete, independently replayed lowering of retained source.
    pub fn is_generated_operation(&self, node: noble_kernel::untrusted::NodeId) -> bool {
        self.lexical_source_node(node)
            .is_some_and(|lexical| self.generated.contains(&lexical))
    }

    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
    )]
    /// Locate an operation in the selected checked definition, mapping its
    /// compact kernel index back to the complete replayed lexical arena.
    /// Generated instructions have a witness, NEVER an authored byte span.
    pub fn checked_operation_site(
        &self,
        node: noble_kernel::untrusted::NodeId,
    ) -> Result<CheckedOperationSite, SourceProofRefusal> {
        use noble_kernel::untrusted::{Lit, Node};
        let candidate = &self.definition.body.candidate;
        let lexical_node = self
            .lexical_source_node(node)
            .ok_or(SourceProofRefusal::MismatchedSource)?;
        let source = self
            .tree
            .node(lexical_node)
            .map_err(|_| SourceProofRefusal::MismatchedSource)?;
        let checked = usize::try_from(node.0)
            .ok()
            .and_then(|index| candidate.nodes.get(index))
            .ok_or(SourceProofRefusal::MismatchedSource)?;
        let start =
            usize::try_from(source.span.start).map_err(|_| SourceProofRefusal::MismatchedSource)?;
        let end =
            usize::try_from(source.span.end).map_err(|_| SourceProofRefusal::MismatchedSource)?;
        let is_generated = self.generated.contains(&lexical_node);
        let bytes = if is_generated {
            // A replayed projection or cleanup has no corresponding authored
            // token. The entire immutable lowering was compared at selection.
            &[][..]
        } else {
            self.source
                .get(start..end)
                .filter(|bytes| !bytes.is_empty())
                .ok_or(SourceProofRefusal::MismatchedSource)?
        };
        let is_valid = match (&source.kind, checked) {
            (
                super::Kind::Literal(Lit::I64(a)),
                Node::Literal {
                    lit: Lit::I64(b), ..
                },
            ) => !is_generated && a == b,
            (super::Kind::Call(super::Target::Builtin(a)), Node::Invocation { def, .. })
                if *a == def.0 =>
            {
                is_generated || {
                    let expected = match a {
                        0 => b"dup".as_slice(),
                        1 => b"drop".as_slice(),
                        2 => b"swap".as_slice(),
                        3 => b"dip".as_slice(),
                        4 => b"+".as_slice(),
                        5 => b"-".as_slice(),
                        6 => b"*".as_slice(),
                        8 => b"quote".as_slice(),
                        9 => b"compose".as_slice(),
                        10 => b"run".as_slice(),
                        _ => return Err(SourceProofRefusal::UnsupportedProfile),
                    };
                    bytes == expected
                }
            }
            (super::Kind::Quotation(source_body), Node::Quotation { body, .. }) => {
                (is_generated || (bytes.first() == Some(&b'[') && bytes.last() == Some(&b']')))
                    && source_body.len() == body.len()
                    && source_body
                        .iter()
                        .zip(body)
                        .all(|(a, b)| Some(*a) == self.lexical_source_node(*b))
            }
            _ => false,
        };
        if !is_valid {
            return Err(SourceProofRefusal::MismatchedSource);
        }
        if self
            .body
            .derivations
            .iter()
            .filter(|item| item.node == node)
            .count()
            != 1
        {
            return Err(SourceProofRefusal::IndependentCheckFailed);
        }
        Ok(CheckedOperationSite {
            lexical_node,
            authored_span: (!is_generated).then_some(source.span),
            generated_witness: self.lexical_witness(lexical_node),
        })
    }

    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
    )]
    /// Authenticate a quote occurrence in the retained source and the
    /// independently rechecked selected definition. The runtime value that
    /// will be captured is not known until the VM executes this instruction.
    pub fn checked_quote_site(
        &self,
        node: noble_kernel::untrusted::NodeId,
    ) -> Result<CheckedQuoteSite<'_>, SourceProofRefusal> {
        use noble_kernel::{contracts::Definition, types::Ty, untrusted::Node};
        let site = self.checked_operation_site(node)?;
        let candidate = &self.definition.body.candidate;
        let source_node = self
            .tree
            .node(site.lexical_node)
            .map_err(|_| SourceProofRefusal::MismatchedSource)?;
        let (
            super::Kind::Call(super::Target::Builtin(8)),
            Some(Node::Invocation {
                def: Definition(8), ..
            }),
        ) = (
            &source_node.kind,
            usize::try_from(node.0)
                .ok()
                .and_then(|index| candidate.nodes.get(index)),
        )
        else {
            return Err(SourceProofRefusal::UnsupportedProfile);
        };
        let mut derivations = self
            .body
            .derivations
            .iter()
            .filter(|item| item.node == node);
        let derived = derivations
            .next()
            .ok_or(SourceProofRefusal::IndependentCheckFailed)?;
        if derivations.next().is_some()
            || !matches!(
                derived.interface.stack_in.last(),
                Some(Ty::I64 | Ty::Program(..))
            )
            || !matches!(derived.interface.stack_out.last(), Some(Ty::Program(..)))
            || !derived.interface.effects.is_empty()
        {
            return Err(SourceProofRefusal::UnsupportedProfile);
        }
        Ok(CheckedQuoteSite {
            definition_id: self.id,
            source_generation: self.generation,
            node,
            authored_span: site.authored_span,
            interface: &derived.interface,
        })
    }
}

/// Checked source/recipe occurrence only: not an observed capture, dynamic
/// Program identity, admitted Wasm artifact, behavioral claim or host grant.
pub struct CheckedQuoteSite<'a> {
    definition_id: CheckedDefinitionId,
    source_generation: u64,
    node: noble_kernel::untrusted::NodeId,
    authored_span: Option<crate::Span>,
    interface: &'a noble_kernel::untrusted::Interface,
}

impl CheckedQuoteSite<'_> {
    pub const fn definition_id(&self) -> CheckedDefinitionId {
        self.definition_id
    }
    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }
    pub const fn node(&self) -> noble_kernel::untrusted::NodeId {
        self.node
    }
    pub const fn authored_span(&self) -> Option<crate::Span> {
        self.authored_span
    }
    pub const fn interface(&self) -> &noble_kernel::untrusted::Interface {
        self.interface
    }
}

/// Expected host context, not an attestation. The host must separately
/// authenticate its pinned checker and the installed artifact bytes.
#[derive(Clone, Copy)]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the type name is the domain term at its use sites and renaming would churn the extracted subject"
)]
pub struct ProofContext<'a> {
    pub module_generation: u64,
    pub model_revision: &'a str,
    pub checker_revision: &'a str,
}

/// An exact source/definition/interface match against one retained proof
/// record. This is deliberately NOT a proof receipt or release authorization:
/// `commit_verified` trusts its host callback, and this no_std crate cannot
/// attest a Lean process, its assumptions, or installed Wasm bytes.
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the type name is the domain term at its use sites and renaming would churn the extracted subject"
)]
pub struct SourceCorrespondenceCandidate<'a> {
    pub(crate) target_id: CheckedDefinitionId,
    pub(crate) target_generation: u64,
    pub(crate) module_generation: u64,
    pub(crate) goal: &'a crate::intrinsic::ContractGoal,
    pub(crate) retained: &'a crate::intrinsic::CheckedProof,
    pub(crate) interface: &'a noble_kernel::untrusted::Interface,
}

impl SourceCorrespondenceCandidate<'_> {
    pub const fn definition_id(&self) -> CheckedDefinitionId {
        self.target_id
    }
    pub const fn target_generation(&self) -> u64 {
        self.target_generation
    }
    pub const fn module_generation(&self) -> u64 {
        self.module_generation
    }
    pub const fn goal(&self) -> &crate::intrinsic::ContractGoal {
        self.goal
    }
    pub const fn retained_proof(&self) -> &crate::intrinsic::CheckedProof {
        self.retained
    }
    pub const fn interface(&self) -> &noble_kernel::untrusted::Interface {
        self.interface
    }
    /// The MC1 theorem has no captured inputs or external premises. A nonempty
    /// capture or dependency is refused before this candidate can be returned.
    pub const fn captures(&self) -> &[crate::companion::CaptureBinding] {
        &[]
    }
    pub const fn assumptions(&self) -> &[crate::intrinsic::ProofDependency] {
        &[]
    }
}

impl super::Session {
    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
    )]
    /// Select only the current retained definition from its complete accepted
    /// single-invocation submission. Recheck both root and definition body in
    /// the exact emitted environment; declarations alone have no authority.
    pub fn checked_selected_target<'a>(
        &'a self,
        prepared: &'a super::Prepared,
        name: &str,
    ) -> Result<CheckedSelectedTarget<'a>, SourceProofRefusal> {
        use noble_kernel::{
            acceptance,
            contracts::Behavior,
            untrusted::{Node, Outcome, Request},
        };
        #[expect(
            tigerstyle::non_trait_imports,
            reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
        )]
        #[expect(
            tigerstyle::renamed_imports,
            reason = "Owner: noble-contracts; the local alias matches the domain vocabulary used throughout this module"
        )]
        use SourceProofRefusal as Refusal;
        if self.live_slots.is_none() {
            return Err(Refusal::UnsupportedProfile);
        }
        if !self.is_current_namespace(prepared) {
            return Err(Refusal::StaleContext);
        }
        let (retained_index, retained) = self
            .definitions
            .iter()
            .enumerate()
            .rev()
            .find(|(_, definition)| definition.name == name)
            .ok_or(Refusal::MissingTarget)?;
        let source = self
            .definition_source(retained_index)
            .map_err(|_| Refusal::MismatchedSource)?;
        let submission = prepared.submission().ok_or(Refusal::MissingTarget)?;
        // A selected root calls one named body, whose checked dependency closure
        // may also contain earlier retained definitions.
        let [Node::Invocation { def: root_def, .. }] = submission.body.candidate.nodes.as_slice()
        else {
            return Err(Refusal::MismatchedDefinition);
        };
        let mut selected = submission
            .definitions
            .iter()
            .filter(|definition| definition.definition == *root_def);
        let definition = selected.next().ok_or(Refusal::MismatchedDefinition)?;
        if selected.next().is_some() {
            return Err(Refusal::MismatchedDefinition);
        }
        let index = u32::try_from(retained_index).map_err(|_| Refusal::MismatchedDefinition)?;
        if prepared.selected_root != Some(index)
            || definition.identity != retained.identity
            || submission.environment.kind(definition.definition) != Some(Behavior::Named)
            || usize::try_from(definition.definition.0)
                .ok()
                .and_then(|slot| submission.environment.definition_owners.get(slot))
                != Some(&None)
            || !submission.body.texts.is_empty()
            || submission.body.candidate.body.len() != 1
            || submission.body.candidate.body[0] != noble_kernel::untrusted::NodeId(0)
        {
            return Err(Refusal::MismatchedDefinition);
        }
        let Outcome::Accepted(root) = acceptance::check(
            &submission.environment,
            &submission.request,
            &submission.body.candidate,
        ) else {
            return Err(Refusal::IndependentCheckFailed);
        };
        let request = Request {
            input_bytes: submission.request.input_bytes,
            expected: definition.expected.clone(),
            limits: submission.request.limits,
        };
        let Outcome::Accepted(body) = acceptance::check(
            &submission.environment,
            &request,
            &definition.body.candidate,
        ) else {
            return Err(Refusal::IndependentCheckFailed);
        };
        if root.interface.stack_in != body.interface.stack_in
            || root.interface.stack_out != body.interface.stack_out
            || root.interface.effects != body.interface.effects
            || root.interface.stack_in != submission.request.expected.stack_in
            || root.interface.stack_out != submission.request.expected.stack_out
        {
            return Err(Refusal::MismatchedInterface);
        }
        // Kernel request work is only a downstream reservation, not the source replay cap.
        let (generated, witnesses, candidate_to_lexical) = replay_lexical_source(
            source,
            retained,
            self,
            retained_index,
            prepared.limits,
            &definition.body.candidate,
        )?;
        Ok(CheckedSelectedTarget {
            id: CheckedDefinitionId {
                index,
                identity: retained.identity,
            },
            generation: self.generation,
            name: &retained.name,
            source,
            submission,
            definition,
            root,
            body,
            tree: &retained.tree,
            generated,
            witnesses,
            candidate_to_lexical,
        })
    }
}
