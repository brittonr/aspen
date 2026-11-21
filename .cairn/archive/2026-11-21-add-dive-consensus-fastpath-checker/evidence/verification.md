# Three-checker parity run

On the integration candidate based on Aspen `701633f4701402480c56c53fedd82cca379f22c3`, pueue task 38 executed the existing `checks.x86_64-linux.consensus-fastpath-model-parity` derivation from the edited worktree via:

```sh
nix build --builders '' --max-jobs 1 --cores 3 \
  --option substituters https://cache.nixos.org \
  path:.#checks.x86_64-linux.consensus-fastpath-model-parity \
  --no-link --keep-failed --print-out-paths -L
```

Exit 0; all five synthetic tests passed, followed by six actual checker invocations. The realized output was `/nix/store/amhlmax4zrjwxv3lr10mqc37nrqs4cy5-molten-consensus-fastpath-model-parity`, derivation `/nix/store/kl8dl2y941d4z8dkn46v2grdz1hmnz4z-molten-consensus-fastpath-model-parity.drv`.

| Checker | Baseline wall | Generated / distinct / queued | Missing-promise mutant |
| --- | ---: | --- | --- |
| TLC 2.19 / TLA+ 1.7.4 | 64.23 s | 6,312,147 / 966,147 / 0 | `NoMissingPromise`, six states |
| TLA-ts 0.0.10 | 84.16 s | 6,312,147 / 966,147 / 0 | `NoMissingPromise`, six states |
| Dive 0.1.0 | 56.62 s | 6,312,147 / 966,147 / 0 | `NoMissingPromise`, six states |

The [original campaign record](run/campaign.json) is retained with all 25 members, including raw stdout/stderr and schema-3 reports. Its BLAKE3 is `cbc6dd86d679e960a066f5afc819d68a9775eb71e365482a5eb9b2f99a0f30c7`. Independent readback verified all 25 BLAKE3 entries and byte identity of the model and both configs to the baseline revision. The checker and campaign bounds remain 180 s and 720 s. Mutant prefix counts are observations, not a gate.

TLC is authoritative. This establishes bounded finite-model parity, not a proof of the Rust implementation, Raft, liveness under arbitrary schedules, or release qualification. Aspen consensus verification maintains the rail.
