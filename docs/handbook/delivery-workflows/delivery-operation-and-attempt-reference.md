# Delivery operation and attempt reference

Mode: Reference

Use this page when joining delivery artifacts in an incident or review. Two implementations share delivery vocabulary but own different facts: the idempotency diagnostic identifies scoped operations and sequence decisions; coordination delivery records queue transitions, claims, attempts, and redrive. Neither record family is worker execution authority. Return to the [Handbook](../README.md); consult the [claims companion](../../technical/replication/delivery-claims-acks-and-retry.md) for explanatory theory.

All entries below are source-checked, not executed verification. Field names in code use underscores; the listed canonical record fields use their encoded spelling. Canonical Preserves plus BLAKE3 framing define identity, not the in-memory Rust layout.

## Diagnostic command surface

The command prefix is `molten test delivery`, declared by the [root command include](../../../src/main/root/parts/command/p000/body.rs) and routed through [main aliases](../../../src/main.rs) to the [delivery declarations](../../../src/cli/workflow/delivery/command.rs). These are reference spellings, not instructions to mutate an incident store.

| Subcommand | Relevant inputs | Owner and effect |
|---|---|---|
| `scope` | `--scope-profile`, `--scope-name`, repeated `--retention-ref`, optional `--out` | Builds a scope-profile artifact |
| `operation-id` | Profile; name or reference; producer, consumer, sequence, intent, payload; repeated policy references | Builds an operation artifact; no delivery-store check |
| `check` | Above bindings plus `--root`, repeated `--evidence-ref`, optional `--semantic-result-ref`, `--gap-policy`, `--receipt-out` | Stateful dedup check; persists diagnostic records |
| `receipt-show` | Positional receipt reference and `--root` | Looks up a stored diagnostic receipt |
| `show` | Positional artifact file | Parses a file and prints a diagnostic summary |

In the [handlers](../../../src/cli/workflow/delivery/ops.rs), explicit `--scope-ref` wins if a name is also supplied. Gap policy accepts `deny` or `retry`, defaulting to `deny`. Supported profiles are `actor-turn`, `service-lifecycle`, `protocol-session`, `remote-dataspace-topic`, `job-worker`, and `control-plane-command`.

Do not assume `receipt-show` is a filesystem-neutral probe: its library path opens/ensures the diagnostic database tables. File `show` is the narrower choice for an already exported artifact. Neither command is a coordination-state browser.

## Diagnostic identities and records

The [idempotency definitions](../../../src/delivery/parts/idempotency/p000/body.rs) and [store framing](../../../src/delivery/parts/idempotency/p002/body.rs) own these values.

| Record or identity | Material bindings | Interpretation |
|---|---|---|
| `operation-id-v1` | `scope`, `producer`, `consumer`, `sequence`, `intent`, `payload`, `policy` | Identity of the proposed scoped operation |
| `dedup-key-v1` | Scope, producer, consumer, sequence, intent | Lookup key; deliberately excludes payload and evidence |
| `delivery-window-v1` | `scope`, `profile`, `next-sequence`, `lowest-retained`, `retention` | Scope-wide sequence tracking |
| `dedup-entry-v1` | Operation bindings, `first-receipt`, optional `semantic-result`, `evidence` | Persisted first-operation comparison material |
| `delivery-idempotency-receipt-v1` | `decision`, `operation`, `scope`, `window`, `prior`, `semantic-result`, `side-effect`, `diagnostics`, `checks` | Diagnostic decision, not proof an effect ran |
| `retry-receipt-v1` | Operation, scope, window, `retry-after-sequence`, diagnostics | Gap-related retry evidence, not wall-clock scheduling |

`delivery-idempotency.redb` contains `delivery_windows_v1`, `delivery_dedup_entries_v1`, `delivery_idempotency_receipts_v1`, and `delivery_retention_pins_v1`. These are implementation-owned tables, not an invitation to edit storage directly.

The decision set is `first`, `duplicate`, `conflict`, `stale`, `gap`, and `retry`. Only `first` returns permission from this diagnostic law to commit a side effect; all other decisions suppress it. Actual effect admission remains separate.

## Coordination identities and attempts

The [state model](../../../crates/molten-core/src/coordination_delivery/model/state.rs) owns the following fields.

| Value | Fields to retain in an evidence join | Common mistaken substitution |
|---|---|---|
| Item | `item_ref`, `content_ref`, `metadata_ref`, `enqueue_sequence`, `policy_ref` | Payload bytes are not embedded work authority |
| Token | `token_ref`, `delivery_id`, queue/item/consumer, attempt, cycle, fencing token, claim/deadline ticks, epoch, generation, policy | Item reference alone cannot complete a claim |
| Attempt | `delivery_id`, `item_ref`, `consumer_id`, `attempt`, `cycle`, `outcome`, `operation_id`, `observed_at_tick` | Attempt number is not lifetime attempt history |
| Dead letter | Item, entry tick, cycle, attempts in cycle, total attempts, reason | Membership does not certify a repaired payload |
| Applied operation | `request_ref`, `operation_ref`, `operation_kind`, optional item/token references | Request replay is not external-effect replay |

A redrive starts another cycle and keeps the attempt history. Consequently `(item_ref, cycle, attempt)` is more informative than attempt number alone, while exact token equality still controls completion.

## Commit receipt interpretation

The [coordination receipt](../../../src/coordination_delivery/records.rs) binds queue, request, operation, before/after state, revision, status, currentness, durability, engine epoch, timer references, failed timer references, optional status reference, and issue. Its status strings are `denied`, `duplicate-replay`, `applied`, `already-applied`, `applied-after-reconciliation`, `not-applied-after-reconciliation`, `stale`, and `unknown`.

Worked failure case: a receipt can name a planned `after-state-ref` while its status is `unknown` or `not-applied-after-reconciliation`. The [builder](../../../src/coordination_delivery/service.rs) takes that field from the transition, not a fresh authoritative final-state proof. Read status and storage observation together before saying that state was published. Likewise, a confirmed queue commit with failed timer references is not a failed queue commit.

## Sources

- [Handbook](../README.md)
- [Coordination delivery contract](../../coordination-delivery.md)
- [Claims and retry companion](../../technical/replication/delivery-claims-acks-and-retry.md)
- [CLI declaration](../../../src/cli/workflow/delivery/command.rs)
- [Diagnostic receipt parsing](../../../src/delivery/parts/idempotency/p001/body.rs)
- [Coordination receipt encoding](../../../src/coordination_delivery/records.rs)
