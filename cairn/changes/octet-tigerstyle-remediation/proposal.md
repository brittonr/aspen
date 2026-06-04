## Why

The first Octet/TigerStyle evidence run identified the shape of the debt blocking strict fail-close: oversized shell files, long functions, too many positional parameters, unbounded collection growth, raw/stringly refs, and critical caveats such as panic/unwrap/time/resource-shape findings. The largest immediate hotspots include `src/main.rs`, `src/job_dag.rs`, and the new `src/node_runtime.rs` path.

Strict fail-close requires source surfaces to be shaped so the gate can pass without hiding warnings. This change scopes the remediation work needed to make Octet strict mode realistic quickly, with Tiger Style as the coding discipline: functional core/imperative shell, bounded resources, typed identities, short functions, input structs instead of long argument lists, and fail-fast receipt-backed errors.

## What Changes

- Split large imperative shell modules so `src/main.rs` dispatches to smaller command modules and pure parsing/evaluation helpers.
- Refactor high-arity builders and receipt constructors into input structs with validated typed refs and explicit invariants.
- Add deterministic bounds or checkpoints for collection growth in job DAG, node runtime, harness/source gate, catalog, and adapter paths.
- Remove or review panic/unwrap/expect, ambient clock, unchecked narrowing/division, sentinel fallbacks, and unbounded loops from critical evidence-bearing surfaces.
- Replace raw `String`/generic hash parameters at public evidence boundaries with typed newtypes or validated ref structs where feasible.
- Track remediation by Octet gate receipts and object corpus/fingerprint evidence rather than informal code review.

## Impact

This is the burn-down lane that gets Molten from warning-only Octet runs to strict fail-close. It keeps the goal concrete: every remediation task should reduce findings, improve evidence quality, and move critical runtime/admission/harness/job surfaces toward Tiger Style constraints without suppressing tests or hiding debt.
