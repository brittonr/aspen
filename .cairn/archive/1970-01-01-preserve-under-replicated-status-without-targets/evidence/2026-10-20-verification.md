# F07 executed verification — 2026-10-20

## Outcome, capability, and owner

The content-replication maintainers own the planner, status projection, shell receipts, adapters, and operator contract. The immediate correction keeps unmet demand visible when there is no eligible target. The durable capability is per-content missing-count accounting independent of action presence or aggregate successes.

The retained pre-edit public counterexample lost the action-free deficit. The corrected public smoke printed:

```text
F07 decision=partial verified=1 unresolved=1
```

The same temporary program exercised the public shell with production deterministic and live-Iroh-loopback transport adapters plus simulated durability and controlled observation/content ports. Both profiles completed a distinct repair attempt 2, then emitted an action-free partial receipt with one remaining deficit and no third transfer. Exact output, canonical receipt ref, baseline counts, commands, common gates, and failed attempts are in the [shared executed record](../../1970-01-01-repair-lost-replicas-after-prior-success/evidence/2026-10-20-verification.md). The harness was removed after execution. This is scoped shell/loopback evidence, not arbitrary live-cluster convergence.

## Permanent controls

`crates/molten-core/src/content_replication/tests/deficits.rs` covers:

- the audit's insufficient-peer/no-action deficit with exact partial decision and cause;
- satisfied inventory with no executable actions;
- a selected successful subset that cannot fill all missing demand;
- two content refs, where another object's success cannot clear a residual deficit;
- mismatched or duplicate terminal evidence that supplies no success;
- bounded combinations of desired/current/available counts and successful selected targets;
- different action-free missing counts producing different plan identities.

`src/content_replication/tests/deficits.rs` covers real shell classification for no-target and partial-target outcomes, historical-success/fresh-loss repair, unavailable transport, rejected verification and stale envelopes, denied planning input, and exhausted attempts. Rejection retains prior inventory and durable history/status/receipt counts; no successful operation or completion is published. Complete/partial canonical statuses round-trip through strict packed Preserves decoding. Truncated bytes and invalid status bindings, counts, refs, bounds, or non-claims reject.

F04 exact saved-operation controls and F05 deferred/rejected cleanup controls run in the same normal suites without merging the requirement packages. A ready plan and empty deficit/active/failure lists are all required for completion. Deferred cleanup cannot become complete merely because replica counts are sufficient.

## Observed gates and compatibility

- Initial post-fix focused runs passed 33 core and 28 shell/conformance-filtered tests, zero failures/ignored cases; subsequent required policy corrections changed only layout, names, and qualified imports.
- The hermetic `world-distribution-octet-deny-all` check then passed clean with zero findings/warnings/errors. Final-tree formatting, unchanged inherited-debt accounting, separate planning gates, and strict owner-policy Cairn validation passed as recorded in the shared evidence.
- Final configured workspace/library Octet producers were clean and the canonical `strict-ci` source gate passed. Default workspace tests passed 2,219 cases; the compatible-feature all-targets run passed 2,231 cases with only the external DoltLite oracle ignored. Strict workspace/all-targets Clippy passed for the same feature set. The shared record retains exact commands and receipts; checked implementation tasks do not grant sync/archive authority.
- The direct focused Octet wrapper failed on its escaping diagnostic path; the shared record preserves that failure and does not claim a passing local focused lint receipt.

The [compatibility decision](../design.md#implementation-compatibility-decision-2026-10-20) retains status and receipt v1 shapes. In-memory plans add ordered `ReplicaDeficit` entries; new plan records and their identity context are explicitly v2. Historical canonical bytes are not rewritten or promoted into current convergence evidence.

No production durability, global availability, VM behavior, lifecycle completion, commit/push, or release authority is claimed.
