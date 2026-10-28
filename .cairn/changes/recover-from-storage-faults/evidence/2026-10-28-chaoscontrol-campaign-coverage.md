# ChaosControl storage-fault campaign coverage — 2026-10-28

## Pinned provider

ChaosControl revision `0b43ae5e4e9ec03e634c9ebce977c2cd2544b27d` publishes the
deterministic storage-recovery campaign shell and the eight-scenario suite
retained under
`.cairn/changes/model-guest-storage-flush-failures/evidence/storage-fault-campaign-suite/`
in that repository.

Producer surfaces used here:

- virtio flush admission/execution and one-attempt fault outcomes
  (`VIRTIO_BLK_T_FLUSH`, `BlockFault::FlushError`);
- bounded per-node persistent disks (`chaoscontrol-smr::persistence`);
- storage-safety and participant-progress decisions
  (`chaoscontrol-smr::storage_recovery`);
- schedules and cohort-bound receipts (`chaoscontrol-smr::storage_schedule`);
- the `storage-recovery-campaign` file shell.

## Campaign results

| Storage condition | Evidence | Result |
|---|---|---|
| flush error | `storage-fault-campaign-suite/flush-error-receipt.json` | refused with `storage-flush-failed`, prior record restarts |
| write error | `storage-fault-campaign-suite/write-error-receipt.json` | refused with `storage-write-failed`, published state unchanged |
| corruption | `storage-fault-campaign-suite/corruption-receipt.json` | restart `denied(storage-record-schema)`; peer repair passes |
| cache retention | ChaosControl `chaoscontrol-vmm` block tests `stale_cache_survives_application_restart_but_not_reboot`, `restart_profiles_distinguish_retained_cache_eviction_and_reboot` | retained/evicted/reboot profiles distinct; stale cache lost on reboot |
| restart | `storage-fault-campaign-suite/restart-receipt.json` | published identity restored |
| local-only recovery | `storage-fault-campaign-suite/local-only-receipt.json` | `pass(local-repair)` |
| peer repair | `storage-fault-campaign-suite/peer-repair-receipt.json` | `pass(peer-repair)` |
| global absence | `storage-fault-campaign-suite/global-absence-receipt.json` | `pass(global-absence)` |
| incomplete observation | `storage-fault-campaign-suite/incomplete-observation-receipt.json` | `incomplete`, exit 1 |

Receipt identities are recorded in that suite's `summary.json` and in the
`2026-10-28-storage-fault-campaign-suite.md` note; each binds its request
BLAKE3, schedule BLAKE3, cohort identity, safety decision, participants,
oracle version, and non-claims.

## Scope

These are deterministic model campaigns over supplied facts and one bounded
file shell. They do not prove guest kernel, filesystem, Redb, Raft, or Molten
behavior; they do not establish repair authority; and they do not close the
remaining voting-readiness gate task in this change, which stays deferred.
