# Inspecting dead-letter and redrive evidence

Mode: How-to

## Goal and prerequisites

Build an evidence packet that distinguishes a dead-lettered item, an admissible redrive plan, and a confirmed redrive commit. This procedure does not issue redrive or cleanup operations. You need the relevant published queue state, request and receipt, admitted policy and manifest, logical-time evidence, and any timer/status observations supplied by the integration owning those ports. Without those inputs, record the missing boundary rather than reconstructing authority from a receipt.

Return to the [Handbook](../README.md). The [coordination contract](../../coordination-delivery.md) governs the operation; the [dead-letter companion](../../technical/replication/dead-letter-redrive-and-recovery.md) explains its mechanics. This guide is source-checked and not runtime-verified. The inspected [delivery CLI](../../../src/cli/workflow/delivery/command.rs) exposes idempotency diagnostics, not a dead-letter inspection or redrive command. Do not substitute its store or `check` operation for this evidence.

## 1. Decide whether you have item evidence or only a count

Start with queue identity, policy reference, state reference, and revision. The [status record](../../../src/coordination_delivery/records.rs) includes ready, retry, in-flight, dead-letter, completed, and failed-attempt counts. It also carries a `truncated` field and a bounded active-claim projection. A dead-letter count identifies a population, not the selected item's history.

Obtain the selected entry from the published state's `dead_letter` map through the owning integration. Match its `item.item_ref`, content and metadata references, entry tick, cycle, attempts in that cycle, total attempts, and reason. Keep referenced payload access separately authorized; status deliberately does not render payload bytes.

Stop if only a screenshot or aggregate count is available. Such evidence cannot identify which item is proposed for re-entry.

## 2. Reconstruct why this item entered the DLQ

Join the selected item to its `attempts` history using item reference. Each [attempt record](../../../crates/molten-core/src/coordination_delivery/model/state.rs) carries delivery identity, consumer, attempt, cycle, outcome, operation identifier, and observation tick. Compare this history with the request that caused dead-lettering and its commit outcome.

The [completion transitions](../../../crates/molten-core/src/coordination_delivery/transition/completion.rs) distinguish poison or exhausted nack from lease expiry at the attempt limit. Unsupported failure classes deny the transition. A full dead-letter collection also denies relocation; a failed attempt is therefore not sufficient evidence that the item actually entered the DLQ.

For a preserving denial, retain the original in-flight state in the packet. Do not relabel the item quarantined because an intermediate helper tried to remove it: the enclosing planner returns the original state on failure.

## 3. Separate eligibility from remediation

For a proposed redrive, check the request against the admitted policy's exact redrive authority reference and inspect ready capacity. Confirm the request's currentness, generation, epoch, logical time, and expected published state through the normal admission path. A receipt, retention timer, or worker claim is not replacement authority.

Independently request evidence addressing the original failure cause. Redrive does not validate payload meaning or certify repair of poison work. If a worker's earlier external effect is uncertain, pause the retry decision and apply the [uncertain-outcome procedure](diagnosing-uncertain-delivery-outcomes.md). Having authority to requeue does not settle whether repeating the worker's action is safe.

The checked-in [profile](../../../config/coordination-delivery/profile.ncl) declares capacity eight, maximum attempts two, and twenty logical ticks of dead-letter retention. Treat these as that reviewed profile's values, not universal production settings. Its repeated-character binding references are fixture/configuration material, not authority credentials to copy.

## 4. Compare before and planned-after states

An admitted [redrive](../../../crates/molten-core/src/coordination_delivery/transition/retention.rs) removes the dead-letter entry, increments its cycle, assigns a fresh enqueue sequence, resets attempts within the new cycle to zero, and sets eligibility to the request's logical tick. It retains prior attempt history and plans cancellation of the old retention timer.

Worked inspection example: suppose supplied evidence shows one poison attempt in cycle one, a confirmed dead-letter entry, and an authorized redrive plan. The packet should show that same item ready in cycle two with zero new-cycle attempts while its earlier poison attempt remains in history. It should not show an erased failure, a completed worker, or a new claim token. This is an illustrative comparison over the source rules, not a reported execution.

## 5. Classify commit and follow-up observations separately

Compare the receipt with the published state and [shell reconciliation rules](../../../src/coordination_delivery/service.rs). An unknown commit is read back once: exact planned state means applied after reconciliation; exact expected state means not applied; another state remains unknown. Retain currentness, durability, and engine epoch alongside that classification.

Only after confirmed commit does the shell attempt timer and status work. A failed retention-timer cancellation does not undo the redrive. Record its timer reference as unresolved follow-up evidence rather than requesting another cycle. Keep the full service observations when available because receipt fields do not encode every observation Boolean.

Stop at unresolved commit or external-effect uncertainty. The completed packet should name the item, its causal history, authority source, state comparison, commit classification, and remaining follow-up issue. Cleanup is separately authorized and removes DLQ entries, not referenced content or attempt history; it is not a repair for uncertainty.

## Sources

- [Handbook](../README.md)
- [Coordination delivery contract](../../coordination-delivery.md)
- [Dead-letter and recovery companion](../../technical/replication/dead-letter-redrive-and-recovery.md)
- [State and attempt fields](../../../crates/molten-core/src/coordination_delivery/model/state.rs)
- [Redrive and cleanup implementation](../../../crates/molten-core/src/coordination_delivery/transition/retention.rs)
- [Commit shell and reconciliation](../../../src/coordination_delivery/service.rs)
