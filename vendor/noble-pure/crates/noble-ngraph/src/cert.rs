//! Replayable certificates for a small set of pure `I64` graph rewrites.
//!
//! The transformation stays untrusted. A certificate names, per rewritten node, the
//! rule identity, the ordered operand identities, and the constant bits and claim each
//! rule requires in canonical form. [`check`] validates those premises against the
//! original graph without calling the emitter, replays the folds on a copy of the
//! original and requires the resulting graph to equal that replay, and endpoint digests
//! bind the exact original and resulting graphs. A digest alone binds the result the
//! producer named, not the result the steps produce, so the replay is what refuses a
//! substituted result. Refusals are typed rather than folded into one failure.
//!
//! A derivation also binds the semantic model revision it was produced against
//! ([`MODEL_REVISION`], the IR's `ngraph-sem/0`), so a checker for another model refuses
//! it instead of silently applying rules proved under different arithmetic. [`encode`]
//! and [`decode`] transport a derivation as the token envelope modelled in
//! `proofs/foundation/NobleFoundation/CertificateCodec.lean`: a revision token, a model
//! token, a declared step count and then exactly seven decimal tokens per step. The
//! decoder refuses a stale revision or model token, an over-bound declared count before it
//! allocates the step list, and truncated, dropped, extra, unknown-rule, out-of-range and
//! non-decimal tokens.
//!
//! [`check`] is stated over **admitted** endpoints. Admission (`crate::check`) is what
//! establishes the graph premise the Lean soundness theorems take as a hypothesis:
//! operands are defined strictly before their node (a forward reference is
//! `Malformed::ForwardReference`, a dependency cycle `Malformed::Cycle`), every slot is
//! defined once, and no node is dangling. The checker additionally refuses any step whose
//! premise reads a node at or after the rewritten node, so a cyclic or forward-referencing
//! step is malformed even when the caller skipped admission.
//!
//! This is the Rust side of the lane modelled in
//! `proofs/foundation/NobleFoundation/GraphRewrite.lean`; the model's rule,
//! congruence and checker-soundness theorems are stated over the Lean graph
//! denotation, and the fixed fixture is kernel-checked in the generated
//! `CertificateCorpus.lean`. No correspondence theorem binds this module's code (as
//! opposed to that fixture) to the model. `ngraph-cert/2` adds a separate chained
//! transition API: each rewrite is checked against its predecessor graph and an
//! optional final prune is checker-derived. Admission still runs independently on
//! both endpoints. No theorem binds this Rust parser, checker or prune implementation
//! to the Lean model. Wire/resource budgets, retained evidence sizes and every `F32`,
//! guard, effect or resource rewrite remain open.

use crate::identity::IdentityScheme;

/// Certificate format revision bound into every accepted derivation.
pub const CERT_REVISION: &str = "ngraph-cert/0";

/// The readable predecessor profile, whose rules remain bound to the original graph.
pub const LEGACY_CHAINLESS_REVISION: &str = "ngraph-cert/1";

/// Certificate format revision for graph-chained derivations and certified pruning.
pub const CHAINED_REVISION: &str = "ngraph-cert/2";

/// Semantic model revision bound into every accepted derivation: the rules are
/// sound for `ngraph-sem/0`'s wrapping `I64` arithmetic and for no other reading.
pub const MODEL_REVISION: &str = crate::ir::SEMANTIC_REVISION;

/// Declared bound on derivation steps: a certificate larger than this is refused
/// before any node is inspected.
pub const MAX_STEPS: usize = 64;

/// Tokens per step in the wire envelope: rule, index, first, second, first bits,
/// second bits and claimed.
pub const STEP_TOKENS: usize = 7;

/// Tokens before the step chunks: revision, model and declared step count.
pub const ENVELOPE_TOKENS: usize = 3;

/// Radix of every numeric token.
const DECIMAL_RADIX: u32 = 10;

/// Digits a `u64` decimal token can occupy.
const DECIMAL_DIGITS: usize = 20;

/// One accepted rewrite rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// An addition of two exact `I64` constants denotes their wrapping sum.
    FoldAddConstants,
    /// `0 + x` denotes `x`.
    NeutralAddLeft,
    /// `x * 1` denotes `x`.
    NeutralMulRight,
}

impl Rule {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FoldAddConstants => "fold-add-constants",
            Self::NeutralAddLeft => "neutral-add-left",
            Self::NeutralMulRight => "neutral-mul-right",
        }
    }
}

/// One derivation step: the whole premise, in ordered form.
///
/// Every field is canonical for the rule and [`check`] refuses a step that is not:
///
/// - [`Rule::FoldAddConstants`]: `first_bits` and `second_bits` are the exact bits of
///   the two operand constants, and `claimed` is their wrapping sum.
/// - [`Rule::NeutralAddLeft`]: the first operand is the graph constant `0`, `first_bits`
///   is `0`, `second_bits` equals `first_bits`, and `claimed` is `0`.
/// - [`Rule::NeutralMulRight`]: the second operand is the graph constant `1`,
///   `second_bits` is `1`, `first_bits` equals `second_bits`, and `claimed` is `0`.
///
/// An identity step records the cancelled identity element in both bit fields and
/// claims the other identity, `0`; it never claims the rewritten node's value, which
/// the surviving operand carries at run time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Rule identity.
    pub rule: Rule,
    /// The rewritten node.
    pub node: crate::ir::NodeId,
    /// Ordered operand identities, destination order preserved.
    pub operands: [crate::ir::NodeId; 2],
    /// The first recorded constant bits, canonical for the rule.
    pub first_bits: u64,
    /// The second recorded constant bits, canonical for the rule.
    pub second_bits: u64,
    /// The claimed value: the folded sum, or `0` for an identity step.
    pub claimed: u64,
}

/// A replayable derivation over exact endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    /// Format revision.
    pub revision: alloc::string::String,
    /// Semantic model revision the rules were proved against.
    pub model: alloc::string::String,
    /// Ordered steps; node identities strictly increase.
    pub steps: alloc::vec::Vec<Step>,
    /// Digest of the original graph.
    pub left: crate::ir::Digest,
    /// Digest of the resulting graph.
    pub right: crate::ir::Digest,
}

/// A transported certificate: exactly what the wire envelope carries.
///
/// The envelope names no endpoint digests, because endpoint identity is not part of
/// the modelled token format (`CertificateCodec.lean` decodes a revision, a model and
/// a step list). A receiver binds the transported steps to the endpoints it holds with
/// [`Envelope::bind`], and [`check`] then verifies those digests like any other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// Format revision, already refused if stale.
    pub revision: alloc::string::String,
    /// Semantic model revision, already refused if stale.
    pub model: alloc::string::String,
    /// Ordered steps exactly as transported.
    pub steps: alloc::vec::Vec<Step>,
}

/// One state transition in `ngraph-cert/2`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainedAction {
    /// Apply one rewrite to the graph state produced by the previous transition.
    Rewrite(Step),
    /// Remove precisely the nodes the checker independently derives as unreachable.
    PruneUnreachable,
}

/// One chained transition with the logical identities of both adjacent states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainedStep {
    /// The transition to replay.
    pub action: ChainedAction,
    /// Identity of the graph immediately before this transition.
    pub before: crate::ir::Digest,
    /// Identity of the graph immediately after this transition.
    pub after: crate::ir::Digest,
}

/// A replayable `ngraph-cert/2` derivation over exact endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainedDerivation {
    /// Format revision.
    pub revision: alloc::string::String,
    /// Semantic model revision the rules were proved against.
    pub model: alloc::string::String,
    /// Ordered graph transitions; every intermediate identity is recorded.
    pub steps: alloc::vec::Vec<ChainedStep>,
    /// Digest of the original graph.
    pub left: crate::ir::Digest,
    /// Digest of the resulting graph.
    pub right: crate::ir::Digest,
}

impl Envelope {
    /// Binds the transported steps to the endpoint digests the receiver will check.
    #[must_use]
    pub fn bind(self, left: crate::ir::Digest, right: crate::ir::Digest) -> Derivation {
        Derivation {
            revision: self.revision,
            model: self.model,
            steps: self.steps,
            left,
            right,
        }
    }
}

/// Why a certificate was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The format revision is not the current one.
    StaleRevision,
    /// The semantic model revision is not the one the rules were proved against.
    StaleModel,
    /// The derivation exceeds the declared step bound.
    TooManySteps,
    /// A step is malformed, unsupported or names a node of another kind.
    Malformed,
    /// A declared constant does not match the graph's own bits.
    ConstantMismatch,
    /// The claimed value is not what the rule computes for those bits.
    ClaimMismatch,
    /// The steps are not in strictly increasing node order.
    Order,
    /// The certificate does not name the endpoints it was checked against.
    EndpointMismatch,
    /// The resulting graph is not the original with the derivation's folds applied.
    ResultMismatch,
}

/// The wire numbering of a rule identity, matching the modelled rule set.
const fn rule_token(rule: Rule) -> u64 {
    match rule {
        Rule::FoldAddConstants => 0,
        Rule::NeutralAddLeft => 1,
        Rule::NeutralMulRight => 2,
    }
}

/// The rule identity a wire token names: a token outside the modelled set is unknown.
const fn decode_rule(token: u64) -> Option<Rule> {
    match token {
        0 => Some(Rule::FoldAddConstants),
        1 => Some(Rule::NeutralAddLeft),
        2 => Some(Rule::NeutralMulRight),
        _ => None,
    }
}

/// Appends `value` as its decimal token.
// justification: appending to an owned String is the required operation; slices and str cannot
// provide this mutation API, and a generic formatting sink would make an infallible write fallible.
#[allow(tigerstyle::borrowed_argument_types)]
fn push_decimal(out: &mut alloc::string::String, value: u64) {
    let mut digits = ['0'; DECIMAL_DIGITS];
    let mut at = DECIMAL_DIGITS;
    let mut rest = value;
    let radix = u64::from(DECIMAL_RADIX);
    assert!(radix != 0);
    loop {
        at -= 1;
        let digit = u32::try_from(rest % radix).unwrap_or(0);
        digits[at] = char::from_digit(digit, DECIMAL_RADIX).unwrap_or('0');
        rest /= radix;
        if rest == 0 {
            break;
        }
    }
    for &digit in &digits[at..] {
        out.push(digit);
    }
}

/// Parses one decimal token: digits only, no sign, no prefix, no overflow.
fn decimal(token: &str) -> Option<u64> {
    if token.is_empty() {
        return None;
    }
    let mut value: u64 = 0;
    for character in token.chars() {
        let digit = character.to_digit(DECIMAL_RADIX)?;
        value = value
            .checked_mul(u64::from(DECIMAL_RADIX))?
            .checked_add(u64::from(digit))?;
    }
    Some(value)
}

/// Encodes a derivation as the token envelope the modelled codec reads: the revision
/// token, the model token, the declared step count and then exactly seven decimal
/// tokens per step, separated by single spaces.
///
/// The envelope carries no endpoint digests, exactly like the modelled format; a
/// receiver binds the decoded steps to the endpoints it holds
/// ([`Envelope::bind`]) before calling [`check`].
#[must_use]
pub fn encode(derivation: &Derivation) -> alloc::string::String {
    let mut wire = alloc::string::String::new();
    wire.push_str(&derivation.revision);
    wire.push(' ');
    wire.push_str(&derivation.model);
    wire.push(' ');
    push_decimal(
        &mut wire,
        u64::try_from(derivation.steps.len()).unwrap_or(u64::MAX),
    );
    for step in &derivation.steps {
        for value in [
            rule_token(step.rule),
            u64::from(step.node.0),
            u64::from(step.operands[0].0),
            u64::from(step.operands[1].0),
            step.first_bits,
            step.second_bits,
            step.claimed,
        ] {
            wire.push(' ');
            push_decimal(&mut wire, value);
        }
    }
    wire
}

fn next_token<'wire>(tokens: &mut core::str::Split<'wire, char>) -> Result<&'wire str, Refusal> {
    tokens.next().ok_or(Refusal::Malformed)
}

fn decimal_token(tokens: &mut core::str::Split<'_, char>) -> Result<u64, Refusal> {
    decimal(next_token(tokens)?).ok_or(Refusal::Malformed)
}

fn node_token(tokens: &mut core::str::Split<'_, char>) -> Result<crate::ir::NodeId, Refusal> {
    let index = u32::try_from(decimal_token(tokens)?).map_err(|_| Refusal::Malformed)?;
    Ok(crate::ir::NodeId(index))
}

/// Decodes one step from exactly seven tokens.
fn decode_step(tokens: &mut core::str::Split<'_, char>) -> Result<Step, Refusal> {
    let rule = decode_rule(decimal_token(tokens)?).ok_or(Refusal::Malformed)?;
    Ok(Step {
        rule,
        node: node_token(tokens)?,
        operands: [node_token(tokens)?, node_token(tokens)?],
        first_bits: decimal_token(tokens)?,
        second_bits: decimal_token(tokens)?,
        claimed: decimal_token(tokens)?,
    })
}

/// Decodes the token envelope [`encode`] writes.
///
/// Refusals follow the modelled order: a stale revision token ([`Refusal::StaleRevision`]),
/// a stale model token ([`Refusal::StaleModel`]), a non-decimal or out-of-range declared
/// count ([`Refusal::Malformed`]), a declared count over [`MAX_STEPS`]
/// ([`Refusal::TooManySteps`], refused before the step list is allocated) and per-step
/// non-decimal, out-of-range or unknown-rule tokens, truncated steps, dropped steps and
/// extra tokens ([`Refusal::Malformed`]).
pub fn decode(wire: &str) -> Result<Envelope, Refusal> {
    if wire.is_empty() {
        return Err(Refusal::Malformed);
    }
    let mut tokens = wire.split(' ');
    let revision = next_token(&mut tokens)?;
    if revision != CERT_REVISION && revision != LEGACY_CHAINLESS_REVISION {
        return Err(Refusal::StaleRevision);
    }
    let model = next_token(&mut tokens)?;
    if model != MODEL_REVISION {
        return Err(Refusal::StaleModel);
    }
    let declared = decimal_token(&mut tokens)?;
    let count = usize::try_from(declared).map_err(|_| Refusal::TooManySteps)?;
    if count > MAX_STEPS {
        return Err(Refusal::TooManySteps);
    }
    let mut steps = alloc::vec::Vec::with_capacity(count);
    for _ in 0..count {
        steps.push(decode_step(&mut tokens)?);
    }
    if tokens.next().is_some() {
        return Err(Refusal::Malformed);
    }
    Ok(Envelope {
        revision: alloc::string::String::from(revision),
        model: alloc::string::String::from(model),
        steps,
    })
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; direct String append avoids a temporary allocation or copy"
)]
fn push_digest(out: &mut alloc::string::String, digest: &crate::ir::Digest) {
    for byte in digest {
        out.push(' ');
        push_decimal(out, u64::from(*byte));
    }
}

fn decode_digest(tokens: &mut core::str::Split<'_, char>) -> Result<crate::ir::Digest, Refusal> {
    let mut digest = [0u8; 32];
    for byte in &mut digest {
        *byte = u8::try_from(decimal_token(tokens)?).map_err(|_| Refusal::Malformed)?;
    }
    Ok(digest)
}

/// Encodes an `ngraph-cert/2` derivation as a bounded decimal-token envelope.
///
/// The header carries both endpoint identities. Each transition carries a kind,
/// its rewrite premise when present, and the 32-byte identities of the graph state
/// before and after the transition.
#[must_use]
pub fn encode_chained(derivation: &ChainedDerivation) -> alloc::string::String {
    let mut out = alloc::string::String::with_capacity(
        256usize.saturating_add(derivation.steps.len().saturating_mul(256)),
    );
    out.push_str(&derivation.revision);
    out.push(' ');
    out.push_str(&derivation.model);
    out.push(' ');
    push_decimal(&mut out, derivation.steps.len() as u64);
    push_digest(&mut out, &derivation.left);
    push_digest(&mut out, &derivation.right);
    for transition in &derivation.steps {
        out.push(' ');
        match &transition.action {
            ChainedAction::Rewrite(step) => {
                push_decimal(&mut out, 0);
                out.push(' ');
                push_decimal(&mut out, rule_token(step.rule));
                out.push(' ');
                push_decimal(&mut out, u64::from(step.node.0));
                for operand in step.operands {
                    out.push(' ');
                    push_decimal(&mut out, u64::from(operand.0));
                }
                out.push(' ');
                push_decimal(&mut out, step.first_bits);
                out.push(' ');
                push_decimal(&mut out, step.second_bits);
                out.push(' ');
                push_decimal(&mut out, step.claimed);
            }
            ChainedAction::PruneUnreachable => push_decimal(&mut out, 1),
        }
        push_digest(&mut out, &transition.before);
        push_digest(&mut out, &transition.after);
    }
    out
}

/// Decodes one `ngraph-cert/2` envelope and refuses legacy profiles.
pub fn decode_chained(wire: &str) -> Result<ChainedDerivation, Refusal> {
    if wire.is_empty() {
        return Err(Refusal::Malformed);
    }
    let mut tokens = wire.split(' ');
    let revision = next_token(&mut tokens)?;
    if revision != CHAINED_REVISION {
        return Err(Refusal::StaleRevision);
    }
    let model = next_token(&mut tokens)?;
    if model != MODEL_REVISION {
        return Err(Refusal::StaleModel);
    }
    let count = usize::try_from(decimal_token(&mut tokens)?).map_err(|_| Refusal::TooManySteps)?;
    if count > MAX_STEPS {
        return Err(Refusal::TooManySteps);
    }
    let left = decode_digest(&mut tokens)?;
    let right = decode_digest(&mut tokens)?;
    let mut steps = alloc::vec::Vec::with_capacity(count);
    for _ in 0..count {
        let kind = decimal_token(&mut tokens)?;
        let action = match kind {
            0 => {
                let rule = decode_rule(decimal_token(&mut tokens)?).ok_or(Refusal::Malformed)?;
                ChainedAction::Rewrite(Step {
                    rule,
                    node: node_token(&mut tokens)?,
                    operands: [node_token(&mut tokens)?, node_token(&mut tokens)?],
                    first_bits: decimal_token(&mut tokens)?,
                    second_bits: decimal_token(&mut tokens)?,
                    claimed: decimal_token(&mut tokens)?,
                })
            }
            1 => ChainedAction::PruneUnreachable,
            _ => return Err(Refusal::Malformed),
        };
        steps.push(ChainedStep {
            action,
            before: decode_digest(&mut tokens)?,
            after: decode_digest(&mut tokens)?,
        });
    }
    if tokens.next().is_some() {
        return Err(Refusal::Malformed);
    }
    Ok(ChainedDerivation {
        revision: alloc::string::String::from(revision),
        model: alloc::string::String::from(model),
        steps,
        left,
        right,
    })
}

fn bits(lit: crate::ir::Literal) -> Option<u64> {
    match lit {
        crate::ir::Literal::I64(value) => Some(value.cast_unsigned()),
        crate::ir::Literal::Bool(_) | crate::ir::Literal::F32(_) => None,
    }
}

fn constant_bits(graph: &crate::ir::Graph, node: crate::ir::NodeId) -> Option<u64> {
    graph
        .nodes
        .get(usize::try_from(node.0).ok()?)
        .and_then(|n| match n.op {
            crate::ir::Op::Const(lit) => bits(lit),
            crate::ir::Op::Input(_)
            | crate::ir::Op::Param(_)
            | crate::ir::Op::I64Add(_, _)
            | crate::ir::Op::I64Sub(_, _)
            | crate::ir::Op::I64Mul(_, _)
            | crate::ir::Op::I64Eq(_, _)
            | crate::ir::Op::F32Add(_, _)
            | crate::ir::Op::F32Sub(_, _)
            | crate::ir::Op::F32Mul(_, _)
            | crate::ir::Op::F32Affine { .. }
            | crate::ir::Op::F32Relu(_)
            | crate::ir::Op::F32Reglu { .. }
            | crate::ir::Op::F32Step(_)
            | crate::ir::Op::F32Select { .. }
            | crate::ir::Op::F32Index { .. }
            | crate::ir::Op::Guard { .. } => None,
        })
}

fn wrapping_add(first: u64, second: u64) -> u64 {
    first.wrapping_add(second)
}

/// Emits a derivation for every `I64` addition of two exact constants and every
/// `x * 1` multiplication, with the resulting graph in which each such addition is
/// replaced, in place, by its constant. The `x * 1` steps are recorded canonical but
/// not applied: replacing the node would change its identity, which is a separate
/// contract. `0 + x` is not recorded at all.
#[must_use]
pub fn emit(ids: &dyn IdentityScheme, graph: &crate::ir::Graph) -> (Derivation, crate::ir::Graph) {
    let mut result = graph.clone();
    let mut steps = alloc::vec::Vec::with_capacity(graph.nodes.len());
    for (index, node) in graph.nodes.iter().enumerate() {
        let Ok(index) = u32::try_from(index) else {
            continue;
        };
        match node.op {
            crate::ir::Op::I64Add(first, second) => {
                let (Some(first_bits), Some(second_bits)) =
                    (constant_bits(graph, first), constant_bits(graph, second))
                else {
                    continue;
                };
                steps.push(Step {
                    rule: Rule::FoldAddConstants,
                    node: crate::ir::NodeId(index),
                    operands: [first, second],
                    first_bits,
                    second_bits,
                    claimed: wrapping_add(first_bits, second_bits),
                });
                if let Some(target) = result.nodes.get_mut(usize::try_from(index).unwrap_or(0)) {
                    target.op = crate::ir::Op::Const(crate::ir::Literal::I64(
                        wrapping_add(first_bits, second_bits).cast_signed(),
                    ));
                }
            }
            crate::ir::Op::I64Mul(first, second) if constant_bits(graph, second) == Some(1) => {
                steps.push(Step {
                    rule: Rule::NeutralMulRight,
                    node: crate::ir::NodeId(index),
                    operands: [first, second],
                    // Canonical identity premise: both recorded bit fields hold the
                    // cancelled identity `1`, and the claim is the other identity `0`.
                    first_bits: 1,
                    second_bits: 1,
                    claimed: 0,
                });
            }
            crate::ir::Op::Input(_)
            | crate::ir::Op::Param(_)
            | crate::ir::Op::Const(_)
            | crate::ir::Op::I64Sub(_, _)
            | crate::ir::Op::I64Eq(_, _)
            | crate::ir::Op::F32Add(_, _)
            | crate::ir::Op::F32Sub(_, _)
            | crate::ir::Op::F32Mul(_, _)
            | crate::ir::Op::F32Affine { .. }
            | crate::ir::Op::F32Relu(_)
            | crate::ir::Op::F32Reglu { .. }
            | crate::ir::Op::F32Step(_)
            | crate::ir::Op::F32Select { .. }
            | crate::ir::Op::F32Index { .. }
            | crate::ir::Op::Guard { .. }
            | crate::ir::Op::I64Mul(_, _) => {}
        }
    }
    let derivation = Derivation {
        revision: alloc::string::String::from(CERT_REVISION),
        model: alloc::string::String::from(MODEL_REVISION),
        steps,
        left: graph.logical_id(ids),
        right: result.logical_id(ids),
    };
    (derivation, result)
}

/// Which rewrites a proposer offers.
///
/// A weaker proposer can still emit a **valid** certificate, so certificate validity alone never
/// says the transformation did useful work. Issue #64 measures the two separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proposer {
    /// Emit no step at all: the empty derivation over identical endpoints.
    Identity,
    /// Emit exactly one step, the first eligible fold in node order.
    FirstEligible,
    /// One ascending pass over the nodes: a node is offered a rewrite once and never revisited.
    SinglePass,
    /// Repeat passes until no fold is applicable.
    FixedPoint,
}

/// The fold step for `index`, if the node is an `I64` addition of two exact constants.
fn fold_site(graph: &crate::ir::Graph, index: usize) -> Option<Step> {
    let node = graph.nodes.get(index)?;
    let crate::ir::Op::I64Add(first, second) = node.op else {
        return None;
    };
    let (Some(first_bits), Some(second_bits)) =
        (constant_bits(graph, first), constant_bits(graph, second))
    else {
        return None;
    };
    Some(Step {
        rule: Rule::FoldAddConstants,
        node: crate::ir::NodeId(u32::try_from(index).ok()?),
        operands: [first, second],
        first_bits,
        second_bits,
        claimed: wrapping_add(first_bits, second_bits),
    })
}

/// Replaces the step's node by its folded constant.
fn apply_fold(graph: &mut crate::ir::Graph, step: &Step) {
    let Ok(index) = usize::try_from(step.node.0) else {
        return;
    };
    if let Some(target) = graph.nodes.get_mut(index) {
        target.op = crate::ir::Op::Const(crate::ir::Literal::I64(step.claimed.cast_signed()));
    }
}

/// The number of nodes where `fold-add-constants` still applies: the residual property a quality
/// target names, counted separately from certificate validity.
#[must_use]
pub fn residual_fold_sites(graph: &crate::ir::Graph) -> usize {
    (0..graph.nodes.len())
        .filter(|index| fold_site(graph, *index).is_some())
        .count()
}

/// Proposes rewrites under `proposer` and returns the derivation with the graph it produces.
///
/// Steps stay in strictly increasing node order in every case, so each proposer's certificate is
/// checked by the same validator; the proposers differ only in how much work they do.
#[must_use]
pub fn propose(
    ids: &dyn IdentityScheme,
    proposer: Proposer,
    graph: &crate::ir::Graph,
) -> (Derivation, crate::ir::Graph) {
    let mut result = graph.clone();
    let mut steps = alloc::vec::Vec::new();
    match proposer {
        Proposer::Identity => {}
        Proposer::FirstEligible => {
            if let Some(step) = (0..graph.nodes.len()).find_map(|index| fold_site(&result, index)) {
                apply_fold(&mut result, &step);
                steps.push(step);
            }
        }
        Proposer::SinglePass => {
            // Recorded from the original graph so a step's premises stay the ones the validator
            // compares, then applied to the result in ascending order.
            let sites: alloc::vec::Vec<Step> = (0..graph.nodes.len())
                .filter_map(|index| fold_site(graph, index))
                .collect();
            for step in sites {
                apply_fold(&mut result, &step);
                steps.push(step);
            }
        }
        Proposer::FixedPoint => {
            while let Some(step) =
                (0..result.nodes.len()).find_map(|index| fold_site(&result, index))
            {
                apply_fold(&mut result, &step);
                steps.push(step);
            }
        }
    }
    steps.sort_by_key(|step| step.node.0);
    let derivation = Derivation {
        revision: alloc::string::String::from(CERT_REVISION),
        model: alloc::string::String::from(MODEL_REVISION),
        steps,
        left: graph.logical_id(ids),
        right: result.logical_id(ids),
    };
    (derivation, result)
}

fn checked_bits(
    graph: &crate::ir::Graph,
    node: crate::ir::NodeId,
    declared: u64,
) -> Result<u64, Refusal> {
    match constant_bits(graph, node) {
        Some(actual) if actual == declared => Ok(actual),
        Some(_) => Err(Refusal::ConstantMismatch),
        None => Err(Refusal::Malformed),
    }
}

/// Whether an identity step's recorded bits are the canonical ones: the operand the rule
/// cancels is the graph's own constant holding `identity_bits`, and both recorded fields
/// hold that same constant.
fn identity_bits_are_canonical(
    graph: &crate::ir::Graph,
    operand: crate::ir::NodeId,
    declared_bits: [u64; 2],
    identity_bits: u64,
) -> bool {
    let [first_bits, second_bits] = declared_bits;
    constant_bits(graph, operand) == Some(identity_bits)
        && first_bits == identity_bits
        && second_bits == identity_bits
}

/// The graph the steps denote: each fold replaces its node, in place, by the claimed
/// constant, and identity steps are recorded but not applied, as the proposers record
/// them. Written beside the checker rather than shared with the proposers, so the checker
/// never runs producer code.
fn replay(graph: &crate::ir::Graph, derivation: &Derivation) -> Result<crate::ir::Graph, Refusal> {
    let mut replayed = graph.clone();
    for step in &derivation.steps {
        match step.rule {
            Rule::FoldAddConstants => {
                let index = usize::try_from(step.node.0).map_err(|_| Refusal::Malformed)?;
                let node = replayed.nodes.get_mut(index).ok_or(Refusal::Malformed)?;
                node.op = crate::ir::Op::Const(crate::ir::Literal::I64(step.claimed.cast_signed()));
            }
            Rule::NeutralAddLeft | Rule::NeutralMulRight => {}
        }
    }
    Ok(replayed)
}

fn remap_node(
    mapping: &[Option<crate::ir::NodeId>],
    node: crate::ir::NodeId,
) -> Result<crate::ir::NodeId, Refusal> {
    mapping
        .get(node.index())
        .copied()
        .flatten()
        .ok_or(Refusal::Malformed)
}

fn remap_fact(
    fact: &crate::ir::Fact,
    mapping: &[Option<crate::ir::NodeId>],
) -> Result<crate::ir::Fact, Refusal> {
    Ok(match fact {
        crate::ir::Fact::Range { lo, hi } => crate::ir::Fact::Range { lo: *lo, hi: *hi },
        crate::ir::Fact::Grid { log2 } => crate::ir::Fact::Grid { log2: *log2 },
        crate::ir::Fact::Bit => crate::ir::Fact::Bit,
        crate::ir::Fact::OneHot { group, allow_zero } => crate::ir::Fact::OneHot {
            group: group
                .iter()
                .map(|node| remap_node(mapping, *node))
                .collect::<Result<_, _>>()?,
            allow_zero: *allow_zero,
        },
        crate::ir::Fact::Finite => crate::ir::Fact::Finite,
        crate::ir::Fact::Shape(size) => crate::ir::Fact::Shape(*size),
        crate::ir::Fact::Integral => crate::ir::Fact::Integral,
    })
}

fn remap_op(
    op: &crate::ir::Op,
    mapping: &[Option<crate::ir::NodeId>],
) -> Result<crate::ir::Op, Refusal> {
    let remap = |node| remap_node(mapping, node);
    Ok(match op {
        crate::ir::Op::Input(slot) => crate::ir::Op::Input(*slot),
        crate::ir::Op::Param(slot) => crate::ir::Op::Param(*slot),
        crate::ir::Op::Const(literal) => crate::ir::Op::Const(*literal),
        crate::ir::Op::I64Add(a, b) => crate::ir::Op::I64Add(remap(*a)?, remap(*b)?),
        crate::ir::Op::I64Sub(a, b) => crate::ir::Op::I64Sub(remap(*a)?, remap(*b)?),
        crate::ir::Op::I64Mul(a, b) => crate::ir::Op::I64Mul(remap(*a)?, remap(*b)?),
        crate::ir::Op::I64Eq(a, b) => crate::ir::Op::I64Eq(remap(*a)?, remap(*b)?),
        crate::ir::Op::F32Add(a, b) => crate::ir::Op::F32Add(remap(*a)?, remap(*b)?),
        crate::ir::Op::F32Sub(a, b) => crate::ir::Op::F32Sub(remap(*a)?, remap(*b)?),
        crate::ir::Op::F32Mul(a, b) => crate::ir::Op::F32Mul(remap(*a)?, remap(*b)?),
        crate::ir::Op::F32Affine { bias, terms } => crate::ir::Op::F32Affine {
            bias: *bias,
            terms: terms
                .iter()
                .map(|term| {
                    Ok(crate::ir::AffineTerm {
                        coefficient: term.coefficient,
                        operand: match term.operand {
                            crate::ir::AffineOperand::One => crate::ir::AffineOperand::One,
                            crate::ir::AffineOperand::Node(node) => {
                                crate::ir::AffineOperand::Node(remap(node)?)
                            }
                        },
                    })
                })
                .collect::<Result<_, Refusal>>()?,
        },
        crate::ir::Op::F32Relu(node) => crate::ir::Op::F32Relu(remap(*node)?),
        crate::ir::Op::F32Reglu { left, gate } => crate::ir::Op::F32Reglu {
            left: remap(*left)?,
            gate: remap(*gate)?,
        },
        crate::ir::Op::F32Step(node) => crate::ir::Op::F32Step(remap(*node)?),
        crate::ir::Op::F32Select { bit, off, on } => crate::ir::Op::F32Select {
            bit: remap(*bit)?,
            off: remap(*off)?,
            on: remap(*on)?,
        },
        crate::ir::Op::F32Index {
            index,
            table,
            below,
            above,
        } => crate::ir::Op::F32Index {
            index: remap(*index)?,
            table: table
                .iter()
                .map(|node| remap(*node))
                .collect::<Result<_, _>>()?,
            below: *below,
            above: *above,
        },
        crate::ir::Op::Guard { fact, value } => crate::ir::Op::Guard {
            fact: remap_fact(fact, mapping)?,
            value: remap(*value)?,
        },
    })
}

/// Computes the unique creation-order pruning of unreachable non-interface graph nodes.
///
/// Roots include ordered outputs, guard roots, interface nodes (whose slot numbering is part
/// of the checked graph contract), and all references held by fact records. Operands are
/// traversed in their original order. Survivors are copied in creation order, which makes the
/// induced old-to-new map a bijection and preserves operand and output order by construction.
fn prune_unreachable(graph: &crate::ir::Graph) -> Result<crate::ir::Graph, Refusal> {
    let node_count = graph.nodes.len();
    let mut live = alloc::vec![false; node_count];
    let mut work = graph.outputs.clone();
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.op.is_guard()
            || matches!(node.op, crate::ir::Op::Input(_) | crate::ir::Op::Param(_))
        {
            work.push(crate::ir::NodeId(
                u32::try_from(index).map_err(|_| Refusal::Malformed)?,
            ));
        }
    }
    for fact in &graph.facts {
        work.push(fact.node);
        if let crate::ir::Fact::OneHot { group, .. } = &fact.fact {
            work.extend_from_slice(group);
        }
        if let crate::ir::Provenance::Guard(guard) = &fact.provenance {
            work.push(*guard);
        }
    }
    while let Some(node) = work.pop() {
        let Some(is_live) = live.get_mut(node.index()) else {
            return Err(Refusal::Malformed);
        };
        if core::mem::replace(is_live, true) {
            continue;
        }
        let Some(source) = graph.nodes.get(node.index()) else {
            return Err(Refusal::Malformed);
        };
        source.op.visit_operands(|operand| work.push(operand));
    }

    let mut mapping = alloc::vec![None; node_count];
    let mut next = 0u32;
    for (old, is_live) in live.iter().copied().enumerate() {
        if is_live {
            let old = u32::try_from(old).map_err(|_| Refusal::Malformed)?;
            let old_index = usize::try_from(old).map_err(|_| Refusal::Malformed)?;
            mapping[old_index] = Some(crate::ir::NodeId(next));
            next = next.checked_add(1).ok_or(Refusal::Malformed)?;
        }
    }
    let mut pruned = graph.clone();
    pruned.nodes = alloc::vec::Vec::with_capacity(mapping.iter().flatten().count());
    for (index, node) in graph.nodes.iter().enumerate() {
        if live[index] {
            pruned.nodes.push(crate::ir::Node {
                op: remap_op(&node.op, &mapping)?,
                origin: node.origin.clone(),
            });
        }
    }
    pruned.outputs = graph
        .outputs
        .iter()
        .map(|node| remap_node(&mapping, *node))
        .collect::<Result<_, _>>()?;
    pruned.facts = graph
        .facts
        .iter()
        .map(|fact| {
            Ok(crate::ir::FactRecord {
                node: remap_node(&mapping, fact.node)?,
                fact: remap_fact(&fact.fact, &mapping)?,
                provenance: match &fact.provenance {
                    crate::ir::Provenance::Guard(guard) => {
                        crate::ir::Provenance::Guard(remap_node(&mapping, *guard)?)
                    }
                    other => other.clone(),
                },
            })
        })
        .collect::<Result<_, Refusal>>()?;
    Ok(pruned)
}

fn validate_chained_rule(graph: &crate::ir::Graph, step: Step) -> Result<(), Refusal> {
    let index = step.node.index();
    for operand in step.operands {
        if operand.index() >= index {
            return Err(Refusal::Malformed);
        }
    }
    let op = &graph.nodes.get(index).ok_or(Refusal::Malformed)?.op;
    match (step.rule, op) {
        (Rule::FoldAddConstants, crate::ir::Op::I64Add(first, second)) => {
            if step.operands != [*first, *second] {
                return Err(Refusal::Malformed);
            }
            let first_bits = checked_bits(graph, *first, step.first_bits)?;
            let second_bits = checked_bits(graph, *second, step.second_bits)?;
            if step.claimed != wrapping_add(first_bits, second_bits) {
                return Err(Refusal::ClaimMismatch);
            }
        }
        (Rule::NeutralAddLeft, crate::ir::Op::I64Add(first, second)) => {
            if step.operands != [*first, *second]
                || !identity_bits_are_canonical(
                    graph,
                    *first,
                    [step.first_bits, step.second_bits],
                    0,
                )
                || step.claimed != 0
            {
                return Err(Refusal::Malformed);
            }
        }
        (Rule::NeutralMulRight, crate::ir::Op::I64Mul(first, second)) => {
            if step.operands != [*first, *second]
                || !identity_bits_are_canonical(
                    graph,
                    *second,
                    [step.first_bits, step.second_bits],
                    1,
                )
                || step.claimed != 0
            {
                return Err(Refusal::Malformed);
            }
        }
        _ => return Err(Refusal::Malformed),
    }
    Ok(())
}

fn replay_chained_rule(graph: &crate::ir::Graph, step: Step) -> Result<crate::ir::Graph, Refusal> {
    let mut result = graph.clone();
    if step.rule == Rule::FoldAddConstants {
        let node = result
            .nodes
            .get_mut(step.node.index())
            .ok_or(Refusal::Malformed)?;
        node.op = crate::ir::Op::Const(crate::ir::Literal::I64(step.claimed.cast_signed()));
    }
    Ok(result)
}

fn record_chained_rule(
    ids: &dyn IdentityScheme,
    graph: &mut crate::ir::Graph,
    step: Step,
) -> Result<ChainedStep, Refusal> {
    let before = graph.logical_id(ids);
    validate_chained_rule(graph, step)?;
    *graph = replay_chained_rule(graph, step)?;
    let after = graph.logical_id(ids);
    Ok(ChainedStep {
        action: ChainedAction::Rewrite(step),
        before,
        after,
    })
}

/// Proposes a chained derivation and its result graph under `ngraph-cert/2`.
///
/// Unlike the legacy profile, each rewrite premise is checked against the state produced by
/// the preceding step. A final certified prune is emitted only when it removes unreachable
/// non-interface nodes; admission itself is unchanged and still runs on both endpoints.
#[must_use = "chained rewrite proposals must be checked before execution"]
pub fn propose_chained(
    ids: &dyn IdentityScheme,
    proposer: Proposer,
    graph: &crate::ir::Graph,
) -> Result<(ChainedDerivation, crate::ir::Graph), Refusal> {
    let mut result = graph.clone();
    let mut steps = alloc::vec::Vec::new();
    match proposer {
        Proposer::Identity => {}
        Proposer::FirstEligible => {
            if let Some(step) = (0..result.nodes.len()).find_map(|index| fold_site(&result, index))
            {
                steps.push(record_chained_rule(ids, &mut result, step)?);
            }
        }
        Proposer::SinglePass => {
            let sites: alloc::vec::Vec<Step> = (0..graph.nodes.len())
                .filter_map(|index| fold_site(graph, index))
                .collect();
            for step in sites {
                steps.push(record_chained_rule(ids, &mut result, step)?);
            }
        }
        Proposer::FixedPoint => {
            while let Some(step) =
                (0..result.nodes.len()).find_map(|index| fold_site(&result, index))
            {
                steps.push(record_chained_rule(ids, &mut result, step)?);
            }
        }
    }
    let pruned = prune_unreachable(&result)?;
    if pruned != result {
        steps.push(ChainedStep {
            action: ChainedAction::PruneUnreachable,
            before: result.logical_id(ids),
            after: pruned.logical_id(ids),
        });
        result = pruned;
    }
    let derivation = ChainedDerivation {
        revision: alloc::string::String::from(CHAINED_REVISION),
        model: alloc::string::String::from(MODEL_REVISION),
        steps,
        left: graph.logical_id(ids),
        right: result.logical_id(ids),
    };
    Ok((derivation, result))
}

/// Checks every `ngraph-cert/2` transition by independently replaying the exact graph state.
///
/// The verifier checks the before/after logical identity of every transition, re-derives every
/// rewrite premise from the current state, recomputes the unique pruning map, and compares the
/// exact replayed endpoint with `result`. Pruning is permitted only as the final transition.
pub fn check_chained(
    ids: &dyn IdentityScheme,
    graph: &crate::ir::Graph,
    result: &crate::ir::Graph,
    derivation: &ChainedDerivation,
) -> Result<(), Refusal> {
    if derivation.revision != CHAINED_REVISION {
        return Err(Refusal::StaleRevision);
    }
    if derivation.model != MODEL_REVISION {
        return Err(Refusal::StaleModel);
    }
    if derivation.steps.len() > MAX_STEPS {
        return Err(Refusal::TooManySteps);
    }
    if graph.logical_id(ids) != derivation.left || result.logical_id(ids) != derivation.right {
        return Err(Refusal::EndpointMismatch);
    }
    let mut current = graph.clone();
    let mut previous = None;
    let mut is_pruned = false;
    for (position, transition) in derivation.steps.iter().enumerate() {
        if current.logical_id(ids) != transition.before {
            return Err(Refusal::EndpointMismatch);
        }
        current = match &transition.action {
            ChainedAction::Rewrite(step) => {
                if is_pruned || previous.is_some_and(|prior| step.node.0 <= prior) {
                    return Err(Refusal::Order);
                }
                previous = Some(step.node.0);
                validate_chained_rule(&current, *step)?;
                replay_chained_rule(&current, *step)?
            }
            ChainedAction::PruneUnreachable => {
                if is_pruned || position + 1 != derivation.steps.len() {
                    return Err(Refusal::Order);
                }
                is_pruned = true;
                prune_unreachable(&current)?
            }
        };
        if current.logical_id(ids) != transition.after {
            return Err(Refusal::ResultMismatch);
        }
    }
    if current != *result {
        return Err(Refusal::ResultMismatch);
    }
    Ok(())
}

/// Validates a derivation against the original and the resulting graph. The
/// operand order is compared, never re-derived from the claimed value, every
/// recorded bit field and claim must be the canonical one for its rule (see
/// [`Step`]), and the resulting graph must equal the original with the folds
/// replayed.
///
/// The derivation premise is exactly the modelled `wellFormed`: the format revision,
/// the semantic model revision, the declared step bound and strictly increasing node
/// order. The graph premise is assumed from admission, and the checker independently
/// refuses a step whose premise reads a node at or after the rewritten node, so a
/// cyclic or forward-referencing certificate is malformed even on an unadmitted graph.
pub fn check(
    ids: &dyn IdentityScheme,
    graph: &crate::ir::Graph,
    result: &crate::ir::Graph,
    derivation: &Derivation,
) -> Result<(), Refusal> {
    if derivation.revision != CERT_REVISION && derivation.revision != LEGACY_CHAINLESS_REVISION {
        return Err(Refusal::StaleRevision);
    }
    if derivation.model != MODEL_REVISION {
        return Err(Refusal::StaleModel);
    }
    if derivation.steps.len() > MAX_STEPS {
        return Err(Refusal::TooManySteps);
    }
    if graph.logical_id(ids) != derivation.left || result.logical_id(ids) != derivation.right {
        return Err(Refusal::EndpointMismatch);
    }
    let mut previous: Option<u32> = None;
    for step in &derivation.steps {
        if previous.is_some_and(|earlier| step.node.0 <= earlier) {
            return Err(Refusal::Order);
        }
        previous = Some(step.node.0);
        let index = usize::try_from(step.node.0).map_err(|_| Refusal::Malformed)?;
        for operand in step.operands {
            if usize::try_from(operand.0).map_err(|_| Refusal::Malformed)? >= index {
                return Err(Refusal::Malformed);
            }
        }
        let op = &graph.nodes.get(index).ok_or(Refusal::Malformed)?.op;
        match (step.rule, op) {
            (Rule::FoldAddConstants, &crate::ir::Op::I64Add(first, second)) => {
                if step.operands != [first, second] {
                    return Err(Refusal::Malformed);
                }
                let first_bits = checked_bits(graph, first, step.first_bits)?;
                let second_bits = checked_bits(graph, second, step.second_bits)?;
                if step.claimed != wrapping_add(first_bits, second_bits) {
                    return Err(Refusal::ClaimMismatch);
                }
            }
            (Rule::NeutralAddLeft, &crate::ir::Op::I64Add(first, second)) => {
                if step.operands != [first, second] {
                    return Err(Refusal::Malformed);
                }
                // Canonical identity step: the graph's own constant zero is the operand the
                // rule cancels, both recorded bit fields hold it, and the claim is the
                // cancelled identity.
                if !identity_bits_are_canonical(
                    graph,
                    first,
                    [step.first_bits, step.second_bits],
                    0,
                ) {
                    return Err(Refusal::Malformed);
                }
                if step.claimed != 0 {
                    return Err(Refusal::Malformed);
                }
            }
            (Rule::NeutralMulRight, &crate::ir::Op::I64Mul(first, second)) => {
                if step.operands != [first, second] {
                    return Err(Refusal::Malformed);
                }
                // Canonical identity step: the graph's own constant one is the operand the
                // rule cancels, both recorded bit fields hold it, and the claim is the
                // cancelled identity.
                if !identity_bits_are_canonical(
                    graph,
                    second,
                    [step.first_bits, step.second_bits],
                    1,
                ) {
                    return Err(Refusal::Malformed);
                }
                if step.claimed != 0 {
                    return Err(Refusal::Malformed);
                }
            }
            (_, _) => return Err(Refusal::Malformed),
        }
    }
    if replay(graph, derivation)? != *result {
        return Err(Refusal::ResultMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::identity::IdentityScheme;

    /// A deterministic test scheme: digests every part in order under a fixed tag.
    struct TestIds;

    impl crate::identity::IdentityHasher for TestHasher {
        fn update(&mut self, bytes: &[u8]) {
            for &byte in bytes {
                self.0 = self.0.wrapping_mul(31).wrapping_add(u64::from(byte));
            }
        }

        fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
            let mut digest = [0u8; 32];
            digest[..8].copy_from_slice(&self.0.to_le_bytes());
            digest
        }
    }

    struct TestHasher(u64);

    impl IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "test"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher(0))
        }
    }

    fn sample() -> crate::ir::Graph {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "cert-sample"),
        );
        let three = builder.i64_const(3);
        let five = builder.i64_const(5);
        let sum = builder.op(crate::ir::Op::I64Add(three, five));
        let one = builder.i64_const(1);
        let scaled = builder.op(crate::ir::Op::I64Mul(sum, one));
        builder.output(scaled);
        builder.finish()
    }

    #[test]
    fn emitted_fold_is_checked_and_endpoints_bind() {
        let graph = sample();
        let scheme = TestIds;
        let (derivation, result) = super::emit(&scheme, &graph);
        assert_eq!(derivation.revision, super::CERT_REVISION);
        assert_eq!(derivation.model, super::MODEL_REVISION);
        assert!(derivation.steps.len() <= super::MAX_STEPS);
        assert!(super::check(&scheme, &graph, &result, &derivation).is_ok());
        assert_ne!(
            derivation.left, derivation.right,
            "the fold changes the graph digest"
        );
    }

    #[test]
    fn mutated_certificates_are_refused() {
        let graph = sample();
        let scheme = TestIds;
        let (derivation, result) = super::emit(&scheme, &graph);
        let fold = derivation
            .steps
            .iter()
            .copied()
            .find(|step| step.rule == super::Rule::FoldAddConstants)
            .expect("the sample folds one addition");

        let mut swapped = derivation.clone();
        swapped.steps[0].operands = [fold.operands[1], fold.operands[0]];
        assert_eq!(
            super::check(&scheme, &graph, &result, &swapped),
            Err(super::Refusal::Malformed)
        );

        let mut claimed = derivation.clone();
        claimed.steps[0].claimed = fold.claimed.wrapping_add(1);
        assert_eq!(
            super::check(&scheme, &graph, &result, &claimed),
            Err(super::Refusal::ClaimMismatch)
        );

        let mut bits = derivation.clone();
        bits.steps[0].first_bits = fold.first_bits.wrapping_add(1);
        assert_eq!(
            super::check(&scheme, &graph, &result, &bits),
            Err(super::Refusal::ConstantMismatch)
        );

        let mut renamed = derivation.clone();
        renamed.steps[0].rule = super::Rule::NeutralAddLeft;
        assert_eq!(
            super::check(&scheme, &graph, &result, &renamed),
            Err(super::Refusal::Malformed)
        );

        let mut stale = derivation.clone();
        stale.revision = alloc::string::String::from(super::CHAINED_REVISION);
        assert_eq!(
            super::check(&scheme, &graph, &result, &stale),
            Err(super::Refusal::StaleRevision)
        );

        let mut stale_model = derivation.clone();
        stale_model.model = alloc::string::String::from("ngraph-sem/1");
        assert_eq!(
            super::check(&scheme, &graph, &result, &stale_model),
            Err(super::Refusal::StaleModel)
        );

        let mut unrelated = derivation.clone();
        unrelated.right = derivation.left;
        assert_eq!(
            super::check(&scheme, &graph, &result, &unrelated),
            Err(super::Refusal::EndpointMismatch)
        );

        let mut exhausted = derivation.clone();
        exhausted.steps = core::iter::repeat_n(fold, super::MAX_STEPS + 1).collect();
        assert_eq!(
            super::check(&scheme, &graph, &result, &exhausted),
            Err(super::Refusal::TooManySteps)
        );

        let mut reordered = derivation.clone();
        reordered.steps = alloc::vec![fold, fold];
        assert_eq!(
            super::check(&scheme, &graph, &result, &reordered),
            Err(super::Refusal::Order)
        );
    }

    /// The emitted certificate with its `right` digest renamed to a substituted result, as a
    /// dishonest producer would re-digest it: the steps stay valid for the original graph.
    fn redigested(derivation: &super::Derivation, result: &crate::ir::Graph) -> super::Derivation {
        let mut renamed = derivation.clone();
        renamed.right = result.logical_id(&TestIds);
        renamed
    }

    #[test]
    fn a_redigested_substituted_result_is_refused() {
        let graph = sample();
        let scheme = TestIds;
        let (derivation, result) = super::emit(&scheme, &graph);
        let fold = derivation
            .steps
            .iter()
            .copied()
            .find(|step| step.rule == super::Rule::FoldAddConstants)
            .expect("the sample folds one addition");
        let folded = usize::try_from(fold.node.0).expect("node index fits");
        let consumer = result
            .nodes
            .iter()
            .position(|node| matches!(node.op, crate::ir::Op::I64Mul(_, _)))
            .expect("the sample scales the sum");

        let mut wrong_constant = result.clone();
        wrong_constant.nodes[folded].op = crate::ir::Op::Const(crate::ir::Literal::I64(
            fold.claimed.wrapping_add(1).cast_signed(),
        ));
        let mut broken_sharing = result.clone();
        broken_sharing.nodes[consumer].op =
            crate::ir::Op::I64Mul(fold.operands[0], crate::ir::NodeId(3));
        let mut swapped_operands = result.clone();
        swapped_operands.nodes[consumer].op =
            crate::ir::Op::I64Mul(crate::ir::NodeId(3), fold.node);
        let mut changed_output = result.clone();
        changed_output.outputs = alloc::vec![fold.node];
        let mut repeated_output = result.clone();
        repeated_output.outputs.push(result.outputs[0]);
        let unrelated = {
            let mut builder = crate::ir::GraphBuilder::new(
                crate::ir::SubProfile::I64,
                crate::ir::Subject::authored(&scheme, "cert-sample"),
            );
            let eight = builder.i64_const(8);
            builder.output(eight);
            builder.finish()
        };

        for substituted in [
            &wrong_constant,
            &broken_sharing,
            &swapped_operands,
            &changed_output,
            &repeated_output,
            &unrelated,
        ] {
            assert_ne!(
                substituted, &result,
                "each control substitutes a different result"
            );
            assert_eq!(
                super::check(
                    &scheme,
                    &graph,
                    substituted,
                    &redigested(&derivation, substituted)
                ),
                Err(super::Refusal::ResultMismatch)
            );
        }
        assert_eq!(
            super::check(&scheme, &graph, &result, &redigested(&derivation, &result)),
            Ok(())
        );
    }

    /// A derivation over `steps` whose endpoints are both `graph`: identity steps are
    /// recorded without changing it.
    fn identity_derivation(
        graph: &crate::ir::Graph,
        steps: alloc::vec::Vec<super::Step>,
    ) -> super::Derivation {
        let scheme = TestIds;
        super::Derivation {
            revision: alloc::string::String::from(super::CERT_REVISION),
            model: alloc::string::String::from(super::MODEL_REVISION),
            steps,
            left: graph.logical_id(&scheme),
            right: graph.logical_id(&scheme),
        }
    }

    /// `zero + x` with `x` a program input: the canonical `neutral-add-left` premise.
    fn neutral_add_left() -> (
        crate::ir::Graph,
        crate::ir::NodeId,
        crate::ir::NodeId,
        crate::ir::NodeId,
    ) {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "cert-neutral-add-left"),
        );
        let zero = builder.i64_const(0);
        let input = builder.input(crate::ir::ScalarType::I64);
        let sum = builder.op(crate::ir::Op::I64Add(zero, input));
        builder.output(sum);
        (builder.finish(), zero, input, sum)
    }

    /// `x * 1` with `x` a program input: the canonical `neutral-mul-right` premise.
    fn neutral_mul_right() -> (
        crate::ir::Graph,
        crate::ir::NodeId,
        crate::ir::NodeId,
        crate::ir::NodeId,
    ) {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "cert-neutral-mul-right"),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        let one = builder.i64_const(1);
        let product = builder.op(crate::ir::Op::I64Mul(input, one));
        builder.output(product);
        (builder.finish(), one, input, product)
    }

    /// `x + 0` with `x` a program input: the operands of `neutral-add-left` in the
    /// other order, so its premise does not hold. Returns the graph and a premise-shaped
    /// step over it.
    fn swapped_sum() -> (crate::ir::Graph, super::Step) {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "cert-neutral-add-left-swapped"),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        let zero = builder.i64_const(0);
        let sum = builder.op(crate::ir::Op::I64Add(input, zero));
        builder.output(sum);
        let step = super::Step {
            rule: super::Rule::NeutralAddLeft,
            node: sum,
            operands: [input, zero],
            first_bits: 0,
            second_bits: 0,
            claimed: 0,
        };
        (builder.finish(), step)
    }

    /// `1 * x` with `x` a program input: the operands of `neutral-mul-right` in the
    /// other order, so its premise does not hold. Returns the graph and a premise-shaped
    /// step over it.
    fn swapped_mul() -> (crate::ir::Graph, super::Step) {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "cert-neutral-mul-right-swapped"),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        let one = builder.i64_const(1);
        let product = builder.op(crate::ir::Op::I64Mul(one, input));
        builder.output(product);
        let step = super::Step {
            rule: super::Rule::NeutralMulRight,
            node: product,
            operands: [one, input],
            first_bits: 1,
            second_bits: 1,
            claimed: 0,
        };
        (builder.finish(), step)
    }

    #[test]
    fn a_canonical_neutral_add_left_step_is_accepted_and_every_field_is_bound() {
        let (graph, zero, input, sum) = neutral_add_left();
        let scheme = TestIds;
        let canonical = super::Step {
            rule: super::Rule::NeutralAddLeft,
            node: sum,
            operands: [zero, input],
            first_bits: 0,
            second_bits: 0,
            claimed: 0,
        };
        assert_eq!(
            super::check(
                &scheme,
                &graph,
                &graph,
                &identity_derivation(&graph, alloc::vec![canonical])
            ),
            Ok(()),
            "the canonical `0 + x` step is accepted"
        );
        for (field, mutant) in [
            (
                "first_bits",
                super::Step {
                    first_bits: 1,
                    ..canonical
                },
            ),
            (
                "second_bits",
                super::Step {
                    second_bits: 1,
                    ..canonical
                },
            ),
            (
                "claimed",
                super::Step {
                    claimed: 1,
                    ..canonical
                },
            ),
        ] {
            assert_eq!(
                super::check(
                    &scheme,
                    &graph,
                    &graph,
                    &identity_derivation(&graph, alloc::vec![mutant])
                ),
                Err(super::Refusal::Malformed),
                "a mutated {field} is not a canonical identity premise"
            );
        }
        // The rule consumes its first operand only when the graph spells it as constant zero.
        let (swapped, swapped_step) = swapped_sum();
        assert_eq!(
            super::check(
                &scheme,
                &swapped,
                &swapped,
                &identity_derivation(&swapped, alloc::vec![swapped_step])
            ),
            Err(super::Refusal::Malformed),
            "`x + 0` is not the `neutral-add-left` premise"
        );
    }

    #[test]
    fn a_canonical_neutral_mul_right_step_is_accepted_and_every_field_is_bound() {
        let (graph, one, input, product) = neutral_mul_right();
        let scheme = TestIds;
        let canonical = super::Step {
            rule: super::Rule::NeutralMulRight,
            node: product,
            operands: [input, one],
            first_bits: 1,
            second_bits: 1,
            claimed: 0,
        };
        assert_eq!(
            super::check(
                &scheme,
                &graph,
                &graph,
                &identity_derivation(&graph, alloc::vec![canonical])
            ),
            Ok(()),
            "the canonical `x * 1` step is accepted"
        );
        for (field, mutant) in [
            (
                "first_bits",
                super::Step {
                    first_bits: 0,
                    ..canonical
                },
            ),
            (
                "second_bits",
                super::Step {
                    second_bits: 0,
                    ..canonical
                },
            ),
            (
                "claimed",
                super::Step {
                    claimed: u64::MAX,
                    ..canonical
                },
            ),
        ] {
            assert_eq!(
                super::check(
                    &scheme,
                    &graph,
                    &graph,
                    &identity_derivation(&graph, alloc::vec![mutant])
                ),
                Err(super::Refusal::Malformed),
                "a mutated {field} is not a canonical identity premise"
            );
        }
        // The rule consumes its second operand only when the graph spells it as constant one.
        let (swapped, swapped_step) = swapped_mul();
        assert_eq!(
            super::check(
                &scheme,
                &swapped,
                &swapped,
                &identity_derivation(&swapped, alloc::vec![swapped_step])
            ),
            Err(super::Refusal::Malformed),
            "`1 * x` is not the `neutral-mul-right` premise"
        );
    }

    #[test]
    fn the_emitted_identity_step_is_canonical_and_accepted() {
        let (graph, one, input, product) = neutral_mul_right();
        let scheme = TestIds;
        let (derivation, result) = super::emit(&scheme, &graph);
        assert_eq!(
            derivation.steps,
            alloc::vec![super::Step {
                rule: super::Rule::NeutralMulRight,
                node: product,
                operands: [input, one],
                first_bits: 1,
                second_bits: 1,
                claimed: 0,
            }],
            "the emitter records the canonical `x * 1` premise"
        );
        assert_eq!(
            result, graph,
            "a recorded identity does not change the graph"
        );
        assert_eq!(super::check(&scheme, &graph, &result, &derivation), Ok(()));
    }

    /// The tokens of `wire`, so a control can replace exactly one of them.
    fn tokens(wire: &str) -> alloc::vec::Vec<alloc::string::String> {
        wire.split(' ').map(alloc::string::String::from).collect()
    }

    fn join(tokens: &[alloc::string::String]) -> alloc::string::String {
        tokens.join(" ")
    }

    #[test]
    fn the_wire_envelope_round_trips_a_derivation() {
        let graph = sample();
        let scheme = TestIds;
        let (derivation, result) = super::emit(&scheme, &graph);
        let wire = super::encode(&derivation);
        let envelope = super::decode(&wire).expect("the emitted envelope decodes");
        assert_eq!(envelope.revision, derivation.revision);
        assert_eq!(envelope.model, derivation.model);
        assert_eq!(envelope.steps, derivation.steps);
        assert_eq!(
            wire.split(' ').count(),
            super::ENVELOPE_TOKENS + super::STEP_TOKENS * derivation.steps.len(),
            "one token per envelope field and exactly seven per step"
        );
        let bound = envelope.bind(derivation.left, derivation.right);
        assert_eq!(
            bound, derivation,
            "binding the endpoints recovers it exactly"
        );
        assert_eq!(super::encode(&bound), wire, "re-encoding is byte-identical");
        assert_eq!(super::check(&scheme, &graph, &result, &bound), Ok(()));
    }

    #[test]
    fn a_stale_revision_or_model_token_is_refused_by_the_codec() {
        let graph = sample();
        let scheme = TestIds;
        let (derivation, _) = super::emit(&scheme, &graph);
        let wire = super::encode(&derivation);
        let revision = wire.replacen(super::CERT_REVISION, super::CHAINED_REVISION, 1);
        assert_eq!(
            super::decode(&revision),
            Err(super::Refusal::StaleRevision),
            "a stale format token is refused before any step"
        );
        let model = wire.replacen(super::MODEL_REVISION, "ngraph-sem/1", 1);
        assert_eq!(
            super::decode(&model),
            Err(super::Refusal::StaleModel),
            "a stale semantic model token is refused before any step"
        );
    }

    #[test]
    fn the_chainless_v1_profile_remains_decodable_and_checkable() {
        let graph = sample();
        let scheme = TestIds;
        let (mut derivation, result) = super::emit(&scheme, &graph);
        derivation.revision = alloc::string::String::from(super::LEGACY_CHAINLESS_REVISION);
        let wire = super::encode(&derivation);
        let envelope = super::decode(&wire).expect("the readable v1 envelope decodes");
        let decoded = envelope.bind(derivation.left, derivation.right);
        assert_eq!(decoded, derivation);
        assert_eq!(super::check(&scheme, &graph, &result, &decoded), Ok(()));
    }

    #[test]
    fn malformed_envelopes_are_refused() {
        let graph = sample();
        let scheme = TestIds;
        let (derivation, _) = super::emit(&scheme, &graph);
        let wire = super::encode(&derivation);
        let fields = tokens(&wire);
        assert_eq!(
            super::decode(""),
            Err(super::Refusal::Malformed),
            "an empty envelope carries no revision"
        );
        assert_eq!(
            super::decode(super::CERT_REVISION),
            Err(super::Refusal::Malformed),
            "a revision alone carries no model token"
        );
        // A declared count over the bound is refused before the steps are allocated.
        let over_bound = [
            alloc::string::String::from(super::CERT_REVISION),
            alloc::string::String::from(super::MODEL_REVISION),
            alloc::format!("{}", super::MAX_STEPS + 1),
        ];
        assert_eq!(
            super::decode(&join(&over_bound)),
            Err(super::Refusal::TooManySteps),
            "the over-bound count is refused with no step token present"
        );
        let mut non_decimal_count = fields.clone();
        non_decimal_count[2] = alloc::string::String::from("two");
        assert_eq!(
            super::decode(&join(&non_decimal_count)),
            Err(super::Refusal::Malformed)
        );
        let truncated = &fields[..fields.len() - 1];
        assert_eq!(
            super::decode(&join(truncated)),
            Err(super::Refusal::Malformed),
            "a truncated final step is refused"
        );
        let dropped = &fields[..fields.len() - super::STEP_TOKENS];
        assert_eq!(
            super::decode(&join(dropped)),
            Err(super::Refusal::Malformed),
            "dropping a chunk disagrees with the declared count"
        );
        let mut extra = fields.clone();
        extra.push(alloc::string::String::from("0"));
        assert_eq!(
            super::decode(&join(&extra)),
            Err(super::Refusal::Malformed),
            "a token after the declared steps is refused"
        );
        let mut unknown_rule = fields.clone();
        unknown_rule[super::ENVELOPE_TOKENS] = alloc::string::String::from("3");
        assert_eq!(
            super::decode(&join(&unknown_rule)),
            Err(super::Refusal::Malformed),
            "a rule token outside the modelled set is refused"
        );
        let mut non_decimal_bits = fields.clone();
        non_decimal_bits[super::ENVELOPE_TOKENS + 4] = alloc::string::String::from("5x");
        assert_eq!(
            super::decode(&join(&non_decimal_bits)),
            Err(super::Refusal::Malformed)
        );
        let mut negative = fields.clone();
        negative[super::ENVELOPE_TOKENS + 4] = alloc::string::String::from("-5");
        assert_eq!(
            super::decode(&join(&negative)),
            Err(super::Refusal::Malformed),
            "a signed token is not a decimal token"
        );
        let mut out_of_range = fields.clone();
        out_of_range[super::ENVELOPE_TOKENS + 4] =
            alloc::string::String::from("18446744073709551616");
        assert_eq!(
            super::decode(&join(&out_of_range)),
            Err(super::Refusal::Malformed),
            "a token at 2^64 is outside the 64-bit range"
        );
        // The unmodified envelope still decodes, so the controls above are the mutations.
        assert!(super::decode(&wire).is_ok());
    }

    /// A graph whose fold node reads two constants defined *after* it: the forward
    /// reference admission refuses.
    fn forward_reference() -> (crate::ir::Graph, super::Step) {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "cert-forward-reference"),
        );
        let folded = builder.op(crate::ir::Op::I64Add(
            crate::ir::NodeId(1),
            crate::ir::NodeId(2),
        ));
        let three = builder.i64_const(3);
        let five = builder.i64_const(5);
        builder.output(folded);
        builder.output(three);
        builder.output(five);
        let step = super::Step {
            rule: super::Rule::FoldAddConstants,
            node: folded,
            operands: [three, five],
            first_bits: 3,
            second_bits: 5,
            claimed: 8,
        };
        (builder.finish(), step)
    }

    #[test]
    fn a_step_may_not_premise_on_a_later_node() {
        let (graph, step) = forward_reference();
        let scheme = TestIds;
        let mut result = graph.clone();
        result.nodes[usize::try_from(step.node.0).expect("node index fits")].op =
            crate::ir::Op::Const(crate::ir::Literal::I64(8));
        assert_eq!(
            crate::check::check(&scheme, graph.clone())
                .err()
                .map(|diagnostic| diagnostic.code),
            Some(crate::diag::Code::GraphMalformed(
                crate::diag::Malformed::ForwardReference
            )),
            "admission refuses the forward-reference endpoint"
        );
        let derivation = super::Derivation {
            revision: alloc::string::String::from(super::CERT_REVISION),
            model: alloc::string::String::from(super::MODEL_REVISION),
            steps: alloc::vec![step],
            left: graph.logical_id(&scheme),
            right: result.logical_id(&scheme),
        };
        assert_eq!(
            super::check(&scheme, &graph, &result, &derivation),
            Err(super::Refusal::Malformed),
            "the checker refuses a premise on a node at or after the rewritten node"
        );
    }
}
