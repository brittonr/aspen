# Design

## Existing owners

The content core owns manifest-specific read grants and verification transitions. The existing live adapter owns Iroh and bounded handoff admission. Node state owns root capabilities and persisted identity. The normal node daemon owns active startup, service locking, heartbeat, supervision, and stop state.

## Composition boundary

An explicit typed content configuration selects a manifest, reader grant, bind address, and finite limits. The normal `node serve` path must validate the configuration before the content listener starts. It must derive chunk and identity capabilities from its existing node root. It must not reopen descendant host paths or infer authority from a pin or ticket.

Content readiness and transfer receipts stay separate from control-plane readiness. Shutdown must close the content router and preserve the existing node lifecycle. A separate fixture server in the same VM does not satisfy this design.

The receiving normal CLI path must admit independent manifest/provider/socket expectations, use existing transfer verification, and publish output only after complete verification. Onix retains its separate fixed archive digest and native package admission.

## Build prerequisites

The exact executable-extent and VM Cohort Git commits are available through owner repositories on the desktop. Invocation-local Git cache transport can recover those objects. Commit and locked NAR identity must match before use. Manifests, source URLs, lockfiles, and the desktop toolchain remain unchanged.

Current dependencies require Rust 1.96 or newer. An existing installed Rust 1.97.1 is available for ordinary compilation. It is not the repository's exact nightly or an Octet acceptance substitute. Cairn 3b4c280 is the producer's pinned tool version, but that tool expects the older `cairn/` layout. The present tree has active content in `.cairn/` and retained history under `cairn/archive/`. Do not overwrite either tree to force a gate.

## Evidence

Use separate VMs, no shared payload mount, and only an initial storage-VM archive seed. Require transfer, server denial, wrong digest, clean restart, damaged-store failure, and the unchanged native replay. No host listener or full-daemon success claim is permitted before these commands actually run.
