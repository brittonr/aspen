# Reviewing performance and readiness claims

Mode: Review checklist

Use this checklist when a change description, benchmark report, or release packet claims improved performance or readiness. Its output is a scoped review disposition with linked evidence and explicit blockers, not a new gate schema. Every checked item should identify actual evidence or an explicit exemption allowed by the governing workflow.

This checklist is source-checked and was not executed against a candidate for this documentation batch. It follows the [development profiling contract](../../development-function-profiling.md), [proof workflow](../../proof-workflow.md), and [readiness companion](../../technical/proof/release-readiness-and-proof-scope.md). The [profiling companion](../../technical/engineering/profiling-without-semantic-authority.md) supplies theory; the questions below support a concrete review decision.

## Claim and comparison

- [ ] Is the claimed improvement stated as a measurable property of a named workload, build, platform, and candidate, rather than “faster” or “production-ready”? Identify the exact before/after subjects.
- [ ] Does the evidence distinguish computation, waiting, startup, and instrumentation overhead where those distinctions affect the conclusion? A long shell interval is not automatically pure-core cost.
- [ ] Is there repeatable benchmark or test evidence for the performance claim, rather than only an attractive trace? Record the comparison procedure and observed variance without inventing a universal threshold.
- [ ] Are any production thresholds taken from the reviewed production profile rather than inferred from one local run? Keep queue, receipt/store size, latency, and recovery claims distinct.

**Acceptance evidence:** a reviewer can reproduce the described comparison from real inputs and understand which conclusion would change if the platform, feature set, or workload changed. Missing baseline provenance prevents a comparative claim even if the new run itself is well documented.

## Profiling configuration and role

- [ ] Was profiling enabled on its supported `x86_64-linux` development target, and is the actual feature set recorded? Other-target disabled annotations are not evidence of a zero-cost workload.
- [ ] If `profiler-perf` was selected, is host counter permission accounted for? The governing guide treats missing permission as an error, not a silently absent counter.
- [ ] If `profiler-alloc` was selected, does the review identify the upstream `CountingAllocator` configuration instead of projecting its observations onto the default allocator?
- [ ] Was capture explicitly bounded in duration or memory? Retain capture configuration in development notes without submitting the trace as release evidence.
- [ ] Are new annotations confined to the runtime shell, with no profiling side effects introduced into `molten-core` or `aspen-core`?
- [ ] Is the `.fxt` artifact classified only as a development observation, excluded from Cairn receipts, Valence evidence, determinism claims, and release-readiness inputs?

The [artifact-role helper](../../../src/profiling.rs) checks the `.fxt` extension and requested enum role. It does not parse trace contents or enforce every possible artifact-upload path. A review must therefore inspect actual evidence handling; helper tests are not a repository-wide guarantee.

## Candidate and evidence truth

- [ ] Do the profile, source gate, policy, Octet, Cairn, stack provenance, and production profile refer to the intended candidate and scope? Inspect the underlying material, not just digest shape.
- [ ] Are expected and actual generated-export refs independently justified before equality is accepted as freshness?
- [ ] Does every required candidate-binding group have evidence for this source: Rust, nextest, Nix, Cairn, Octet, dogfood, bundle verification, promotion, export verification, and pilot decision?
- [ ] Are canonical Preserves identities distinguished from Rust representation and terminal rendering? The dependency report is a separate textual-material hash and must not be misrepresented as a Preserves receipt.
- [ ] Are conformance references visibly marked as fixtures instead of reused as actual candidate evidence?

The candidate builder records that declared binding does not prove external artifact truth. Passing the profile helper also does not execute the evidence producers. A reviewer must be able to explain where actual execution was established and where only a supplied-field check occurred.

## Proof scope and denial evidence

- [ ] Does each readiness claim retain its execution scope: deterministic model, local process, VM, live observation, or diagnostic-only? Unavailable platform evidence remains unavailable.
- [ ] Are positive and expected-deny results both present where the proof workflow requires them, with appropriate candidate/subject binding?
- [ ] For changed mutation boundaries, is denial-before-effect supported by unchanged-state references or a no-mutation receipt, rather than inferred from a log line?
- [ ] Are retries or exploratory reruns excluded from deterministic release-pass interpretation? “Eventually passed” is a different observation.
- [ ] Do caveats preserve limitations without granting authority, transport trust, retention clearance, or promotion permission?

## Worked rejection and narrower acceptance

A proposed review packet contains a `profilerprobe` trace and a passing release-profile fixture, and describes the candidate as meeting production latency requirements. Reject that conclusion. The checked probe performs wrapping multiplication behind `black_box`, pauses one millisecond between frames, and runs for three seconds. It is an instrumentation probe, not the production workload. The fixture exercises supplied-reference validation, not production deployment.

A narrower statement may be supportable only with observed evidence: an executed, bounded probe demonstrated useful local instrumentation under its recorded configuration. That statement still cannot include an unexecuted capture. Request workload-specific repeatable measurements and actual candidate-scoped readiness evidence before accepting the stronger claim; do not turn the diagnostic trace into a canonical receipt.

## Review disposition

Record accepted claims, rejected or unsupported claims, evidence references, execution limits, and the next required evidence-producing operation. Stop approval of a claim when its candidate identity, workload, execution scope, or authority boundary cannot be established. Other well-supported claims may remain valid, but their existence does not erase the blocker.

## Sources

- [Handbook](../README.md)
- [Development function profiling](../../development-function-profiling.md)
- [Proof workflow](../../proof-workflow.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Profiling companion](../../technical/engineering/profiling-without-semantic-authority.md)
- [Readiness companion](../../technical/proof/release-readiness-and-proof-scope.md)
- [Feature and platform configuration](../../../Cargo.toml)
- [Profiler role gate and allocator](../../../src/profiling.rs)
- [Bounded probe implementation](../../../examples/profilerprobe.rs)
- [Candidate binding and non-claims](../../../src/prod/parts/readiness/p002/body.rs)
