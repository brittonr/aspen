# Evidence Gates

## Purpose

Evidence gates define how Molten receipts are attributed, verified, chained, and kept separate from subsystem authority.

## Requirements

### Requirement: Signed envelopes preserve subject hashes
r[molten.evidence.signed_receipts.envelope] A signed receipt envelope MUST wrap a canonical receipt without changing the subject receipt's canonical ref.

#### Scenario: Signing a gate receipt preserves gate receipt ref
- GIVEN a canonical `<gate-receipt-v1 ...>`
- WHEN the receipt is signed
- THEN the signed envelope names the same gate receipt ref
- AND the signature covers the canonical bytes of that receipt

### Requirement: Signature verification is fail-closed
r[molten.evidence.signed_receipts.verify] Evidence profiles that require signatures MUST reject missing, malformed, unauthorized, or stale signatures before accepting pass evidence.

#### Scenario: Wrong key purpose is rejected
- GIVEN a signed receipt using a key authorized only for diagnostics
- WHEN a production gate profile requires pass-evidence signing
- THEN the receipt is rejected

#### Scenario: Mismatched subject ref is rejected
- GIVEN a signed envelope whose subject ref does not hash to the embedded receipt
- WHEN verification runs
- THEN verification fails closed

### Requirement: Local signer fixtures
r[molten.evidence.signed_receipts.key_fixtures] Molten SHOULD provide deterministic local signer, key, purpose, and trust-root fixtures for harness and CLI verification without treating those fixtures as production trust.

#### Scenario: Local fixture signs pass evidence
- GIVEN a local development receipt and fixture signer settings
- WHEN signing runs in the harness or CLI
- THEN the signed envelope verifies under the matching fixture purpose, trust root, and key
- AND production profiles MAY still require non-fixture trust roots

### Requirement: Generic signed receipt CLI
r[molten.evidence.signed_receipts.cli] Molten MUST provide CLI commands to sign canonical receipts and verify signed receipt envelopes with configured purpose, trust root, key, signer, and subject constraints.

#### Scenario: CLI verifies configured signature policy
- GIVEN a signed receipt envelope
- WHEN the CLI verifies it with an expected signer, subject ref, purpose, trust root, and key
- THEN verification passes only when all configured signature policy fields match

### Requirement: Signed receipt test coverage
r[molten.evidence.signed_receipts.tests] Molten SHOULD cover wrong signer, wrong purpose, wrong key, mismatched subject ref, malformed envelope, unsupported algorithm, and missing trust-root failures in automated tests.

#### Scenario: Signature negative tests fail closed
- GIVEN malformed or policy-mismatched signed receipt envelopes
- WHEN verification runs
- THEN tests assert verification fails closed with diagnostics before the receipt is accepted

### Requirement: Receipt chains are explicit
r[molten.evidence.signed_receipts.chain] Signed receipt envelopes SHOULD name parent receipt refs when one receipt depends on another.

#### Scenario: Verify receipt chains to report gate receipt
- GIVEN a sealed repro verify receipt
- WHEN it is signed
- THEN the signed envelope names the embedded report gate receipt as a parent
- AND chain verification can reconstruct the gate-to-verify dependency

### Requirement: Operator receipt signing CLI
r[molten.evidence.signed_receipts.operator_receipts_cli] Molten MUST expose top-level receipt signing and signed receipt verification commands for canonical dogfood and release evidence artifacts.

#### Scenario: Operator signs and verifies a release receipt
- GIVEN a canonical dogfood or release receipt file
- WHEN an operator runs receipt signing and signed verification with a signer, purpose, trust root, key, and optional expected subject ref
- THEN Molten emits or verifies a `signed-receipt-v1` envelope that binds the subject ref, signer identity, purpose, trust root, parent refs, and canonical subject bytes

### Requirement: Signed release bundle members
r[molten.evidence.signed_receipts.release_bundle_members] Release evidence bundle verification MAY require signed Preserves member receipts and MUST deny the bundle review when required signatures are missing, malformed, signed by the wrong signer, scoped to the wrong purpose, or bound to a subject ref outside the bundle.

#### Scenario: Bundle verification requires signed member receipts
- GIVEN a release evidence bundle with dogfood report, release gate, Nix evidence, and Nix verify Preserves members
- WHEN verification is run with signed members required
- THEN every Preserves member ref has a verified signed envelope for the configured signer, purpose, trust root, and key before the bundle receipt can pass

#### Scenario: Wrong signer denies bundle review
- GIVEN a signed member envelope from a signer that is not configured for the release review profile
- WHEN release bundle verification requires signed member receipts
- THEN it emits a `release-evidence-bundle-verify-receipt-v1` with decision `deny` and diagnostics identifying the signer mismatch

### Requirement: Signed receipt keyring records
r[molten.evidence.signed_receipts.keyring_records] Molten MUST represent signed receipt verification keys and key revocations as canonical evidence artifacts with stable refs, signer identity, trust root, key id, generation or revocation reason, and evidence-only checks.

#### Scenario: Key import writes auditable evidence
- GIVEN a signer id, trust root, key id, and local fixture verification key
- WHEN an operator imports the key into the signed receipt keyring
- THEN Molten stores a canonical `signed-receipt-key-v1` artifact in the ledger
- AND the artifact binds the signer, trust root, key id, generation, predecessor ref, and evidence-only caveat

#### Scenario: Revocation writes immutable evidence
- GIVEN an imported signed receipt key
- WHEN an operator revokes the key
- THEN Molten stores a canonical `signed-receipt-key-revocation-v1` artifact naming the revoked key ref
- AND future keyring verification treats that key as ineligible without mutating the original key record

### Requirement: Signed receipt keyring CLI
r[molten.evidence.signed_receipts.keyring_cli] Molten MUST expose ledger-backed CLI commands to import, list, show, revoke, and rotate signed receipt verification keys.

#### Scenario: Operator rotates a signing key
- GIVEN a current signed receipt key record
- WHEN an operator rotates it with a new key id and key material
- THEN Molten stores a new key record with a predecessor ref
- AND Molten stores a revocation record for the old key that names the new key as its successor

### Requirement: Signed receipt keyring verification
r[molten.evidence.signed_receipts.keyring_verify] Signed receipt verification MAY resolve keys from a ledger keyring and MUST fail closed when the selected key is missing, ambiguous, stale, revoked, scoped to the wrong signer, scoped to the wrong trust root, or unable to verify the envelope signature.

#### Scenario: Revoked key denies signed receipt verification
- GIVEN a signed receipt envelope that verifies with an imported key
- AND a key revocation record for that key is present in the keyring ledger
- WHEN verification runs with that keyring
- THEN verification fails closed with diagnostics that identify the revocation

#### Scenario: Ambiguous keyring denies verification
- GIVEN multiple current unrevoked key records for the same signer and trust root
- WHEN verification runs without a key id or key ref disambiguator
- THEN verification fails closed and requires an explicit key id or key ref

### Requirement: Release bundle signed members use keyring policy
r[molten.evidence.signed_receipts.keyring_release_bundle] Release evidence bundle verification MAY require signed member receipts to verify through a ledger keyring and MUST deny the bundle review when a required member is signed by a missing, ambiguous, stale, revoked, wrong-signer, wrong-purpose, or wrong-trust-root key.

#### Scenario: Bundle verification uses current keyring key
- GIVEN a release evidence bundle with signed Preserves members
- AND a keyring ledger containing the current unrevoked signer key
- WHEN release bundle verification runs with `--require-signed-members` and keyring inputs
- THEN every signed member must verify through the current key before the bundle verify receipt can pass

### Requirement: Signed receipt keyring remains evidence only
r[molten.evidence.signed_receipts.keyring_evidence_only] Signed receipt key records, revocation records, and keyring verification decisions MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Current key does not grant release authority
- GIVEN a current unrevoked keyring key and passing signed receipt verification
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the keyring record or signed envelope as subsystem authority

### Requirement: Release promotion gate receipt
r[molten.evidence.release_promotion.receipt] Molten MUST emit a canonical `release-promotion-gate-receipt-v1` that aggregates release bundle verification, signed keyring currentness, source evidence, Octet evidence, Cairn evidence, diagnostics, and evidence-only caveats.

#### Scenario: Promotion receipt passes for complete evidence graph
- GIVEN a passing release evidence bundle verification receipt
- AND a current unrevoked signed receipt key selected from the keyring
- AND source, Octet, and Cairn evidence markers
- WHEN release promotion verification runs
- THEN Molten emits a `release-promotion-gate-receipt-v1` with decision `pass`
- AND the receipt binds the bundle verify ref, selected key ref, source evidence ref, Octet evidence ref, and Cairn evidence ref

### Requirement: Release promotion CLI
r[molten.evidence.release_promotion.cli] Molten MUST expose `molten dogfood release-promote` to create release promotion receipts from a realized dogfood output, bundle verification receipt, signed keyring ledger, and explicit source/Octet/Cairn evidence markers.

#### Scenario: CLI writes pass or deny receipt
- GIVEN promotion inputs that are complete or incomplete
- WHEN the CLI runs
- THEN it writes a canonical promotion receipt to the requested output path
- AND denial diagnostics are stored in the receipt rather than only in logs

### Requirement: Promotion binds signed keyring currentness
r[molten.evidence.release_promotion.keyring] Release promotion MUST fail closed when the selected signed receipt key is missing, ambiguous, stale, revoked, scoped to the wrong signer, or scoped to the wrong trust root.

#### Scenario: Revoked key denies promotion
- GIVEN a release bundle verification receipt that otherwise passes
- AND a keyring ledger containing a revocation record for the selected signed receipt key
- WHEN release promotion runs with that key selected
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the signed keyring currentness failure

### Requirement: Promotion binds bundle verification and output path
r[molten.evidence.release_promotion.bundle] Release promotion MUST bind the release bundle verification receipt and the realized output path ref and MUST deny promotion when the bundle verification receipt is not passing or was produced for a different output path.

#### Scenario: Stale bundle verification denies promotion
- GIVEN a bundle verification receipt for one output path
- WHEN promotion is run against a different realized output path
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the output path ref mismatch

### Requirement: Promotion binds source, Octet, and Cairn evidence markers
r[molten.evidence.release_promotion.source_gates] Release promotion MUST bind explicit source, Octet, and Cairn evidence markers as deterministic refs and MUST deny promotion when any required marker is missing.

#### Scenario: Missing source gate marker denies promotion
- GIVEN a passing bundle verification receipt and current signed key
- WHEN the source evidence marker is empty
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the missing source evidence marker

### Requirement: Signed release promotion receipt
r[molten.evidence.release_promotion.signed_receipt] Molten's dogfood release evidence MUST sign the final `release-promotion-gate-receipt-v1` with a distinct `release-promotion` signed receipt purpose.

#### Scenario: Promotion receipt is signed after promotion passes
- GIVEN a dogfood release output with a passing release promotion gate receipt
- WHEN the dogfood release check finishes
- THEN it emits a signed receipt envelope for `release-promotion-gate.preserves`
- AND the signed envelope uses purpose `release-promotion`

### Requirement: Signed promotion receipt keyring verification
r[molten.evidence.release_promotion.signed_receipt_verify] Molten's dogfood release evidence MUST verify the signed release promotion receipt through the signed receipt keyring and fail the check when verification does not pass.

#### Scenario: Keyring verifies signed promotion receipt
- GIVEN a signed promotion receipt envelope and the dogfood signed receipt keyring
- WHEN signed receipt verification runs with the selected key id, signer, trust root, and purpose
- THEN verification passes
- AND the verification log is preserved with the dogfood release output

### Requirement: Release promotion summary record
r[molten.evidence.release_promotion.summary_record] Molten MUST emit a canonical `release-promotion-summary-v1` artifact that summarizes promotion receipt, signed promotion envelope, selected keyring key, source evidence, Octet evidence, Cairn evidence, diagnostics, and evidence-only checks.

#### Scenario: Summary binds promotion readback refs
- GIVEN a realized dogfood release output with promotion evidence
- WHEN release promotion summary generation runs
- THEN the summary binds the promotion receipt ref, signed envelope ref, signed subject ref, signed key ref, source ref, Octet ref, and Cairn ref

### Requirement: Release promotion summary CLI
r[molten.evidence.release_promotion.summary_cli] Molten MUST expose `molten dogfood release-promotion-summary` to write promotion summaries from a realized dogfood output and signed receipt keyring.

#### Scenario: CLI writes summary artifact
- GIVEN a dogfood release output and signed receipt keyring
- WHEN the summary CLI runs
- THEN it writes `release-promotion-summary-v1` to the requested output path
- AND prints a status line containing the summary decision and refs

### Requirement: Signed promotion summary readback
r[molten.evidence.release_promotion.summary_signed_readback] Release promotion summaries MUST verify the signed promotion envelope through the selected keyring key and require the signed subject ref to match the promotion receipt ref.

#### Scenario: Signed promotion readback passes
- GIVEN a passing promotion receipt and signed promotion envelope over that receipt
- AND a current unrevoked signed receipt key
- WHEN summary generation runs
- THEN the summary decision is `pass`
- AND it binds the signed envelope and key refs

### Requirement: Promotion summary denies stale or missing readback
r[molten.evidence.release_promotion.summary_deny_readback] Release promotion summaries MUST emit deny evidence when the promotion receipt is missing, not passing, bound to another output path, unsigned, signed for another subject, or signed by a missing/stale/revoked key.

#### Scenario: Missing signed promotion denies summary
- GIVEN a passing promotion receipt
- AND no signed promotion envelope in the dogfood output
- WHEN summary generation runs
- THEN the summary decision is `deny`
- AND diagnostics identify signed promotion verification failure

### Requirement: Promotion summary remains evidence only
r[molten.evidence.release_promotion.summary_evidence_only] Release promotion summaries MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Summary does not replace subsystem gates
- GIVEN a passing release promotion summary
- WHEN a subsystem or release publisher requires its own authority or gate evidence
- THEN the summary MUST NOT be sufficient authority
- AND the subsystem MUST still require its own gate evidence

### Requirement: Signed promotion subject binding
r[molten.evidence.release_promotion.signed_subject_binding] Molten's dogfood release evidence MUST verify the signed release promotion receipt against the exact subject ref emitted by `molten dogfood release-promote`.

#### Scenario: Signed promotion subject matches emitted promotion receipt
- GIVEN a release promotion gate receipt emitted by dogfood release promotion
- AND a signed receipt envelope for release promotion
- WHEN signed promotion verification runs
- THEN verification requires the signed envelope subject ref to equal the emitted promotion receipt ref
- AND verification fails when the signed envelope subject ref differs

### Requirement: Signed promotion subject binding remains evidence only
r[molten.evidence.release_promotion.signed_subject_evidence_only] Signed promotion subject-ref binding MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Subject binding does not grant publication authority
- GIVEN a signed promotion receipt whose subject ref matches the emitted promotion receipt
- WHEN release publication or a subsystem operation requires authority
- THEN the matching subject binding MUST NOT be treated as sufficient authority
- AND the subsystem MUST still require its own gate evidence

### Requirement: Signed promotion receipt remains evidence only
r[molten.evidence.release_promotion.signed_receipt_evidence_only] Signed release promotion receipts MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Signed promotion is not release authority
- GIVEN a verified signed release promotion receipt
- WHEN a subsystem or release publisher requires its own authority or gate evidence
- THEN it MUST NOT treat the signed promotion receipt as sufficient authority
- AND it MUST still require the subsystem or publication gate evidence

### Requirement: Release promotion remains evidence only
r[molten.evidence.release_promotion.evidence_only] Release promotion receipts MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Promotion pass does not replace subsystem gates
- GIVEN a passing release promotion receipt
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the promotion receipt as subsystem authority

### Requirement: Signed evidence remains evidence only
r[molten.evidence.signed_receipts.evidence_only] Signed receipt envelopes and signed release bundle member checks MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Signature does not replace subsystem gates
- GIVEN a signed release evidence receipt passes verification
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the signed envelope as subsystem authority

### Requirement: Release export manifest
r[molten.evidence.release_export.manifest] Release evidence export manifests MUST bind a realized dogfood output path, the release promotion summary ref, deterministic member path/content refs, and evidence-only/no-authority checks in canonical Preserves.

#### Scenario: Manifest binds portable release evidence members
- GIVEN a realized dogfood output with pass promotion summary evidence
- WHEN an operator creates a release export manifest
- THEN the manifest records the output path ref, promotion summary ref, member refs, deterministic layout check, evidence-only check, and no-release-authority check

### Requirement: Release export archive
r[molten.evidence.release_export.archive] Release evidence archives MUST use deterministic member ordering and file metadata so the same manifest and member bytes produce a stable portable review artifact.

#### Scenario: Deterministic archive export
- GIVEN a release export manifest and its listed members
- WHEN an operator writes the release evidence archive
- THEN the archive contains the manifest and listed payload members with deterministic tar metadata and without using logs as primary evidence

### Requirement: Release export verification
r[molten.evidence.release_export.verify] Release export verification MUST recompute member refs from the archive and emit a pass/deny receipt instead of relying on archive command logs.

#### Scenario: Tampered export denies
- GIVEN a release evidence archive with a missing, extra, stale, or tampered payload member
- WHEN an operator verifies the archive
- THEN verification emits `release-export-verify-receipt-v1` with decision `deny` and diagnostics identifying the member binding failure

### Requirement: Release export dogfood
r[molten.evidence.release_export.dogfood] The Nix dogfood release check MUST emit a portable release evidence archive, manifest, and verification receipt while preserving the evidence-only boundary.

#### Scenario: Dogfood emits portable export evidence
- GIVEN the dogfood release flow has produced signed promotion and promotion summary evidence
- WHEN the Nix dogfood check completes
- THEN it emits `release-evidence.tar.zst`, `release-export-manifest.preserves`, and `release-export-verify.preserves` with pass verification and no release authority granted

### Requirement: Malformed release export archives deny with receipts
r[molten.evidence.release_export.malformed_archive_denies] Release export verification MUST emit a canonical deny receipt, rather than relying on process failure or logs, when an archive is structurally readable but missing its manifest.

#### Scenario: Missing manifest emits deny receipt
- GIVEN a release evidence archive without `release-export-manifest.preserves`
- WHEN an operator runs `molten dogfood release-export-verify`
- THEN Molten emits `release-export-verify-receipt-v1` with decision `deny`
- AND diagnostics identify the missing manifest

### Requirement: Release export member diagnostics
r[molten.evidence.release_export.member_diagnostics] Release export verification MUST diagnose duplicate archive paths, extra unlisted members, missing listed members, stale member refs, and tampered member bytes in the verification receipt.

#### Scenario: Archive member mismatch emits diagnostics
- GIVEN a release evidence archive with duplicate, extra, missing, stale, or tampered members
- WHEN release export verification runs
- THEN the verification receipt has decision `deny`
- AND diagnostics identify the archive member binding problem

### Requirement: Gate receipts bind executor execution receipts
r[molten.evidence.executor_resource_gate_receipts.aggregate_ref] Gate receipts MUST include a canonical aggregate ref for all executor execution receipts embedded in the validated report.

#### Scenario: Execution receipt aggregate is named
- GIVEN a report with Steel or Wasm execution receipt events
- WHEN a gate receipt is emitted
- THEN its artifact refs include `executor-execution-receipts`
- AND the ref is derived from the canonical sequence of execution receipt events

### Requirement: Gate receipts expose executor resource checks
r[molten.evidence.executor_resource_gate_receipts.checks] Gate receipts MUST explicitly include checks for executor resource and ABI bounds that are required for pass evidence.

#### Scenario: Steel resource bounds are visible at the gate
- GIVEN a report containing reviewed Steel execution receipts
- WHEN a gate receipt is parsed
- THEN the receipt includes `steel-resource-bounds`

#### Scenario: Wasm ABI bounds are visible at the gate
- GIVEN a report containing reviewed Wasm ABI execution receipts
- WHEN a gate receipt is parsed
- THEN the receipt includes `wasm-abi-byte-bounds` and `wasm-guest-memory-bounds`

### Requirement: Missing executor resource checks fail closed
r[molten.evidence.executor_resource_gate_receipts.validation] Gate receipt parsing MUST reject receipts missing the execution receipt aggregate ref or required executor resource checks.

#### Scenario: Tampered gate receipt drops resource check
- GIVEN a valid gate receipt
- WHEN `steel-resource-bounds` or `wasm-abi-byte-bounds` is removed
- THEN receipt parsing fails closed

### Requirement: Chain links preserve payload identity
r[molten.evidence.chain_hashing.link_model] Evidence chain links MUST be canonical Preserves artifacts whose refs are computed from the link bytes while preserving the canonical refs of linked payload artifacts.

#### Scenario: Linking a gate receipt does not change the receipt ref
r[molten.evidence.chain_hashing.link_model.preserve_payload]
- GIVEN a canonical `<gate-receipt-v1 ...>` with a known receipt ref
- WHEN a chain link names that receipt as its payload
- THEN the link has its own canonical link ref
- AND the payload ref inside the link equals the original gate receipt ref

#### Scenario: Link identity is stable
r[molten.evidence.chain_hashing.link_identity.stable]
- GIVEN the same chain scope, sequence, previous ref, payload ref, context refs, producer refs, and checks
- WHEN the link is encoded canonically twice
- THEN both encodings produce the same link ref

### Requirement: Chain appends are scoped and monotonic
r[molten.evidence.chain_hashing.genesis_append] Chain append validation MUST enforce genesis shape, same-scope previous-link binding, and monotonic sequence numbers within a chain scope/id/epoch.

#### Scenario: Genesis starts a scoped chain
r[molten.evidence.chain_hashing.genesis_append.genesis]
- GIVEN a chain link with sequence `0`
- WHEN the link has no previous ref and has an admitted chain scope/id/epoch
- THEN append validation accepts it as a genesis link

#### Scenario: Non-genesis links bind the previous link
r[molten.evidence.chain_hashing.genesis_append.previous]
- GIVEN an existing link at sequence `41` for a chain scope/id/epoch
- WHEN a new link for the same chain names the existing link as `prev` and uses sequence `42`
- THEN append validation accepts the continuity check

#### Scenario: Sequence gaps are rejected
r[molten.evidence.chain_hashing.genesis_append.gap]
- GIVEN an existing link at sequence `41`
- WHEN a new link for the same chain names that link as `prev` but uses sequence `43`
- THEN append validation fails closed with a gap diagnostic

### Requirement: Chain verification detects tampering and forks
r[molten.evidence.chain_hashing.verify_receipts] Chain verification MUST emit canonical verification receipts that identify verified links, payload refs, accepted anchor/head refs, and any tamper, gap, stale-head, missing-payload, or fork diagnostics.

#### Scenario: Previous-ref tampering is rejected
r[molten.evidence.chain_hashing.verify_receipts.prev_tamper]
- GIVEN a chain segment from an accepted anchor to a claimed head
- WHEN a link in the segment names a previous ref that does not match the prior verified link
- THEN verification rejects the segment
- AND the verification receipt names the first divergent link

#### Scenario: Fork is rejected under no-fork policy
r[molten.evidence.chain_hashing.verify_receipts.fork]
- GIVEN two links in the same chain scope/id/epoch that both name the same previous link
- WHEN the chain policy requires no forks
- THEN verification rejects the claimed head
- AND emits fork evidence naming both child link refs

#### Scenario: Diagnostic profile can retain fork evidence
r[molten.evidence.chain_hashing.verify_receipts.fork_diagnostic]
- GIVEN a detected fork
- WHEN the active evidence profile is diagnostic-only
- THEN the ledger may retain both fork links and the fork diagnostic receipt
- AND those artifacts do not satisfy production pass evidence gates

### Requirement: Gate profiles may require chain continuity
r[molten.evidence.chain_hashing.gate_receipts] Evidence gates SHOULD be able to require selected pass artifacts to descend from trusted chain anchors or fresh control-plane checkpoints.

#### Scenario: Production gate requires anchored receipt
r[molten.evidence.chain_hashing.gate_receipts.anchor_required]
- GIVEN a production evidence profile that requires chain-hashed receipts
- WHEN a valid gate receipt is not reachable from an accepted chain anchor or checkpoint
- THEN the production gate rejects the receipt as insufficient pass evidence

#### Scenario: Stale head is rejected
r[molten.evidence.chain_hashing.gate_receipts.stale_head]
- GIVEN a verified chain segment that descends from a trusted anchor
- AND a control-plane checkpoint names a newer accepted head for the same chain
- WHEN a gate attempts to use the older head without an admitted historical policy
- THEN the gate rejects the evidence as stale

### Requirement: Trellis predicates bound chain continuity
r[molten.evidence.chain_hashing.trellis_append_predicates] The system SHOULD provide Trellis-backed bounded predicates for chain genesis validity, append validity, no-gap continuity, no-fork policy, anchor descent, and checkpoint range coverage.

#### Scenario: Trellis append predicate agrees with pure validation
r[molten.evidence.chain_hashing.trellis_append_predicates.append]
- GIVEN a bounded previous link summary and candidate link summary
- WHEN pure chain validation accepts the append
- THEN the Trellis append predicate also accepts the append

#### Scenario: Trellis no-fork predicate rejects duplicate children
r[molten.evidence.chain_hashing.trellis_append_predicates.no_fork]
- GIVEN a bounded segment containing two accepted children for one parent under no-fork policy
- WHEN the Trellis no-fork predicate evaluates the segment
- THEN it rejects the segment and names the parent/child summaries

### Requirement: Chain hashing is not global actor ordering
r[molten.evidence.chain_hashing.no_global_chain] Chain hashing MUST NOT require ordinary actor messages or unrelated actor turns to depend on one global chain head.

#### Scenario: Independent turn journals can advance concurrently
r[molten.evidence.chain_hashing.no_global_chain.concurrent_turns]
- GIVEN two unrelated actors with independent turn-journal chain scopes
- WHEN each actor commits an admitted turn
- THEN each turn journal advances under its own chain head
- AND neither turn requires the other's head as previous evidence

### Requirement: Octet and Valence evidence boundary
r[molten.octet_gates.reference_boundary] Molten MUST treat Octet checks and Valence function objects/fingerprints as bounded source-shape and evidence gates, not as semantic correctness proofs or replacements for runtime policy, capability checks, deterministic replay, Trellis predicates, Hegel properties, or Cairn receipt validation.

#### Scenario: Valence evidence displays caveats
- GIVEN a harness report or Cairn receipt that references a Valence function object
- WHEN an operator inspects the evidence
- THEN the report identifies the function object ref, source caveats, fingerprint metadata, and the fact that it does not prove behavioral correctness.

### Requirement: Critical source-surface markers
r[molten.octet_gates.source_surface_markers] Molten MUST identify critical source surfaces for Octet/Valence gating by marker attributes, module paths, object-corpus source paths, remediation-plan critical-surface inventory, or Octet config. Initial surfaces include core transitions, adapter boundaries, test capabilities, secret/capability-bearing types, harness report/oracle validators, redaction/export paths, protocol transition gates, and golden update tools.

#### Scenario: Core transition surface is classified
- GIVEN a function or module implements a runtime core transition
- WHEN Octet evidence is generated
- THEN the surface is classified by marker, manifest, module path, object corpus, remediation inventory, or Octet config
- AND source-gate evidence records the applicable critical surface.

### Requirement: Octet evidence artifacts in reports and receipts
r[molten.octet_gates.evidence_artifacts] Harness reports and Cairn receipts that rely on source gates MUST reference Octet command/config versions, findings, severity summaries, structured finding indexes, Valence function object refs or object-corpus refs, caveat summaries, fingerprints, review manifests, suppressions, and drift summaries as canonical content refs or receipt refs.

#### Scenario: CI receipt references Octet artifact bundle
- GIVEN a CI run that used Octet as an evidence gate
- WHEN the final Cairn or Molten receipt is emitted
- THEN it references the Octet artifact bundle and the harness report refs that consumed its findings.

### Requirement: Octet CI command shape is explicit
r[molten.octet_gates.ci_command_shape] Molten MUST document and support an explicit source-gate command shape that includes `cargo octet check`, focused object-corpus/fingerprint evidence, artifact import, strict gate receipt generation, remediation-plan evidence, harness tests, and Cairn strict validation.

#### Scenario: Strict source gate command emits canonical receipt
- GIVEN the documented strict Octet source-gate sequence is run
- WHEN artifact import and gate commands complete
- THEN Molten emits canonical artifact-ledger, fingerprint, and `octet-gate-receipt-v1` evidence refs.

### Requirement: Core purity gate
r[molten.octet_gates.core_purity_gate] Marked core transition functions MUST be rejected or require review receipts when Octet/Valence detects ambient-effect or abort caveats, including filesystem, network, wall-clock, entropy, process, environment, database, scripting, unsafe, panic, unwrap/expect, direct adapter calls, or semantic thread-scheduling observations.

#### Scenario: Wall-clock in core transition is blocked
- GIVEN a function marked as a Molten core transition
- WHEN the function source uses wall-clock time directly
- THEN Octet flags the caveat
- AND evidence gates reject the change unless review evidence reclassifies or removes the ambient observation.

### Requirement: Adapter boundary evidence gate
r[molten.octet_gates.adapter_boundary_gate] Marked adapter boundary functions MUST identify their effect manifest id, handler profile compatibility, capability and policy check location, trace and receipt emission obligation, resource checkpoint behavior, replay/record behavior or non-replayable status, and structured error mapping.

#### Scenario: Adapter boundary lacks receipt obligation
- GIVEN an adapter function marked for a storage write effect
- WHEN Octet cannot find or validate the boundary's receipt-emission obligation
- THEN the source gate fails before the adapter can be accepted into evidence-bearing profiles.

### Requirement: Effect manifest linkage
r[molten.octet_gates.effect_manifest_linkage] Adapter boundary source evidence MUST link to the Molten effect manifest entries that authorize the boundary, and evidence gates MUST fail when a marked adapter boundary has missing, stale, or mismatched effect-manifest linkage.

#### Scenario: Stale effect linkage is denied
- GIVEN an adapter boundary whose source marker references an effect manifest id
- WHEN that effect id no longer exists in the artifact dependency closure
- THEN Octet or the harness report validator rejects the source evidence as stale.

### Requirement: Adapter conformance runs on boundary drift
r[molten.octet_gates.adapter_conformance_trigger] Drift in adapter boundary source fingerprints SHOULD trigger adapter conformance, replay, or golden evidence before release or admission gates accept the changed boundary.

#### Scenario: Adapter fingerprint drift requires conformance
- GIVEN a changed Valence or object-corpus fingerprint for an adapter boundary
- WHEN CI evaluates the evidence bundle
- THEN it requires adapter conformance or replay evidence before accepting the change.

### Requirement: Authority typing gate
r[molten.octet_gates.authority_typing_gate] Public runtime, policy, storage, harness, and adapter boundary APIs MUST NOT use raw strings, byte arrays, or generic hashes where typed actor/session/peer/run/turn ids, artifact/schema/policy/receipt/evidence/effect-log refs, capability/secret/content/snapshot/trace refs, profile markers, or staged/committed/redacted/revealed state markers are required.

#### Scenario: Stringly capability API is blocked
- GIVEN a public runtime API that accepts a raw string where a capability ref is required
- WHEN Octet evaluates boundary APIs
- THEN it flags the API
- AND evidence gates reject it until the API parses and validates the value into a typed capability ref at the boundary.

### Requirement: Harness backdoor gate
r[molten.octet_gates.harness_backdoor_gate] Harness code MUST NOT directly mutate runtime internals, stores, actor state, fixture state, policy decisions, receipts, traces, or snapshots outside explicit admitted test capabilities that emit canonical trace and receipt evidence.

#### Scenario: Direct store mutation in harness is rejected
- GIVEN harness code writes directly to a runtime store to set up a test
- WHEN the operation is not represented as an admitted test capability or fixture effect
- THEN Octet flags the backdoor
- AND evidence gates reject the harness change.

### Requirement: Testing-harness evidence gates
r[molten.octet_gates.testing_harness_gate] Evidence gates MUST enforce the first-class testing-harness requirements as admissibility criteria, including Preserves rail use, deterministic/replayable execution, actor-registry evidence, resource budgets, adapter conformance, security suites, repro bundles, first-divergence diagnostics, canonical failure artifacts, and canonical gate receipts. A `<harness-failure-v1 ...>` artifact MUST be accepted only as diagnostic evidence, never as a passing harness report for CI, release, admission, or upgrade gates.

#### Scenario: Failure artifact cannot pass a report gate
- GIVEN a gate that requires a passing harness report for a suite, replay, validation, adapter conformance, or security check
- WHEN the available artifact is `<harness-failure-v1 ...>`
- THEN the gate preserves the failure artifact as canonical diagnostic evidence
- AND rejects it as pass evidence.

#### Scenario: Missing canonical failure artifact is a gate failure
- GIVEN a harness command failed during preflight, execution, replay, validation, or export and was configured with an artifact output path
- WHEN the evidence bundle contains only stderr, a nonzero exit code, or renderer-specific JSON/JUnit output
- THEN the gate rejects the bundle because no canonical Preserves failure artifact was produced.

#### Scenario: Passing gate emits receipt artifact
- GIVEN a harness report or report repro bundle is accepted as passing gate evidence
- WHEN the gate decision is recorded for CI, release, admission, or upgrade use
- THEN the decision is represented by a canonical `<gate-receipt-v1 ...>` artifact containing artifact refs plus validation, replay, budget, and actor-registry check evidence.

### Requirement: Production/test separation gate
r[molten.octet_gates.production_test_separation] Test-only APIs, fixture adapters, bypass capabilities, debug hooks, and exploratory non-replayable profiles MUST be feature/profile/policy isolated from production builds and MUST be denied in production profiles unless explicitly admitted for record, replay, or debug use with evidence.

#### Scenario: Test bypass leaks into production profile
- GIVEN a test-only bypass capability reachable from a production profile
- WHEN Octet or the harness report validator evaluates the build evidence
- THEN the evidence gate fails unless an explicit policy and receipt authorize the debug, record, or replay use.

### Requirement: Secret and capability rendering gate
r[molten.octet_gates.secret_rendering_gate] Secret and capability-bearing types MUST NOT expose unredacted debug, display, serialization, tracing, logging, report export, panic, or error rendering paths unless those paths route through redaction, encryption, or reveal-policy evidence.

#### Scenario: Secret ref debug output is blocked
- GIVEN a type marked as a secret ref
- WHEN the type derives or implements debug output that renders secret material without redaction policy
- THEN Octet flags the path
- AND evidence gates reject the change.

### Requirement: Resource source-shape gate
r[molten.octet_gates.resource_shape_gate] Runtime, adapter, harness, transcript, property, and report paths MUST have deterministic bounds or checkpoints for loops, queues, recursion, deferred work, trace/report builders, Wasm fuel, Steel/native checkpoints, and materialization of content or snapshots.

#### Scenario: Unbounded report builder is blocked
- GIVEN a report export path accumulates runtime traces without a declared trace-byte or record-count budget
- WHEN Octet evaluates the source surface
- THEN it flags the unbounded path
- AND evidence gates require a budget checkpoint or review receipt.

### Requirement: Fingerprint drift gate
r[molten.octet_gates.fingerprint_drift_gate] Drift in Valence function objects, object corpus, or Octet fingerprints for critical surfaces MUST trigger required follow-up evidence such as harness replay, Hegel property reports, golden trace updates, adapter conformance, security suites, Trellis checks, migration notes, or review receipts before CI, release, admission, or upgrade gates accept the change.

#### Scenario: Adapter fingerprint drift requires conformance
- GIVEN a changed Valence fingerprint for an adapter boundary
- WHEN CI evaluates the evidence bundle
- THEN it requires an adapter conformance report and replay or record evidence before accepting the change.

### Requirement: Fail-closed Octet caveats
r[molten.octet_gates.fail_closed_caveats] Evidence gates MUST fail when required Octet or Valence artifacts, caveat summaries, function objects, object-corpus refs, fingerprints, review manifests, suppressions, or drift summaries are missing, malformed, stale, unsupported, or not linked to the relevant harness/Cairn evidence.

#### Scenario: Missing caveat summary fails evidence
- GIVEN a harness report claims a core transition passed source gating
- WHEN the referenced Valence caveat summary is missing or stale
- THEN the report validator rejects the claim rather than treating missing caveats as clean evidence.

### Requirement: Review receipt linkage
r[molten.octet_gates.review_receipt_linkage] Octet suppressions, review manifests, caveat overrides, and fingerprint-drift approvals MUST link to Cairn receipt refs or authenticated content refs so source-gate exceptions remain auditable.

#### Scenario: Suppression lacks review receipt
- GIVEN an Octet suppression for a core purity caveat
- WHEN the suppression lacks a review receipt or authenticated review manifest ref
- THEN evidence gates reject the suppression for CI, release, admission, or upgrade evidence.

### Requirement: Core purity source-gate tests
r[molten.octet_gates.core_purity_tests] Molten SHOULD test that strict source gates deny ambient-effect, abort, stale metadata, malformed artifact, missing object-corpus, and missing fingerprint cases before downstream consumers accept the source evidence.

#### Scenario: Warning-only status denies strict gate
- GIVEN Octet artifacts with warning-only status
- WHEN the strict source-gate evaluator runs
- THEN it emits a deny receipt with diagnostics rather than pass evidence.

### Requirement: Authority typing source-gate tests
r[molten.octet_gates.authority_typing_tests] Molten SHOULD test that stringly capability, receipt, schema, content-ref, or source-gate mixups are rejected by source-gate validation or downstream consumers before runtime admission.

#### Scenario: Raw source summary is not a typed gate receipt
- GIVEN a downstream consumer receives only an Octet summary ref
- WHEN source-gate validation runs
- THEN validation denies because a canonical typed `octet-gate-receipt-v1` value is required.

### Requirement: Harness backdoor source-gate tests
r[molten.octet_gates.harness_backdoor_tests] Molten SHOULD test that invisible harness store mutation, private runtime backdoors, and canonical failure artifacts cannot be accepted as passing gate evidence.

#### Scenario: Failure artifact is diagnostic only
- GIVEN a canonical harness failure artifact
- WHEN a pass gate requires a harness report
- THEN the failure artifact remains diagnostic evidence only and cannot satisfy the pass gate.

### Requirement: Adapter boundary source-gate tests
r[molten.octet_gates.adapter_boundary_tests] Molten SHOULD test that adapter boundaries missing effect, trace, receipt, resource, replay, or fingerprint evidence are denied by source-gate or source-gate-validation logic.

#### Scenario: Missing fingerprint evidence denies adapter gate
- GIVEN an otherwise pass-shaped Octet gate receipt without object-corpus fingerprint evidence
- WHEN source-gate validation runs
- THEN the validation denies with missing fingerprint coverage diagnostics.

### Requirement: Fingerprint drift source-gate tests
r[molten.octet_gates.fingerprint_drift_tests] Molten SHOULD test that stale config/profile hashes, changed structured findings, missing object-corpus refs, warning-baseline regressions, and unreviewed critical findings deny or require review evidence.

#### Scenario: Stale source gate denies
- GIVEN a previously passing Octet gate receipt with stale config or profile hash metadata
- WHEN source-gate validation runs
- THEN the validation denies and records deterministic stale-evidence diagnostics.

### Requirement: Octet gate policy is canonical
r[molten.octet_fail_closed_ci.gate_policy] Molten MUST represent strict Octet source-gate policy as canonical `octet-gate-policy-v1` evidence that binds profile, command shape, required artifacts, deny statuses, critical lint classes, quarantine policy, and checks.

#### Scenario: Strict profile policy lists deny statuses
- GIVEN the strict CI Octet profile is evaluated
- WHEN Molten renders the gate policy
- THEN the policy lists `warning-only`, missing, malformed, stale, unsupported, and error outcomes as deny statuses
- AND names the required Octet artifacts and critical lint classes.

### Requirement: Octet gate receipt is canonical
r[molten.octet_fail_closed_ci.gate_receipt] Molten MUST represent Octet source-gate decisions with canonical `octet-gate-receipt-v1` records that bind decision, policy ref, command ref, status ref, summary ref, structured findings ref, object-corpus ref, fingerprint evidence ref, config hash, profile hash, toolchain, finding counts, baseline/review refs, diagnostics, and checks.

#### Scenario: Passing strict gate has complete evidence
- GIVEN an Octet run for a strict CI profile
- AND all required artifacts are present and bound by canonical content refs
- AND the run has no findings
- WHEN Molten emits the Octet gate decision
- THEN the decision is a canonical pass receipt
- AND the receipt references the exact command, config, profile, toolchain, and evidence artifacts used to decide.

### Requirement: Octet gate artifacts are ledger-classified
r[molten.octet_fail_closed_ci.ledger_classification] Molten MUST classify Octet gate policies, receipts, command artifacts, status artifacts, summary artifacts, object-corpus artifacts, structured findings, warning baselines, review manifests, source-gate validation receipts, and fingerprint evidence in the local ledger/catalog.

#### Scenario: Imported Octet artifacts are searchable
- GIVEN Octet gate artifacts are imported into the local ledger
- WHEN the ledger classifies them
- THEN operators can distinguish Octet status, summary, object corpus, fingerprint, gate receipt, baseline, and validation artifacts by kind.

### Requirement: Octet artifacts are bound by canonical refs
r[molten.octet_fail_closed_ci.artifact_ref_binding] Molten MUST bind `command.txt`, `status.json`, `summary.txt`, structured findings, object-corpus receipts, and fingerprint evidence by canonical content refs before accepting an Octet gate result.

#### Scenario: Summary drift changes receipt evidence
- GIVEN a gate receipt binds a summary artifact ref
- WHEN the summary text changes
- THEN the structured findings or summary ref changes
- AND stale receipts cannot silently cover the new summary.

### Requirement: Warning-only status fails strict CI
r[molten.octet_fail_closed_ci.status_semantics] Molten MUST treat `warning-only` as a deny status for strict Octet profiles even when the `cargo-octet` process exit code is `0`.

#### Scenario: Process success alone is not pass evidence
- GIVEN `cargo octet check` exits with code `0`
- AND the Octet status is `warning-only`
- WHEN a strict CI, release, admission, or upgrade gate evaluates the run
- THEN the gate emits a deny receipt rather than pass evidence.

### Requirement: Required Octet artifacts fail closed
r[molten.octet_fail_closed_ci.missing_artifact_denial] Octet gate evaluation MUST deny when required artifacts are missing, malformed, stale, unsupported, or not bound to the expected command, config hash, profile hash, toolchain, object corpus, or source scope.

#### Scenario: Missing status denies with receipt
- GIVEN the Octet artifacts directory lacks `status.json`
- WHEN strict source-gate evaluation runs
- THEN Molten emits a deny receipt with diagnostics
- AND no downstream consumer may claim a source-gate pass.

### Requirement: Critical lint findings deny without review
r[molten.octet_fail_closed_ci.critical_lint_denial] Strict and quarantine Octet profiles MUST deny unreviewed critical findings for panic/abort paths, unwrap/expect, ambient time or entropy in core evidence paths, unbounded loops, critical resource-shape failures, authority typing violations, harness backdoors, secret/capability rendering leaks, and missing adapter boundary evidence.

#### Scenario: Unreviewed critical finding denies quarantine
- GIVEN an Octet warning baseline contains a `no_unwrap` finding on a critical evidence path
- WHEN no review manifest covers the exact finding and profile
- THEN quarantine evaluation denies even though the finding existed before.

### Requirement: Object-corpus and fingerprint evidence is required
r[molten.octet_fail_closed_ci.object_corpus_denial] Strict source-gate pass claims MUST deny when configured critical paths lack object-corpus and fingerprint evidence.

#### Scenario: Missing object corpus denies
- GIVEN `status.json` and `summary.txt` are clean
- WHEN the required object-corpus receipt or object-set fingerprint is missing
- THEN the strict gate denies before any downstream evidence consumer can claim source-gate pass evidence.

### Requirement: CLI exposes Octet gate command
r[molten.octet_fail_closed_ci.cli_gate] Molten MUST expose a local command shape such as `molten test octet gate --artifacts target/octet --profile strict-ci --receipt-out ...` that reads Octet artifacts and writes canonical gate receipts.

#### Scenario: CLI writes deny receipt for warning-only
- GIVEN an Octet artifacts directory with warning-only status
- WHEN the operator runs the Octet gate CLI
- THEN the command writes a canonical deny receipt preserving diagnostics.

### Requirement: Strict CI sequence is documented
r[molten.octet_fail_closed_ci.ci_command_shape] Molten MUST document the strict CI sequence: Octet check, lib-only check where applicable, object corpus receipt, artifact import, Octet gate receipt, remediation plan, harness gates/tests, Clippy, and Cairn strict validation.

#### Scenario: Documented sequence includes source gate receipt
- GIVEN an operator follows the strict Octet source-gate sequence
- WHEN the sequence reaches the source-gate step
- THEN it produces an `octet-gate-receipt-v1` suitable for downstream validation.

### Requirement: Release and admission bind strict Octet receipts
r[molten.octet_fail_closed_ci.release_admission_binding] Release, upgrade, node-runtime startup, remote job admission, and evidence-bearing harness profiles that require source-shape evidence MUST bind passing strict Octet gate receipt refs or source-gate validation refs rather than raw `cargo octet` output.

#### Scenario: Node startup requires source gate receipt
- GIVEN node runtime startup claims source-gated daemon or adapter code
- WHEN startup evidence is evaluated
- THEN it must include passing strict Octet gate validation evidence for the relevant source scope
- AND missing, denied, stale, or tampered gate evidence denies startup before adapters start.

#### Scenario: Remote job admission rejects raw summary only
- GIVEN remote job admission receives only `summary.txt` or process output as source evidence
- WHEN target-side admission evaluates executable readiness
- THEN admission denies because a passing strict source-gate validation receipt is required.

### Requirement: Denied gates preserve diagnostics
r[molten.octet_fail_closed_ci.diagnostic_output] Molten MUST preserve raw Octet status, summary, structured findings, object-corpus, and diagnostics as evidence even when the gate denies, without treating those artifacts as pass receipts.

#### Scenario: Warning artifacts remain diagnostic
- GIVEN strict source-gate evaluation denies a warning-only run
- WHEN the receipt is inspected
- THEN the raw Octet artifacts are still referenced for diagnosis
- AND the decision remains deny.

### Requirement: Warning-only strict test coverage
r[molten.octet_fail_closed_ci.warning_only_test] Molten SHOULD test that `status=warning-only` denies under the strict profile.

#### Scenario: Warning-only fixture denies
- GIVEN a fixture Octet status with warnings and no errors
- WHEN strict gate evaluation runs
- THEN the test asserts the decision is deny.

### Requirement: Missing and stale artifact tests
r[molten.octet_fail_closed_ci.missing_status_test] Molten SHOULD test missing, malformed, stale, unsupported, and mismatched `status.json`, missing object corpus receipts, and mismatched config/profile hash denial.

#### Scenario: Stale metadata fixture denies
- GIVEN a fixture Octet status with stale config or profile hash
- WHEN strict gate evaluation runs
- THEN the test asserts the decision is deny with stale metadata diagnostics.

### Requirement: Critical lint tests
r[molten.octet_fail_closed_ci.critical_lint_test] Molten SHOULD test that unreviewed critical lint findings deny and exact review manifests are required for temporary quarantine acceptance.

#### Scenario: Reviewed critical finding is temporary
- GIVEN a critical finding and a matching unexpired review manifest for quarantine
- WHEN quarantine baseline evaluation runs
- THEN it may pass while strict CI still requires strict evidence.

### Requirement: Receipt binding tests
r[molten.octet_fail_closed_ci.receipt_binding_test] Molten SHOULD test that tampering with command, status, summary, structured findings, object-corpus, or fingerprint refs changes or denies the gate receipt.

#### Scenario: Tampered fingerprint denies downstream validation
- GIVEN a pass-shaped Octet gate receipt whose fingerprint ref is replaced with a malformed ref
- WHEN source-gate validation runs
- THEN validation denies before downstream side effects.

### Requirement: Octet remediation metrics are canonical evidence
r[molten.octet_tigerstyle_remediation.baseline_metrics] Molten MUST capture Octet/TigerStyle remediation metrics as canonical evidence that binds workspace, lib-only, and focused critical-path status refs, summary refs, object-corpus refs, finding counts, warning counts, error counts, autofixable counts, and plan refs.

#### Scenario: Remediation plan binds current counts
- GIVEN workspace and lib-only Octet artifacts
- WHEN Molten builds the remediation plan
- THEN the plan records status, summary, object-corpus refs, counts, diagnostics, and checks for those scopes.

### Requirement: Critical source surfaces are inventoried
r[molten.octet_tigerstyle_remediation.critical_surface_inventory] Molten MUST inventory critical source surfaces relevant to Octet/TigerStyle remediation, including source-gate/admission, harness/gates, node runtime startup, job execution, ledger/evidence, adapter boundaries, redaction/export, and CLI artifact-output paths.

#### Scenario: Critical surface lists source files
- GIVEN the remediation plan is rendered
- WHEN an operator inspects critical surfaces
- THEN each surface lists source files, warning counts, critical counts, and rationale.

### Requirement: Remediation priority is explicit
r[molten.octet_tigerstyle_remediation.priority_order] Molten MUST prioritize Octet/TigerStyle burn-down work as critical deny classes first, resource bounds second, high-arity and long functions third, file/module splits fourth, and style/autofix cleanup last.

#### Scenario: Critical finding outranks style churn
- GIVEN both a critical source-gate finding and a style-only import finding exist
- WHEN remediation work is scheduled
- THEN the critical source-gate finding is scheduled first unless a review receipt explains the exception.

### Requirement: Hidden suppressions are forbidden
r[molten.octet_tigerstyle_remediation.no_suppression_policy] Molten MUST NOT treat hidden suppressions as remediation. Every retained active warning MUST have scheduled remediation, an explicit reviewed quarantine receipt, or a documented configuration caveat that strict consumers can distinguish from source-remediated zero.

#### Scenario: Disabled lint remains a caveat
- GIVEN an Octet lint family is disabled in configuration
- WHEN the remediation plan is inspected
- THEN the plan records the disabled family as a caveat or future burn-down item rather than hidden clean evidence.

### Requirement: Panic and unwrap caveats are removed or reviewed
r[molten.octet_tigerstyle_remediation.no_panic_unwrap] Molten MUST remove, deny, or review `panic`, `unwrap`, and `expect` findings on critical evidence-bearing paths before those paths can satisfy strict source-gate evidence.

#### Scenario: Critical unwrap requires review
- GIVEN a critical path contains an `unwrap` finding
- WHEN strict or quarantine source-gate evidence is evaluated
- THEN the finding denies unless an exact review manifest covers it temporarily for the profile.

### Requirement: Ambient clock caveats are isolated
r[molten.octet_tigerstyle_remediation.no_ambient_clock] Molten MUST remove ambient wall-clock/time findings from deterministic evidence paths or isolate them behind explicit shell receipts and source-gate review evidence.

#### Scenario: Clock use in deterministic core denies
- GIVEN Octet reports ambient clock use in a deterministic core path
- WHEN strict source-gate evidence is evaluated
- THEN the gate denies until the clock use is removed or isolated behind explicit receipt evidence.

### Requirement: Unbounded loops are bounded or reviewed
r[molten.octet_tigerstyle_remediation.no_unbounded_loops] Molten MUST add explicit limits, yield/checkpoints, or review receipts for unbounded loop and recursion findings in runtime, harness, job, adapter, and report paths before strict source-gate acceptance.

#### Scenario: Unbounded report loop denies
- GIVEN a report renderer accumulates unbounded data-dependent output
- WHEN Octet evaluates the critical surface
- THEN strict source-gate evidence denies unless a deterministic budget or review receipt covers the path.

### Requirement: Sentinel fallbacks are replaced by typed denial paths
r[molten.octet_tigerstyle_remediation.no_sentinel_fallbacks] Source-gate, admission, and evidence paths MUST avoid sentinel fallback refs or strings where typed option/result handling and canonical denial receipts are required.

#### Scenario: Missing ref becomes explicit denial
- GIVEN a required source-gate ref is absent
- WHEN startup, job admission, or upgrade planning validates evidence
- THEN Molten emits a deny receipt instead of substituting a synthetic passing sentinel.

### Requirement: Collections on evidence paths are bounded
r[molten.octet_tigerstyle_remediation.collection_bounds] Evidence-bearing runtime, job, node, harness, catalog, adapter, source-gate, and report paths MUST use deterministic bounds, validated prior limits, or explicit resource accounting for data-dependent collection growth.

#### Scenario: Finding index has a maximum
- GIVEN Octet structured findings are parsed
- WHEN the finding index is loaded
- THEN Molten enforces a maximum entry count before inserting into collections.

### Requirement: Builder input structs replace high-arity evidence helpers
r[molten.octet_tigerstyle_remediation.builder_input_structs] Molten SHOULD replace high-arity receipt/value builders on critical evidence paths with named input structs that validate typed refs and invariants before rendering canonical Preserves values. Remaining high-arity helpers MUST remain visible in the remediation plan or future burn-down work rather than hidden as clean evidence.

#### Scenario: Receipt builder uses named inputs
- GIVEN a critical receipt helper grows many fields
- WHEN it is remediated
- THEN a named input struct carries the fields and validation before canonical rendering.

### Requirement: Public evidence boundaries validate refs
r[molten.octet_tigerstyle_remediation.typed_ref_boundaries] Public evidence boundaries SHOULD replace raw strings and generic hashes with typed ref/id/profile structs or parsing functions that fail closed. Remaining raw-string boundaries MUST be limited to CLI/config parsing edges or documented future burn-down items.

#### Scenario: CLI string is parsed before runtime use
- GIVEN a CLI command accepts a source-gate receipt ref string
- WHEN runtime or admission logic consumes it
- THEN the value is parsed, validated, or denied before it is treated as evidence.

### Requirement: Remediation adds assertion coverage
r[molten.octet_tigerstyle_remediation.assertion_density] Remediation slices SHOULD add positive and negative assertions around pure helpers, denial paths, source-gate validators, collection bounds, and receipt binding behavior introduced by the cleanup.

#### Scenario: Denial helper has negative assertion
- GIVEN a source-gate validation helper denies stale evidence
- WHEN tests run
- THEN they assert both the deny decision and a diagnostic that identifies the stale evidence.

### Requirement: CLI shell split remains tracked
r[molten.octet_tigerstyle_remediation.cli_shell_split] Molten SHOULD split large CLI imperative-shell surfaces into smaller dispatch modules and pure command input conversion helpers over time. Until that source-remediated split is complete, the remediation evidence MUST document the caveat and MUST NOT claim that disabled file/function-size lint families represent source-remediated zero.

#### Scenario: CLI split is documented as future work
- GIVEN strict source-gate evidence is configuration-clean because file/function-size lint families are disabled
- WHEN the remediation plan is inspected
- THEN CLI/module split work remains listed as a burn-down item or caveat.

### Requirement: Job DAG split remains tracked
r[molten.octet_tigerstyle_remediation.job_dag_split] Molten SHOULD split large job DAG surfaces into DTO, parse, sync, admission, execution, memo/cache, and test-support modules without changing canonical refs. Until complete, current Octet evidence MUST distinguish configuration-clean status from source-remediated zero.

#### Scenario: Job DAG split preserves canonical refs
- GIVEN job DAG module splitting is performed in a future slice
- WHEN validation runs
- THEN canonical job refs, receipts, and replay outputs remain stable unless intentionally versioned.

### Requirement: Node runtime shape remains tracked
r[molten.octet_tigerstyle_remediation.node_runtime_shape] Molten SHOULD keep node runtime startup code shaped around typed inputs, bounded adapter lists, deterministic duplicate-free ordering, short receipt helpers, and deny receipts for failed startup. Remaining shape debt MUST stay visible in remediation evidence.

#### Scenario: Startup denial remains receipt-backed
- GIVEN source-gate evidence is missing or denied
- WHEN node startup evaluates configuration
- THEN startup emits a canonical deny receipt and starts no production adapters.

### Requirement: Object corpus evidence is refreshed after remediation
r[molten.octet_tigerstyle_remediation.object_corpus_refresh] Molten MUST refresh object-corpus/fingerprint evidence for changed critical paths before claiming strict source-gate pass evidence for those paths.

#### Scenario: Changed source path refreshes corpus
- GIVEN a critical source path changes during remediation
- WHEN source-gate evidence is produced
- THEN the object-corpus and fingerprint refs reflect the changed source scope.

### Requirement: Focused Octet runs are recorded
r[molten.octet_tigerstyle_remediation.focused_octet_runs] Remediation slices SHOULD re-run focused Octet checks after changes and record before/after finding deltas, even when the current result is configuration-clean.

#### Scenario: Focused run records zero findings
- GIVEN a focused critical-path Octet run reports zero findings
- WHEN the remediation plan is generated
- THEN it records the status and count refs for that focused evidence.

### Requirement: Strict profile dry-runs drive burn-down
r[molten.octet_tigerstyle_remediation.strict_profile_dry_run] Molten SHOULD run strict Octet gate dry-runs until warning-only status is eliminated or only reviewed noncritical debt remains under explicit quarantine. Configuration-clean strict passes MUST be labeled with disabled-lint caveats when applicable.

#### Scenario: Strict dry-run rejects warning-only
- GIVEN a strict profile dry-run sees warning-only status
- WHEN the gate evaluates it
- THEN it denies and records warning counts for the next burn-down slice.

### Requirement: Remediation must preserve canonical behavior
r[molten.octet_tigerstyle_remediation.no_regression_tests] Remediation SHOULD include tests or validation proving canonical refs, report receipts, job execution outputs, source-gate receipts, and node startup evidence remain stable except where intentionally versioned.

#### Scenario: Source gate ref remains deterministic
- GIVEN the same Octet artifacts and policy
- WHEN gate evaluation runs after a remediation-only refactor
- THEN the canonical gate receipt is stable unless an input artifact or version changed.

### Requirement: Cairn task drain follows source evidence
r[molten.octet_tigerstyle_remediation.cairn_task_drain] Molten MUST check off Octet fail-close, quarantine, and TigerStyle remediation tasks only when the corresponding code, documentation, caveat, strict gate receipt, or future-work evidence is present and validated by Cairn gates.

#### Scenario: Deferred split is not claimed as finished source cleanup
- GIVEN a module split remains future work
- WHEN the Cairn task package is archived
- THEN the accepted spec records the future-work caveat instead of claiming source-remediated zero.

### Requirement: Downstream consumers validate Octet gate receipt content
r[molten.octet_source_gate_receipt_validation.spec.content_validation] Downstream evidence consumers MUST validate the actual canonical `octet-gate-receipt-v1` value before treating an Octet source-gate ref as pass evidence.

#### Scenario: Raw summary is not pass evidence
- GIVEN a downstream node startup, remote job admission, or upgrade plan with only `summary.txt`, `status.json`, or `cargo octet` process output
- WHEN source-gate validation runs
- THEN validation denies
- AND the consumer cannot claim `strict-octet-source-gate-bound`

#### Scenario: Denied gate receipt is rejected
- GIVEN an `octet-gate-receipt-v1` with decision `deny`
- WHEN a downstream consumer validates it as strict source-gate evidence
- THEN validation emits a canonical deny receipt
- AND no downstream side effect is admitted

### Requirement: Strict pass receipts must be current and scoped
r[molten.octet_source_gate_receipt_validation.spec.current_strict_scope] Source-gate validation MUST require decision `pass`, profile `strict-ci`, current Octet config/profile/toolchain refs, and source-scope object-corpus/fingerprint coverage for the downstream consumer.

#### Scenario: Stale config hash denies
- GIVEN a previously passing Octet gate receipt
- AND the current `[workspace.metadata.octet]`, command scope, pass-through args, `Cargo.toml`, or `dylint.toml`-derived profile evidence has changed
- WHEN node startup, remote job admission, or upgrade planning validates the receipt
- THEN validation denies as stale

#### Scenario: Quarantine profile is not strict source evidence
- GIVEN a quarantine-profile Octet receipt that covers existing warning debt
- WHEN a strict downstream consumer validates it for production startup, remote admission, or upgrade planning
- THEN validation denies because quarantine evidence is not a strict source-gate pass

#### Scenario: Missing fingerprint coverage denies
- GIVEN a pass-shaped Octet gate receipt without object-corpus or fingerprint evidence for the required consumer source scope
- WHEN source-gate validation runs
- THEN validation denies before downstream side effects

### Requirement: Consumers bind validation receipts before side effects
r[molten.octet_source_gate_receipt_validation.spec.consumer_binding] Node startup, remote job admission, and upgrade planning MUST bind `octet-source-gate-validation-v1` pass receipt refs in their own receipts before performing side effects.

#### Scenario: Node startup denies before adapters start
- GIVEN a node config that references a missing, denied, stale, or tampered Octet gate receipt
- WHEN startup validation runs
- THEN `node-startup-receipt-v1` denies
- AND production adapters are not started

#### Scenario: Remote job admission denies before executable readiness
- GIVEN a remote job admission request with invalid source-gate evidence
- WHEN target-side admission evaluates executable readiness
- THEN the job admission receipt denies
- AND it does not claim executable-artifact readiness or allow execution

#### Scenario: Upgrade planning denies before irreversible work
- GIVEN an upgrade plan that would move names, run storage migrations, or schedule irreversible tasks
- AND strict Octet source-gate validation fails
- WHEN the upgrade plan is evaluated
- THEN the plan denies before name moves, migrations, or transcript-gated work

### Requirement: Tampered source-gate evidence fails closed
r[molten.octet_source_gate_receipt_validation.spec.tamper_denial] Source-gate validation MUST deny when receipt diagnostics/checks claim pass but bound refs, counts, structured findings, object-corpus evidence, or fingerprint evidence are missing, malformed, or inconsistent.

#### Scenario: Object corpus ref tampering denies
- GIVEN an Octet gate receipt whose object-corpus ref has been replaced after the gate was generated
- WHEN downstream validation recomputes and checks the receipt evidence
- THEN validation denies and reports the mismatched object-corpus evidence

#### Scenario: Critical finding count tampering denies
- GIVEN an Octet gate receipt whose decision/checks claim pass
- AND structured findings still contain uncovered critical findings or inconsistent counts
- WHEN downstream validation runs
- THEN validation denies and records deterministic diagnostics

### Requirement: Octet warning baselines are explicit quarantine evidence
r[molten.octet_warning_quarantine.spec.baseline_artifact] Temporary Octet warning baselines MUST be canonical `octet-warning-baseline-v1` artifacts that bind the source scope, Octet config hash, profile hash, toolchain, source snapshot ref, stable finding keys, expiry, allowed profiles, burn-down targets, review refs, and checks.

#### Scenario: Baseline is visible evidence
- GIVEN CI is running in a quarantine profile
- WHEN the Octet gate evaluates existing warnings
- THEN the gate references an `octet-warning-baseline-v1` artifact
- AND downstream receipts identify the decision as quarantine-covered debt rather than strict source-gate pass evidence

#### Scenario: Hidden suppression file is rejected
- GIVEN Octet warnings are suppressed by local comments or hidden config with no canonical baseline artifact
- WHEN the quarantine profile evaluates the run
- THEN the gate denies because the warning debt is not auditable evidence

### Requirement: Quarantine comparison denies regressions
r[molten.octet_warning_quarantine.spec.no_new_findings] Quarantine profiles MUST deny when a current Octet run contains new, moved, unkeyed, escalated, malformed, or unsupported findings relative to the baseline and attached review receipts.

#### Scenario: One new warning fails quarantine CI
- GIVEN an Octet baseline covering all existing findings
- AND a new run with one additional finding key
- WHEN the quarantine profile evaluates the run
- THEN it emits an `octet-baseline-receipt-v1` deny receipt
- AND the deny diagnostics identify the new finding key

#### Scenario: Removed warning is accepted and counted
- GIVEN an Octet baseline covering an old finding
- AND a new run where that finding is absent
- WHEN the quarantine profile evaluates the run
- THEN the baseline receipt records the finding as removed
- AND the burn-down count decreases

### Requirement: Critical findings cannot be silently baselined
r[molten.octet_warning_quarantine.spec.critical_review] Baselines MUST NOT silently cover critical findings such as panic/unwrap, ambient time or entropy in core paths, unbounded loops, critical resource-shape failures, authority typing violations, secret rendering, harness backdoors, or missing adapter evidence; each retained critical finding MUST have a review receipt bound to the exact finding key, source fingerprint, risk rationale, and replacement plan.

#### Scenario: Baseline contains unreviewed critical finding
- GIVEN a baseline includes a `no_unwrap` finding on a critical evidence path
- AND no review receipt covers that exact finding and profile
- WHEN quarantine CI evaluates the baseline
- THEN the gate denies even though the finding existed before

#### Scenario: Reviewed critical finding is temporary
- GIVEN a critical finding has an authenticated review receipt with expiry and mitigation plan
- WHEN quarantine CI evaluates it before expiry
- THEN the gate may count it as reviewed debt
- BUT strict CI still denies unless the strict profile explicitly accepts that review receipt

### Requirement: Baselines expire and shrink
r[molten.octet_warning_quarantine.spec.expiry_and_shrink] Octet warning baselines MUST expire, and every refresh MUST reduce uncovered finding count or bind review receipts explaining deferred findings and a new burn-down target.

#### Scenario: Expired baseline denies
- GIVEN a baseline whose `expires-at` is before the current gate evaluation time or logical release milestone
- WHEN the quarantine profile evaluates an otherwise matching run
- THEN the gate denies and requires a refreshed baseline or strict warning-free run

#### Scenario: Refresh without shrink requires review
- GIVEN a baseline refresh with the same or higher warning count
- WHEN no review receipts justify deferred findings
- THEN the gate denies the refresh because warning debt did not shrink

### Requirement: Quarantine receipts do not replace strict pass receipts
r[molten.octet_warning_quarantine.spec.strict_separation] Quarantine pass receipts MUST NOT be accepted as strict release, upgrade, node startup, or remote admission source-gate pass evidence after the configured transition deadline.

#### Scenario: Release rejects quarantine receipt
- GIVEN a release evidence bundle contains an `octet-gate-receipt-v1` that passed only through `quarantine-ci`
- WHEN the release gate requires strict source evidence
- THEN the release gate denies until a strict Octet gate pass receipt is provided
