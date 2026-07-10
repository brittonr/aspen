## Context

The accepted `node-runtime` requirements already define production deployment profiles and require startup receipts to bind profile metadata. Today, however, the concrete node lifecycle path still builds a local `node-config-v1` from Rust-side defaults. That is useful for fixtures, but it leaves a gap between reviewed profile artifacts and the durable daemon configuration operators actually start.

## Decisions

### Runtime consumes checked profile exports only

**Choice:** Profile-backed startup accepts checked exported JSON/Preserves profile artifacts and profile refs. It does not evaluate Nickel during node startup, `run`, or `serve`.

**Rationale:** Nickel should catch authoring mistakes before review. Startup should remain deterministic and limited to parsing canonical artifacts, checking refs, and emitting receipts.

### Profile resolution has a pure core

**Choice:** Add a pure profile-resolution core that takes an exported profile value, optional CLI override values, and explicit evidence refs, then returns an effective node configuration plus diagnostics.

**Rationale:** The safety decisions can be tested without filesystem state, environment variables, clocks, network, or a running daemon. CLI handlers stay as thin shells that read files and write receipts.

### CLI overrides are explicit and receipt-bound

**Choice:** CLI flags may override only fields marked overrideable by the profile tier. Each override is recorded in the effective-config/startup receipts and denied when it would weaken a required profile invariant.

**Rationale:** Operators need practical local flexibility, but production review needs to see when a command diverged from the reviewed profile.

### Local defaults remain local-fixture defaults

**Choice:** If no profile is supplied, existing `node init` behavior remains available for local fixtures, but startup receipts carry a caveat such as `local-fixture-config` and cannot satisfy release-tier profile requirements.

**Rationale:** This avoids breaking development workflows while preventing fixture defaults from looking production-ready.

## Validation strategy

- Positive unit tests for profile export → effective node config → startup receipt binding.
- Negative tests for missing profile refs, malformed metadata, unsupported adapter names, stale profile content refs, and denied production overrides.
- CLI integration tests for `node init --profile` and `node run --profile-ref`.
- Focused Cairn gates for proposal, design, tasks, and the `node-runtime` spec delta.

## Non-claims

Profile-backed configuration does not prove adapter implementation correctness, source-code correctness, release eligibility, live transport correctness, or runtime authority. Those remain separate receipts and gates.
