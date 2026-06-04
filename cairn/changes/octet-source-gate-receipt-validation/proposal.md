## Why

Node startup, remote job admission, and upgrade planning now carry strict Octet source-gate refs, but a ref or label alone is not pass evidence. A downstream receipt could still point at a denied Octet gate, a stale receipt from an older `Cargo.toml`/Octet profile, a warning-only run, or a receipt whose object-corpus/fingerprint evidence no longer matches the current workspace. That would recreate the exact gap the Octet gate was meant to close: evidence exists, but consumers do not validate the evidence before allowing side effects.

Molten needs a shared, fail-closed source-gate validator that parses the actual `octet-gate-receipt-v1` content, recomputes the required current workspace/profile evidence, and emits canonical validation evidence for each consumer before node runtime startup, remote job admission, or upgrade planning can pass.

## What Changes

- Add a canonical downstream source-gate requirement/validation model for strict Octet receipts.
- Validate referenced `octet-gate-receipt-v1` artifacts by content, not by ref shape or check label alone.
- Require decision `pass`, profile `strict-ci`, supported tool/config/profile hashes, current workspace metadata, object-corpus evidence, fingerprint evidence, and expected source scope bindings.
- Deny warning-only, denied, quarantine-only, stale, malformed, unsupported, missing, or tampered Octet gate receipts before any downstream side effect.
- Wire the validator into node runtime startup, remote job admission, and upgrade planning receipts, with consumer receipts binding the source-gate validation receipt refs.
- Preserve diagnostic refs for rejected Octet evidence without allowing raw Octet artifacts or raw summaries to stand in for pass receipts.

## Impact

This closes the downstream enforcement gap for source-shape evidence. Node/runtime, job admission, and upgrade paths will be able to claim `strict-octet-source-gate-bound` only when a current strict pass receipt was parsed and validated against the live source/config/profile evidence. Release evidence remains a follow-up path, but it should reuse the same validator rather than inventing another source-gate rule set.
