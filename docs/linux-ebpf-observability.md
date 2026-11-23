# Linux eBPF observation boundary

## Current delivery status

The `add-linux-ebpf-observability-adapter` change has [executed local core, reader, and observer-admission fixtures](../.cairn/changes/add-linux-ebpf-observability-adapter/tasks.md#local-verification-2026-11-21) and a [passing exact disposable-VM run](../.cairn/changes/add-linux-ebpf-observability-adapter/tasks.md#passing-cleanup-corrected-disposable-vm-2026-11-22). The unprivileged Aspen consumer read all six required signals. Replacement, old-generation denial, cancellation, cleanup and guarded evidence export also passed in that fixture. This does not qualify an installed node adapter or a production host endpoint.

The archived contract is `AgentReport.runtime`, serialized by OnixOS `cli/src/cmd/bpf_runtime_agent.rs::emit_report` to stdout and `receipts/<receipt_identity_blake3>.json`. Its `BpfRuntimeStatus` binds machine, active generation, bounded generation statuses and read-only/no-handle/no-credential flags. Each generation reports Pack, kernel-build/BTF, module/object/loader, attachment, verifier, observation-loss and cleanup classes. This remains **status only** and is not relabelled as an observation stream. The new `onixos.bpf-observation.window.v1` endpoint is a separate governed extension with actual producer records and final accounting.

## Implemented consumer boundary

- `OnixBpfReader` receives a `node_state::Scope`, one typed relative receipt leaf and independently selected machine/generation/Pack/kernel/BTF/object/loader/attachment expectations. It reads only regular bounded files, rejects malformed/unknown/ambiguous runtime fields and credential/mutable-handle flags, closes on cancellation or cohort drift, and never discovers ambient bpffs state. A fresh reader and independently supplied selection are required after generation rollover.
- Wire decoding is in the shell; typed status validation and cohort admission are in `molten-core`. A status receipt is never converted into a complete observation window.
- `EbpfBinding`, `EbpfWindow` and related DTOs implement the closed `onixos.bpf-observation.window.v1` protocol now authored by OnixOS. `EbpfEndpointReader` validates the independently supplied checked profile before I/O, reads only one supplied bounded regular-file capability, consumes an immutable epoch once and closes on cancellation, timeout, malformed input or cohort drift. Pure admission enforces exact generation/schema/cohort/scope, finite mappings/records/bytes/streams, supplied freshness, duplicate/gapped source sequences, overflow and reconciled accounting.
- Producer or consumer loss, missing counters, pending records, cancellation or failed final close cannot yield complete health. Zero-event admitted signals project zero counters, not fabricated events. Source-local sequence facts are checked independently of delivery (nested tracepoints can reserve records out of attempt order); cross-CPU order remains partial. Canonical packed Preserves evidence sorts by source/sequence and excludes framing/encoded byte count, so equivalent interleavings preserve canonical metric/health/status/event/snapshot identity. Raw capture-byte references stay separate from semantic identity.
- `observe_linux_ebpf` composes existing canonical metrics, health, adapter status and one finite lifecycle event. It maps absent capability, permission, timeout, stale/mismatched source, bounds, malformed accounting and cancellation into typed unavailable/partial outcomes. A required incomplete source sets `deny_required_readiness`; an optional source does not authorize anything. No raw kernel event detail or labels are emitted. The finite status event contains only a typed adapter outcome.
- A validated window that reports unsuccessful provider final close yields `failed` with an `adapter-failure` issue; successful close with cancellation yields `cancelled`. Both retain accounting and deny required readiness. Neither outcome describes the consumer's own read-capability closure or grants permission to clean provider state.
- Endpoint admission checks each signal sum against the signed 64-bit metric range. An out-of-range sum produces a corrupt, unavailable observation without samples; it blocks readiness only when the source is required.
- The reader rejects a window's configured wire-byte allowance if it exceeds the enclosing fabric queue budget, before opening the endpoint. Producer queue accounting remains separate.
- `config/linux-ebpf/contracts.ncl` bounds mappings, nanosecond windows/freshness, queue/records/bytes/cardinality, raw-event detail, loss, required policy and canonical non-claims. Rust rejects unknown/duplicate behavior-affecting JSON fields and checks the same export. The VM publisher performs the actual pinned Nickel export before delivering expected-profile input. No fabricated production profile or source identity is installed.

## Ownership and setup

OnixOS alone loads, attaches, replaces, detaches and cleans links/maps/pins. Molten closes only its own read access; it cannot clean host BPF state. Do not run Molten with loader capabilities or provide writable map/link descriptors. Provide an explicitly scoped read capability to the exact receipt leaf for status debugging. Status availability does not enable live observation or satisfy required-window readiness.

The separate OnixOS prerequisite is `add-generation-bound-observation-endpoint`. Its `native/bpf-observation` program attaches real `sched_process_exit`, `sched_switch`, `block_rq_issue` and `net_dev_queue` tracepoints, scopes every event by the current task's cgroup and emits bounded ring-buffer records plus per-CPU attempts/submissions/drop counters. Process exit, scheduler switch, current-task block issue, current-task network queue, runnable-preemption saturation and observed-source health are count signals. Saturation is **not PSI**; block/network attribution is current-task context, not complete application I/O attribution. The collector validates exact packaged/running BTF and supported scheduler trace format, owns only its process-local links/maps, handles cancellation, detaches, drains and reconciles final counters. Missing counters never become zero.

The Nix package takes the caller's locked `pkgs` and exact target BTF, generating the CO-RE header and actual ELF with declared clang/libbpf inputs. The VM-only publisher measures actual kernel/BTF/program/object/loader/schema/scope/accounting/Pack identities, publishes immutable bounded files readable by unprivileged Aspen, and revokes future opens on replacement/cancellation. Revocation cannot erase bytes already copied or held by an existing descriptor. OnixOS owns production lifecycle admission; a disposable harness marker cannot grant it.

The current fixture retains the packaged observation-source manifest separately from the canonical `onix:blake3:bpf-pack` used in the admitted binding. Independently provisioned, signed experimental credentials authorized exact start and owned cleanup effects for the disposable cohort. Those decisions are separate from artifact inspection and Aspen telemetry. This is not a Mantle observation, a ChaosControl receipt, or production target authority.

`examples/ebpf-observe.rs` is the actual unprivileged consumer hook, not a source-model substitute. It requires endpoint root/relative leaf, independently delivered expected binding and Nickel-checked profile, epoch and actual supplied monotonic nanosecond tick. It runs the same core and canonical service paths, emits sample/health/status/event/snapshot references and accounting, and exits nonzero on denied or partial results. The VM rail requires this real executable and Molten source for its checked contract; no successful skip or fake consumer exists.

An operator can explicitly select one copied observation window during `molten node run-loop` or `molten node serve` with `--linux-observation-selection RELATIVE_PATH`. The path is relative to the selected node state root. Provision the signed `linux-observation-reader` grant, boot-bound selection, exact Nickel JSON export and read-only window under `receipts/` independently of node-source trust. The node checks source admission first, verifies the exact selection and consumes its holder nonce, then rechecks trust around the bounded read. It emits canonical `node-linux-observation-start-v1` status and health at local-component scope. No selection means no observation. An optional unavailable window can report status without denying node startup; an incomplete required window denies readiness. A new process needs a fresh selection and grant; the node does not discover or follow a replacement endpoint.

Before publishing host readiness, a complete required window must still be fresh
on the boot-monotonic clock and its authority must still be current on the wall
clock. The node closes the hosting session if that recheck denies. An optional
observation remains a startup fact, not a gate or proof of current host health.

This is a one-window startup path. It does not start a polling daemon or qualify a continuously supervised installed observer. The disposable VM evidence above was produced with the standalone consumer, not with this node entry point.

For offline conformance, set `MOLTEN_TEST_EBPF_REPLAY_ROOT` to the guarded retained capture directory and run
`cargo test -p molten --lib retained_vm_epochs_match_recorded_projection -- --ignored --nocapture`.
The test uses the recorded endpoint bindings and window ticks, compares sample and health references to the
pure projection, and checks the original consumer's references for generation 1. This does not start a new
collector, open a current provider endpoint, or qualify the installed-node lifecycle.

The [NixOS module](nixos-service-module.md#options) can forward the same explicit
selection for one bounded serve. It does not start or retry the selected mode
automatically. Provision a fresh grant for each explicit restart.

## Privacy and debugging

Command lines, payloads, packet contents, private paths, socket/peer identities, secrets and raw kernel pointers are outside normalized records. No raw event detail is enabled. Inspect typed denial classes rather than dumping raw untrusted endpoint bytes. Descriptor labels, samples, health and snapshots must still pass existing canonical bounds/redaction/scope rails. A read receipt does not authorize repair, BPF mutation or capability grants and cannot establish application correctness, cluster truth, confidentiality, complete telemetry or release/production readiness.

## Actual host observations, 2026-10-29

Read-only commands observed Linux `6.18.50`; `/sys/kernel/btf/vmlinux` BLAKE3 `2eea5b1a7caf88852249017c5e07442e4dbe63be15d7f867ced7216937921397`; uid/euid `1555`; empty effective capability set; `kernel.unprivileged_bpf_disabled = 2`; bpffs mounted mode `700`. `sudo -n true` failed with `a password is required`. `/run/onix` and `/var/lib/onix/bpf/state` were absent. `/dev/kvm` was readable/writable, so a separately admitted disposable VM is a possible route, not an executed result.

The archive's 2026-07-16 lifecycle evidence names Linux `6.18.20` and BTF `08c14844b3b9cde37f8b3bba9effde104fa59194195777f3dd859db3919e8064`; it cannot admit this host. No live observation endpoint was available on this host during that check. The later disposable-VM result does not change that physical-host observation.
