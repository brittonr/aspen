# Following an artifact through the registry

Mode: Walkthrough

This source-only walkthrough follows the checked-in `dependency_closure_impact_missing_dependencies_and_rebuild_work` fixture, rather than inventing a deployment. It shows where an artifact becomes locally addressable, what a closure observation means, and where the fixture stops. The fixture and implementation were inspected, not executed for this guide. Return to the [Handbook](../README.md) for other practical routes; the [canonical identity companion](../../technical/foundations/canonical-identity-model.md) explains the underlying representation model.

## 1. Establish the fixture's inputs

Open the [registry dependency fixture](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs) and its [input constructor](../../../src/artifacts/parts/mod/tests/m000/p002/body.rs). The fixture creates an isolated temporary registry, installs a `schema` artifact labelled `base`, and installs a `steel` artifact labelled `dependent` whose sole dependency is the returned base artifact reference.

`test_input` constructs a Preserves `payload` record containing the label. It also supplies schema, policy, evidence, installer, and capability references produced by hashing `artifact-test-ref` records. These are deterministic fixture inputs, not demonstrated policy decisions or live credentials. Copying their construction into an operational importer would not establish authority.

The observable input boundary is an `ArtifactInstallInput`: a typed Rust request containing a Preserves value and explicit references. It is not a Steel source compiler invocation. The artifact's `steel` kind does not prove its payload has been compiled, executed, or behaviorally validated.

## 2. Prepare the payload and canonical record

Follow `install_artifact` into [payload preparation and installation](../../../src/artifacts/parts/mod/p006/body.rs). The path opens an `ArtifactStoreRoot`, validates input reference shape, and canonicalizes the supplied Preserves payload. Canonical byte length determines representation: at most 4096 bytes uses `Inline`; larger values go through chunk storage and use `ContentRef` with a manifest reference.

This threshold belongs to this registry implementation, not every Molten envelope. The fixture's small label payload follows the inline path. Its value reference identifies the payload; the eventual artifact reference identifies the enclosing `artifact-v1` record. They answer different questions.

The [record constructor and parser](../../../src/artifacts/parts/mod/p001/body.rs) bind kind, domain, payload descriptor, schemas, dependencies, effect manifest, policy, evidence, and check markers. Parsing computes the artifact reference from the canonical record. Neither Rust field layout nor a human-readable name supplies identity.

## 3. Observe installation, not execution admission

Installation obtains an identity receipt and checks whether the explicitly listed dependencies have registry entries. Because `base` was installed first, `dependent` can pass this local availability check. The [commit helper](../../../src/artifacts/parts/mod/p001/body.rs) stores the artifact and install receipt in one Redb transaction when dependencies are present. The [storage helper](../../../src/artifacts/parts/mod/p011/body.rs) stores inline payload bytes and derived dependency indexes alongside the artifact entry.

Record the distinction between the returned `artifact_ref`, `decision`, `missing_dependencies`, and receipt value. A returned artifact record is not by itself proof it was committed: denied installs also return a proposed record. Nor does `pass` mean the executable is admitted to a live handler. Current authority, provenance, resource, policy, and lifecycle decisions belong to the consuming boundary.

## 4. Follow the dependency direction

The fixture asks for the closure rooted at `dependent`. Its assertions require both `dependent` and `base` in `closure_refs`, with no missing references. It then asks for impact rooted at `base`; that result includes `base` and `dependent`. Closure follows dependencies forward; impact follows dependents backward. Confusing the two can omit code required for use or misidentify consumers affected by a change.

The [closure implementation](../../../src/artifacts/parts/mod/p008/body.rs) hashes a closure value and persists a `dependency-closure` receipt. Its traversal uses the artifact and dependency index tables. It is not an independent payload-integrity scan and does not traverse every schema, policy, or evidence reference as an import. Preserve that scope when attaching the receipt to a review.

## 5. Inspect the deliberate missing-dependency branch

Next the fixture constructs a `steel` artifact labelled `bad`, depending on a generated reference that was not installed. It expects an ordinary result with `decision` equal to `deny` and that reference in `missing_dependencies`. The commit helper records the denial receipt but does not store the proposed artifact.

This is the worked failure boundary: a well-formed content reference can identify unavailable content. Do not repair it by changing the dependency to a convenient local artifact or treating the receipt as permission to import. A real receiver must select, fetch, verify, and admit the exact missing content under its own policy, as specified by [reference execution](../../unison-reference-execution.md).

## 6. Stop at the fixture's evidence boundary

The final fixture step rebuilds derived indexes and checks an artifact count. That is maintenance behavior, not a recommendation to rebuild during diagnosis. No remote fetch, handler execution, atomic live-binding publication, restart durability demonstration, or release-readiness claim follows from this fixture.

For live binding, the [governing contract](../../live-artifact-binding-and-semantic-effects.md) adds verified closure loading, compatibility and migration checks, admission, a checked successor plan, and shell-owned atomic publication. The registry walkthrough supplies useful local observations; it does not complete those stages.

## Sources

- [Handbook](../README.md)
- [Canonical identity companion](../../technical/foundations/canonical-identity-model.md)
- [Reference execution contract](../../unison-reference-execution.md)
- [Live binding and semantic effects](../../live-artifact-binding-and-semantic-effects.md)
- [Checked-in dependency fixture](../../../src/artifacts/parts/mod/tests/m000/p001/body.rs)
- [Fixture input construction](../../../src/artifacts/parts/mod/tests/m000/p002/body.rs)
- [Installation and payload boundary](../../../src/artifacts/parts/mod/p006/body.rs)
- [Canonical record and commit helpers](../../../src/artifacts/parts/mod/p001/body.rs)
- [Index storage and closure traversal](../../../src/artifacts/parts/mod/p011/body.rs)
