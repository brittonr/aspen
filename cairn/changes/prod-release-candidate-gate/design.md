## Context

Molten already emits Nix dogfood release evidence, release bundle verification receipts, promotion gates, and Octet source-gate receipts. The current production-readiness gap is orchestration and freshness: the final candidate must prove that those artifacts correspond to the same source tree, same Nix inputs, same active source-gate policy, and same intended pilot scope.

The active `octet-source-remediated-zero` change is not duplicated here. This change treats that work as an input: either the source-remediated-zero caveat is resolved, or the release-candidate receipt must name the remaining configuration-clean caveat and deny broad production promotion.

## Design

### Release-candidate receipt

Add a canonical `prod-release-candidate-gate-v1` receipt that binds:

- source tree/ref or source-gate artifact refs;
- Octet artifact import, strict gate, remediation-plan, and object-corpus/fingerprint refs;
- hermetic nextest/Nix check output refs;
- `dogfood-local-node` output path and verification refs;
- release evidence bundle verification refs;
- promotion gate, signed promotion, promotion summary, and export verification refs where present;
- explicit pilot-scope limits and caveats.

The receipt decision is `pass` only when all required current evidence is passing, fresh, and mutually bound. Missing, stale, mismatched, denied, or configuration-clean-but-broad-promotion evidence emits a deny receipt with diagnostics.

### Pilot-scope decision

A pass receipt is still scoped. It must identify:

- allowed pilot workloads;
- disallowed destructive or customer-critical workloads;
- rollback and stop-the-line conditions;
- required operator review artifacts;
- known caveats such as disabled lint-family burn-down or live-distributed-soak gaps.

### Non-goals

- Do not grant subsystem authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust.
- Do not replace active source-remediated-zero work.
- Do not require broad customer production readiness before enabling a constrained internal pilot.
