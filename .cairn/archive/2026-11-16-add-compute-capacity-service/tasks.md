## Phase 1: Pure core

- [x] [serial] Model bounded versioned offers, compatibility, and physical pool/view relationships in `molten-core` r[molten.compute_capacity.offers] r[molten.compute_capacity.backing_pools].
- [x] [serial] Implement atomic pure reservation, idempotency, uncertainty, release and readback decisions r[molten.compute_capacity.reservation_authority] r[molten.compute_capacity.uncertainty_release] r[molten.compute_capacity.readback].
- [x] [serial] Run focused positive/negative two-host and synthetic TT overlap tests r[molten.compute_capacity.fixtures].

## Phase 2: Admitted service and evidence

- [x] [serial] Add the capability-rooted observe/reserve/inspect/release shell, real Redb intent/reconciliation and interruption/reopen tests r[molten.compute_capacity.reservation_authority] r[molten.compute_capacity.uncertainty_release].
- [x] [serial] Add canonical Preserves offer/reservation/readback records and versioned Nickel contracts/profiles r[molten.compute_capacity.offers] r[molten.compute_capacity.readback].
- [x] [serial] Update architecture and profile documentation and a concrete two-host synthetic consumer example, linking Kiln #49 as owner of pending live qualification r[molten.compute_capacity.fixtures].
- [x] [serial] Run bounded supplied-observation adapter and durable restart/reconciliation evidence r[molten.compute_capacity.durable_reconciliation].
- [x] [serial] Run the integrated core/shell, supplied-observation, canonical/Nickel and real-store restart checks, formatting, Clippy and strict source/lifecycle gates for the bounded service r[molten.compute_capacity.fixtures] r[molten.compute_capacity.durable_reconciliation].

## External qualification boundary

Live cross-host/provider/device qualification remains unexecuted. As specified by the proposal, design and issue #42, it is separate from this bounded service: Kiln #49 owns actual host probes, provider/process lifecycle, credentials and end-to-end qualification; TT-Onix #44 owns TT host health integration, with Aspen #15/#20 owning their transport/fencing scopes. The former final checkbox incorrectly placed that external work inside this change's completion criterion. It is retained here as an explicit non-claim, not marked executed or removed from its external owners. Synthetic observations and real Redb restart tests do not establish physical device enforcement.

The [ordered integrated source checks](../../../evidence/integrated-cutover-2026-11-16/checks.json) passed complete workspace library tests, configured Octet/strict source admission, Clippy, formatting and strict provider-policy Cairn validation; earlier focused core/shell/store checks remain separately retained. The [exact Nickel execution](../../../evidence/integrated-cutover-2026-11-16/capacity-contracts.json) accepted the profile and all four positive fixtures and observed real contract denials for all nine negative fixtures, with source hashes and pinned toolchain. These are synthetic authoring/accounting and local durable-owner controls, not live host/device enforcement.
