# VM Cohort

VM Cohort owns product-neutral planning and Linux/KVM mechanics for creating bounded worker cohorts from one retained, initialized virtual-machine checkpoint.

Its pure core defines checkpoint admission, cohort state, resource limits, retries, cancellation, cleanup, and evidence projections. A thin shell performs KVM, memory, device, disk, thread, and teardown effects.

## Status

This repository now contains an experimental `no_std` core, a Linux/KVM shell, reusable conformance, typed Nickel profiles, and a bounded CLI.

The first profile runs local Linux/KVM workers on one x86_64 host. Independent fixture and live KVM conformance passed.

ChaosControl pinned the immutable implementation candidate and passed portable, live-KVM, parity, cleanup, authority, and lifecycle gates.

## Scope

VM Cohort owns:

- reusable initialized checkpoint identities and compatibility checks;
- independent clone plans for memory, device, disk, and endpoint state;
- bounded cohort admission and deterministic lifecycle transitions;
- transactional creation, cancellation, retry, and cleanup plans;
- KVM adapter conformance and bounded evidence records.

Consumers retain workload meaning, guest-image trust, authorization, scheduling, secrets, network policy, artifact retention, and release decisions.

VM Cohort does not provide OCI intake, Kubernetes integration, a hosted fleet scheduler, or universal sandbox and determinism claims.

## Architecture

The crate split is:

- `vm-cohort-core`: a `no_std + alloc` functional core;
- `vm-cohort-kvm`: the Linux/KVM imperative shell;
- `vm-cohort-conformance`: reusable positive and negative adapter tests;
- `vm-cohort-cli`: a bounded local operator shell.

Human-authored profiles use typed Nickel. Runtime Rust consumes checked generated values. See [`docs/architecture.md`](docs/architecture.md), [`docs/source-inventory.md`](docs/source-inventory.md), and [`docs/conformance.md`](docs/conformance.md).

## License

The planned reusable core and adapters use `MIT OR Apache-2.0`.

## References

- [SmolVM v1.7.7](https://github.com/smol-machines/smolvm/releases/tag/v1.7.7) — Apache-2.0 design reference for retained golden checkpoints, coordinated fork workers, bounded KVM retry, and transactional clone recovery. No source is copied.
- [`brittonr/chaoscontrol` at `7433557b85990f0f07a37ca44b97fef26c2a4c7e`](https://github.com/brittonr/chaoscontrol/tree/7433557b85990f0f07a37ca44b97fef26c2a4c7e) — reviewed snapshot, dirty-page overlay, and exact KVM restore source.
- [`brittonr/chaoscontrol` at `b8c440ea3b19df796542e58e8ee36200e1c3db85`](https://github.com/brittonr/chaoscontrol/tree/b8c440ea3b19df796542e58e8ee36200e1c3db85) — archived first-consumer pilot. ChaosControl retains fault, scheduler, assertion, coverage, exploration, replay, evidence, and release authority.
- [Trellis](https://github.com/OnixResearch/trellis) — copy-on-write isolation and lifecycle proof-pattern reference.
