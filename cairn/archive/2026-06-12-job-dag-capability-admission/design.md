## Context

Target-side job admission verifies synced artifacts, topology, executable stage artifacts, and resource envelopes without executing. Capabilities must now be real target authority contexts, not opaque placeholder refs.

## Goals

- Reuse the existing `authority` module and `admit_authority` path.
- Treat job admission capability refs as canonical `authority-context-v1` payload refs stored in target registry artifacts.
- Admit `job:execute` scoped to the job ref.
- Record authority admission receipt refs in job admission plans and receipts.
- Deny when no authority context admits the requested job execution.

## Non-Goals

- No remote execution yet.
- No UCAN proofset expansion in this slice.
- No network transport or peer-to-peer authority exchange.
- No authority from synced artifact possession alone.

## Admission Rule

For a `job-admission-request-v1`:

1. Every capability ref remains a canonical blake3 ref.
2. The target registry is searched for artifacts whose payload parses as `authority-context-v1` and whose context ref equals a requested capability ref.
3. Each matching context is passed to `admit_authority(context, "job:execute", job-ref, logical-time=0, revocations=[])`.
4. At least one authority admission must pass.
5. Authority admission receipt refs are added to `job-admission-plan-v1` and `job-admission-receipt-v1` refs/checks.

## Denials

Admission denies if:

- capability refs are absent;
- capability refs do not resolve to target registry authority contexts;
- authority context capability name is not `job:execute`;
- authority context scope does not match the job ref or `*`;
- authority context is expired, not yet valid, revoked, or attenuated to deny;
- sync evidence is present but authority context is missing.

## CLI

The existing `admit-plan` and `admit-loopback` commands accept `--capability-ref`. Passing a real target authority-context ref can admit; generated placeholder refs remain denial evidence.

## Tests

- Admission passes after sync when the target registry contains an authority context with `job:execute` for the job ref.
- Admission denies placeholder capability refs that do not resolve to authority contexts.
- Admission receipts bind authority admission receipt refs and `capability-authority-context` checks.
