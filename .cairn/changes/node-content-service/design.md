# Design

## Existing owners

The content core owns manifest-specific read grants and verification transitions. The existing live adapter owns Iroh and bounded handoff admission. Node state owns root capabilities and persisted identity. The normal node daemon owns active startup, service locking, heartbeat, supervision, and stop state.

## Composition boundary

An explicit typed content configuration selects a manifest, reader grant, bind address, and finite limits. The normal `node serve` path must validate the configuration before the content listener starts. It must derive chunk and identity capabilities from its existing node root. It must not reopen descendant host paths or infer authority from a pin or ticket.

Content readiness and transfer receipts stay separate from control-plane readiness. Shutdown must close the content router and preserve the existing node lifecycle. A separate fixture server in the same VM does not satisfy this design.

The receiving normal CLI path must admit independent manifest/provider/socket expectations, use existing transfer verification, and publish output only after complete verification. Onix retains its separate fixed archive digest and native package admission.

## Build prerequisites

The exact executable-extent and VM Cohort Git commits are available through owner repositories on the desktop. Invocation-local Git cache transport can recover those objects. Commit and locked NAR identity must match before use. Recovery kept manifests, source URLs, lockfiles, and the desktop toolchain unchanged. Service implementation promotes the already-locked `cap-tempfile` dependency from test-only to runtime use in Molten and `molten-node-host`. It supplies atomic capability-relative publication. Neither lockfile nor any dependency revision changes.

Current dependencies require Rust 1.96 or newer. An existing installed Rust 1.97.1 is available for ordinary compilation. It is not the repository's exact nightly or an Octet acceptance substitute. Cairn 3b4c280 is the producer's pinned tool version, but that tool expects the older `cairn/` layout. The present tree has active content in `.cairn/` and retained history under `cairn/archive/`. Do not overwrite either tree to force a gate.

## Implemented policy migration

The authored Nickel policy lagged the committed runtime projection. The migration preserves that projection's trust hashes, gate settings, workflow choices, replay cases, and receipt identities. It adds required traceability metadata and task-order marker definitions. Five missing receipt-schema rows return without deleting their existing contract references. A pinned Cairn Nickel snapshot supplies shared schemas. Regression tests reject weaker gates and missing prior records.

The node host owns atomic leaf replacement without exporting directory capabilities. Content status uses that operation. The optional listener starts after normal startup and service-lock admission, runs inside the normal tick loop, and closes before the normal service receipt completes. Its tick-wait budget is finite; it is not a hard wall-clock bound on arbitrary blocking filesystem calls.

## Startup evidence blocker found in the real VM run

The normal `run_local_with_root` path calls `synthetic_clean_octet_gate_receipt_for_tests`. The first VM attempt stopped at its ambient `/Cargo.toml` read before a listener started. Supplying a minimal manifest, as the older Onix compatibility adapter does, would permit manufactured clean findings and fixture artifact refs. This change must not use that workaround.

Production startup and content serving therefore remain blocked until a real, source-bound Octet evidence admission route exists. Test-only startup fixtures can remain behind `cfg(test)`. No flag, environment variable, or workspace file can bypass the production guard. The installed Rust 1.97.1 and focused Clippy results do not replace the exact nightly/Octet gate.

## Evidence

Use separate VMs, no shared payload mount, and only an initial storage-VM archive seed. Require transfer, server denial, wrong digest, clean restart, damaged-store failure, and the unchanged native replay. No host listener or full-daemon success claim is permitted before these commands actually run.
