# Reviewing artifact import boundaries

Mode: Review checklist

Use this checklist before accepting a change that imports artifacts, consumes a registry closure, or connects an artifact to live binding. The review output should identify the exact subject, evidence for each applicable gate, and unresolved blockers. A row is not satisfied by a helper name, a nonempty reference list, or a successful process exit.

This is a source-backed review aid; no commands or tests were executed for it. The [Handbook](../README.md) links adjacent workflows, while the [canonical identity companion](../../technical/foundations/canonical-identity-model.md) explains representation boundaries. Requirements remain with the [reference-execution contract](../../unison-reference-execution.md) and [live-binding contract](../../live-artifact-binding-and-semantic-effects.md).

## Subject and representation acceptance

- [ ] Does the review identify the exact root artifact reference and the operation that will consume it? Attach the original canonical request or artifact, not just a friendly name or path. If a name was used, retain its resolution evidence and exact target.
- [ ] Are payload identity and artifact-record identity kept separate? Show which canonical value or raw byte sequence each digest covers. A content-reference string alone does not establish its encoding contract.
- [ ] Does the loading path compare the requested identity with the actual loaded record? Point to the [artifact reader](../../../src/artifacts/parts/mod/p001/body.rs), or the consumer's equivalent measured boundary, rather than a syntax-only validator.
- [ ] Is the payload's own loading path checked? Inline data and chunk-manifest data take different branches. A verified artifact descriptor does not prove its referenced payload is present.
- [ ] Are kind and schema interpretations admitted by the consumer? The registry's `steel` or `wasm` label is not evidence of compilation, execution, or behavioral correctness.

For acceptance, retain the measured input and outcome at the relevant boundary. Do not use Rust debug output or structure layout as an identity definition; canonical Preserves plus BLAKE3 owns that boundary.

## Closure and import selection acceptance

- [ ] Does the importer distinguish `dependency_refs` from schema, effect, policy, and evidence refs? Attach the required member list for each owning loader. The ordinary [registry traversal](../../../src/artifacts/parts/mod/p011/body.rs) follows indexed imports, not every semantic edge.
- [ ] Is an empty missing set described accurately as local indexed availability? Require separate integrity and admission observations where the consumer needs them.
- [ ] Does the caller inspect `decision` and diagnostics rather than only return status? Both installation and closure CLI operations can return normally while describing denial or missing dependencies.
- [ ] Are receiver-selected missing refs separated from unsolicited sender-pushed extras? The receiver must choose fetches and verify/admit their results; transport possession is not import authority.
- [ ] Are traversal, receipt, and diagnostic bounds handled as explicit failures? Demonstrate that a bound error cannot become a silently truncated complete closure. Do not assume the maximum list constant guarantees every graph of that cardinality fits all intermediate bounds.
- [ ] Are mutations visible in the proposed workflow? Opening registry tables may initialize state, and closure inspection stores a receipt. Identify the approved inspection registry and fresh evidence destinations.

An acceptable evidence packet preserves the roots, available members, missing members, decision, and limitations of the observation. It does not convert an index digest into a full byte-verification assertion.

## Current authority and shell ownership acceptance

- [ ] What actually validates policy, capability, provenance, resources, and lifecycle for this use? The [install validator](../../../src/artifacts/parts/mod/p016/body.rs) checks reference shape and requires a capability ref; it is not a current credential verifier.
- [ ] Has the review excluded local CLI-generated references from live-authority claims? The [CLI](../../../src/cli/core/artifact/ops.rs) constructs policy, evidence, installer, and capability refs through `local_ref`. The [helper](../../../src/cli/core/artifact/io.rs) hashes an `artifact-cli-ref` record; that construction is not release or authorization evidence.
- [ ] Are pure facts distinguished from shell observations? `ProductGateFacts` booleans must have producers and evidence. The pure planner does not load bytes, query authority, publish a revision, or perform an effect.
- [ ] Does any denial preserve the prior current binding? Link the owning publication operation and its observed outcome. A plan whose `publication_authorized` field is false must not be relabelled a published transition.

## Binding and semantic compatibility acceptance

- [ ] Does one explicit request, turn, callback pass, job, or protocol session resolve from one immutable snapshot? Preserve the chosen artifact and normalized closure for that unit.
- [ ] Is there exactly one supplied closure matching the resolved target? The [resolver](../../../crates/molten-core/src/live_binding/binding.rs) rejects absent or duplicate matches, but does not independently prove the supplied closure complete.
- [ ] Are nested late-bound operations explicit and separately evidenced? Do not infer that an outer resolution permits ambient nested lookup.
- [ ] Do strict semantic operation identities match across the required surfaces? If not, attach the exact directional compatibility artifact, its context, and current Molten admission evidence. Replay-only compatibility cannot authorize live execution.
- [ ] Are retirement and retention conclusions separate? A retirement report cannot authorize deletion, and old work or rollback evidence may still require the artifact.

## Worked review rejection and acceptance boundary

Consider a proposed importer that runs local install, sees a zero exit status, and immediately updates a consumer target. The [missing-dependency fixture](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs) supplies a concrete rejection reason: installation may return `deny` with a proposed record and a persisted receipt while not storing the artifact. The review fails at materialization before compatibility or publication is considered.

Accept only after the importer distinguishes the denial, obtains the exact receiver-selected dependency evidence, verifies and admits the target, and reaches the separately owned binding or execution boundary. This does not require inventing a new retry loop or bypass. If the available evidence ends at local registry installation, state that scope explicitly and withhold claims about live execution, publication, durability, or production readiness.

## Sources

- [Handbook](../README.md)
- [Canonical identity companion](../../technical/foundations/canonical-identity-model.md)
- [Reference execution contract](../../unison-reference-execution.md)
- [Live binding and semantic effects](../../live-artifact-binding-and-semantic-effects.md)
- [Registry installation](../../../src/artifacts/parts/mod/p006/body.rs)
- [Registry validation](../../../src/artifacts/parts/mod/p016/body.rs)
- [CLI-generated reference helper](../../../src/cli/core/artifact/io.rs)
- [Dependency denial fixture](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs)
- [Pure binding checks](../../../crates/molten-core/src/live_binding/binding.rs)
- [Semantic compatibility admission](../../../crates/molten-core/src/live_binding/semantic.rs)
