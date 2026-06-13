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
