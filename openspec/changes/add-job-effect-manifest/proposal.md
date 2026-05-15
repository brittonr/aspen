## Why

Unison's abilities make effects explicit: Remote, Storage, Config, Log, Http, and so on. Aspen has equivalent security/product boundaries spread across UCAN capabilities, job payloads, workers, sandbox policy, secrets, logs, blob/KV access, network access, and receipts.

Aspen should define an explicit job/service effect manifest so admission, sandboxing, UCAN authorization, and receipts all use the same declared effect set instead of executor-specific implicit policy.

## What Changes

- Define a versioned effect manifest for jobs/services/closures, e.g. remote execution, blob read/write, KV read/write, secret read, Forge read/write, network outbound, log write, and runtime-host access.
- Require deny-by-default admission against granted capabilities before execution.
- Map effect manifest entries to worker sandbox policy and receipt redaction rules.
- Add negative tests for undeclared effect use and denied requested effects.

## In Scope

- Effect taxonomy and manifest schema.
- Admission mapping to existing UCAN/capability policy.
- One executor slice enforcing the manifest.
- Receipt and redaction requirements.

## Out of Scope

- A general-purpose language effect system.
- Full eBPF/syscall enforcement in this slice unless already supported by the chosen runtime.
- Broad executor migration before the first enforced slice is proven.

## Verification

- `openspec validate add-job-effect-manifest --strict`
- Focused manifest validation and capability-mapping tests.
- Product-path executor test proving declared effects allowed and denied/undeclared effects rejected.
- Secret redaction tests.
- `openspec validate --all --strict --json`
- `git diff --check`
