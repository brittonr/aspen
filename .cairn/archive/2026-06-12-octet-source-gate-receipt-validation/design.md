## Context

The Octet gate slices introduced canonical `octet-gate-policy-v1`, `octet-gate-receipt-v1`, quarantine baselines, review manifests, structured finding refs, and fingerprint evidence. Strict gates deny current warning-only artifacts. Recent node-runtime/job/upgrade work began carrying source-gate refs, but the consumers still need a concrete validation boundary: they must read the receipt value, verify it is a strict pass for the expected scope, and bind a fresh validation result into their own receipts.

This slice is about downstream consumption of already-produced Octet gate receipts. It does not burn down existing Octet warnings and does not make warning-only artifacts pass.

## Goals

- Define canonical downstream source-gate requirement and validation receipt records.
- Expose a shared validator for strict Octet pass receipts.
- Validate actual `octet-gate-receipt-v1` values, including decision, profile, policy ref, command ref, status ref, summary ref, structured findings ref, object-corpus ref, fingerprint ref, counts, diagnostics, and checks.
- Recompute or load the current workspace Octet metadata/config/profile hashes and reject stale receipts.
- Require source-scope/object-corpus/fingerprint evidence for configured critical paths.
- Wire validation receipts into node runtime startup, remote job admission, and upgrade planning.
- Ensure consumer denial receipts explain which source-gate precondition failed and run before downstream side effects.

## Non-Goals

- No permanent quarantine pass for strict release/admission profiles.
- No warning burn-down or automatic Octet autofix work.
- No new release-bundle gate implementation; release should use this validator in a later slice.
- No claim that Octet proves semantic correctness.
- No acceptance of raw `cargo octet` output, `summary.txt`, or process exit code as pass evidence.

## Records

```preserves
<octet-source-gate-requirement-v1
  "molten.octet.source-gate-requirement.v1"
  <consumer "node-startup"|"job-remote-admission"|"upgrade-plan">
  <subject <subject-ref>>
  <required-profile "strict-ci">
  <source-scope ["src/octet_gate.rs" "src/node_runtime.rs" ...]>
  <current-config-ref <config-ref>>
  <current-profile-ref <profile-ref>>
  <required-evidence ["status" "summary" "structured-findings" "object-corpus" "fingerprint"]>
  <freshness <same-workspace-metadata>>
  <checks [<check "strict-profile-required" "pass"> ...]>>
```

The `subject` is the downstream thing being admitted: node config/startup ref, job admission request/ref, or upgrade plan ref. The source-scope list is a canonical, deterministic list of source paths or source-scope fingerprints expected by the downstream consumer.

```preserves
<octet-source-gate-validation-v1
  "molten.octet.source-gate-validation.v1"
  <decision "pass"|"deny">
  <requirement <requirement-ref>>
  <gate-receipt <octet-gate-receipt-ref-or-none>>
  <gate-policy <policy-ref-or-none>>
  <status <status-ref-or-none>>
  <summary <summary-ref-or-none>>
  <findings <findings-ref-or-none>>
  <object-corpus <object-corpus-ref-or-none>>
  <fingerprint <fingerprint-ref-or-none>>
  <counts <findings n> <warnings n> <errors n> <critical n> <uncovered n>>
  <diagnostics ["..." ...]>
  <checks [<check "gate-receipt-pass" "pass"|"fail"> ...]>>
```

A pass validation receipt means the referenced Octet gate receipt was parsed and accepted for this consumer. A deny validation receipt is still canonical evidence, but downstream pass receipts must not cite a deny validation as satisfying source-gate policy.

## Validator Semantics

The shared validator accepts an expected requirement and an Octet gate receipt artifact/ref. It must:

1. Parse the artifact as canonical Preserves and verify the schema tag is `molten.octet.gate-receipt.v1`.
2. Require decision `pass`; `deny`, `warning-only`, unknown, or missing decisions fail closed.
3. Require profile/check evidence for `strict-ci`; quarantine or advisory profiles are not sufficient for node startup, remote job admission, or upgrade planning.
4. Recompute the current Octet config/profile refs from `[workspace.metadata.octet]`, `dylint.toml`, command scope/pass-through args, toolchain metadata, and the configured source scope; deny stale or mismatched refs.
5. Verify the gate receipt binds required artifact refs: command, status, summary, structured findings, object corpus, and fingerprint evidence.
6. Verify the object corpus/fingerprint evidence covers the expected critical paths for the consumer.
7. Reject raw Octet outputs, raw summary files, missing refs, malformed records, unsupported cargo-octet versions, and receipts with uncovered critical findings.
8. Emit a validation receipt with deterministic diagnostics and check labels.

## Consumer Integration

### Node runtime startup

`node-startup-receipt-v1` must include a source-gate validation ref for the daemon/adapters being started. Startup denies before starting production adapters if the validation decision is not `pass`.

### Remote job admission

`job-admission-plan-v1`/`job-admission-receipt-v1` must include source-gate validation refs for executor/job DAG surfaces required by the target. Admission denies before claiming executable-artifact readiness if validation fails.

### Upgrade planning

`upgrade-plan-v1`/upgrade task receipts must include source-gate validation refs for upgrade code and affected executor surfaces. Plans that would move names, migrate storage, or schedule irreversible tasks deny unless the strict source gate validates.

## Denial Cases

Consumers MUST deny before side effects when:

- the source-gate ref is missing, unreadable, or not an `octet-gate-receipt-v1`;
- the gate receipt decision is `deny`, `warning-only`, or unknown;
- the receipt is for a quarantine/advisory profile rather than `strict-ci`;
- workspace metadata, Octet config hash, profile hash, command scope, pass-through args, or toolchain evidence is stale;
- object-corpus or fingerprint refs are missing, malformed, or do not cover required source scopes;
- structured findings indicate uncovered critical findings;
- diagnostics/checks were tampered to claim pass without matching refs;
- consumer receipt tries to bind only raw Octet artifacts, raw summaries, or process exit status.

## Tests

- Pass validation with a clean strict Octet gate receipt fixture.
- Deny validation for a denied or warning-only Octet gate receipt.
- Deny validation for stale config/profile hash after changing `[workspace.metadata.octet]`-derived inputs.
- Deny validation for quarantine profile receipts in node/job/upgrade consumers.
- Deny validation for tampered object-corpus or fingerprint refs.
- Deny node startup before adapters start when source-gate validation fails.
- Deny remote job admission before executable readiness when source-gate validation fails.
- Deny upgrade planning before name moves/migrations when source-gate validation fails.
