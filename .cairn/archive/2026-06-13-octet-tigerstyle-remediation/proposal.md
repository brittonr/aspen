## Why

The first Octet/TigerStyle evidence run identified the shape of the debt blocking strict fail-close: oversized shell files, long functions, too many positional parameters, unbounded collection growth, raw/stringly refs, and critical caveats such as panic/unwrap/time/resource-shape findings. Later slices reached a configuration-clean strict gate while documenting disabled lint families and remaining source-remediated-zero work.

Strict fail-close requires source surfaces to be shaped so the gate can pass without hiding warnings. This archived change records the remediation lane and current evidence boundary: functional core/imperative shell, bounded resources, typed identities, input structs instead of long argument lists, fail-fast receipt-backed errors, and explicit caveats for future module splits/source burn-down.

## What Changes

- Record large imperative shell module splits (`src/main.rs`, `src/job_dag.rs`) as future source-remediated-zero work while keeping current configuration-clean caveats visible.
- Refactor high-arity builders and receipt constructors into input structs with validated typed refs and explicit invariants where recent critical surfaces were touched.
- Add deterministic bounds or checkpoints for collection growth in job DAG, node runtime, harness/source gate, catalog, and adapter paths.
- Remove, deny, or require review for panic/unwrap/expect, ambient clock, unchecked narrowing/division, sentinel fallbacks, and unbounded loops from critical evidence-bearing surfaces.
- Replace raw `String`/generic hash parameters at public evidence boundaries with typed newtypes or validated ref structs where feasible, and keep remaining CLI/config edges explicit.
- Track remediation by Octet gate receipts, remediation plans, and object corpus/fingerprint evidence rather than informal code review.

## Impact

This is the burn-down lane that gets Molten from warning-only Octet runs to strict fail-close. The current archive preserves the distinction between source-remediated zero and configuration-clean strict passes with documented disabled-lint caveats. Every future remediation task should reduce findings, improve evidence quality, and move critical runtime/admission/harness/job surfaces toward Tiger Style constraints without suppressing tests or hiding debt.
