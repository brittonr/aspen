# Tasks: Adopt command-pattern resource reservation

All tasks are proposed work. This package grants no implementation permission.

- [ ] [serial] Record the current read-then-write reservation sequence, capacity accounting types, existing scheduler and capacity test baselines, and the coordination boundaries with F09, lease-epoch, and the oracles change. r[molten.reservation_command.atomic_eval]
- [ ] [serial] Add the atomic reservation command transition over authoritative state with expected-generation checking and typed rejection that preserves state. r[molten.reservation_command.atomic_eval]
- [ ] [serial] Move capacity, selection, and allocation decisions into the pure transition and reduce the shell to decided-effect execution. r[molten.reservation_command.pure_core]
- [ ] [serial] Add bounded batch admission with item-count, byte-count, and waiting-time limits, per-item identities, and per-item results. r[molten.reservation_command.bounded_batches]
- [ ] [serial] Add the expiry-not-release rule: stale lease capacity is reusable only through a transition with an enforcement or termination precondition, with visible shortfall otherwise. r[molten.reservation_command.expiry_not_release]
- [ ] [parallel] Add contended-reservation traces, stale-generation rejection, and capacity-invariant checks after every path including rejections. r[molten.reservation_command.atomic_eval] r[molten.reservation_command.pure_core]
- [ ] [parallel] Add batch boundary fixtures: each limit at its edge, over-limit rejection, per-item isolation, non-enlarged atomicity, and malformed batch records. r[molten.reservation_command.bounded_batches]
- [ ] [parallel] Add expiry fixtures with and without enforcement facts, plus negative reuse attempts and operator-status shortfall visibility. r[molten.reservation_command.expiry_not_release]
- [ ] [serial] Update fabric-time docs and receipt fixtures with the command shape, batch limits, and the expiry-does-not-stop-external-jobs non-claim. r[molten.reservation_command.expiry_not_release]
- [ ] [serial] Keep F09 capacity-composition and lease-epoch token regressions passing, then run focused Octet, Clippy, workspace, Nix, and required Cairn gates. r[molten.reservation_command.validation]
