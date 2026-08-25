# Validation evidence

## Scope

This change binds release-tier profile validation to one candidate content reference. It exposes the pure validator through `molten test gate release-profile` and a Nix conformance check.

Base source commit: `e51b5f5f1ebdaa04e1790e731ecd0872b3b4f245`.

## Core and shell evidence

The pre-change focused baseline passed five tests.

After the change, six focused release-profile tests passed. They include valid development, pilot, and release inputs plus missing, malformed, stale, and placeholder denial cases.

Two focused CLI tests passed. The positive test emitted a candidate-bound canonical pass value. The negative test wrote a canonical deny value before the process returned an error for a missing candidate.

`cargo fmt --all --check` passed.

`cargo clippy --workspace --all-targets -- -D warnings` passed. The evidence gate command enum has one narrow `large_enum_variant` allowance because Clap owns the complete operator input shape.

## Nix evidence

`nix build .#checks.x86_64-linux.release-profile-validation -L --option builders "" --no-link` passed.

The focused pass receipt is `blake3:d54663d8d304af6d5f486ead64a51c06d74e1ab6320f62b2c6e9864dfe5f33c6`.
The missing-candidate negative receipt is `blake3:6c0498ca351e3817c49603bc14dede0ecc22c5df9718c8f0ffa71f31aba38203`.
The placeholder-source negative receipt is `blake3:1da41a989db85a9c3f51f5536b7e53275bcd07d5174f44a8a46891cf2f615f79`.

`nix build .#checks.x86_64-linux.molten -L --option builders "" --no-link` passed.
The Nix nextest rail ran 1,414 tests with no failures or skips.
Its CI receipt is `blake3:07a363ec714438ebdf999afd3a87e6baab1b2072b99bd5867dbb481552c0494a`.

## Octet evidence

The base source produced 5,856 warning-level findings with the current local Octet cohort.
The changed source produced 5,838 warning-level findings.
The change did not increase the finding count.

This warning-only result is not strict Octet acceptance evidence. The repository still requires its strict source gate for candidate release evidence.

## Lifecycle evidence

Strict Cairn validation passed before lifecycle sync.

Final gate receipts before sync:

- proposal: `39be61d688a18b56bddc6aa6f4779c9571562027be61ce2904a65da7138acc14`
- design: `bba441cd7cb2431b11a0c22d760fb6e5afb1ad9737a302e74cf03e3c6d0f1fbc`
- tasks: `dde46e92c74f800ca94690c328d2bd396de43abcfbacc7ff7826192fee458968`

The sync dry-run passed with plan `a41c8ea7e5ef51b9ce51fad6df97a88b0d5379a156a395e029ea3707b57413ca`.
The executed sync added three requirements to the accepted node-runtime and operator-workflow specifications.
The sync receipt is `89c214110dd447d8e70ef1ff804d189313469757e3a12f5a0f7760afc53079a9`.
Strict Cairn validation passed after sync.

The archive dry-run passed with plan `c23aa22cb593f5b25dcb38768b74449b233e11b2c40067b586b3615dfc6bf5eb`.
Archive execution moved the package to `2026-08-25-add-candidate-bound-release-profile-gate`.
The archive receipt is `97474df965bb4a77e7b55608bdec45c4afb9e5cd1a910b5a07e6f3efba76709d`.
Strict Cairn validation passed after archive execution.

## Non-claims

The Nix fixture uses deterministic non-placeholder test references. It proves validator and command wiring only.

A passing profile validation does not prove source-gate success, evidence truth, artifact freshness outside the supplied refs, deployment success, runtime authority, or release eligibility.
