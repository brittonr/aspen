# Design: distributed test CI risk matrix

## Scope

Define and document a risk/cost matrix for distributed validation. The matrix does not invent new trust; it organizes existing and planned evidence surfaces so developers and release reviewers know which checks are fast, which are heavy, and which claims each check can support.

## Proposed matrix

| Profile | Purpose | Evidence boundary |
| --- | --- | --- |
| `fast` | pure core, unit, parser, receipt validation | no platform or transport claims |
| `protocol` | deterministic simulation, model/property checks, drift fixtures | simulated distributed invariants |
| `cli` | harness and receipt CLI workflows | local process/receipt behavior |
| `vm-smoke` | two-node NixOS topology and core workflow | platform integration smoke evidence |
| `vm-fault` | executable VM fault injection | bounded platform fault evidence |
| `soak` | dogfood, pilot, external/live evidence | constrained pilot/readiness review only |

## Proof checklist

- **Proof claim**: distributed test shards are explicit, reproducible, traceable to requirements, and cannot pass release evidence by retrying or skipping required heavy checks silently.
- **Out of scope**: forcing every local developer run to execute VM or soak checks, broad production approval, or replacing authority/policy/provenance/resource/source-gate/retention gates.
- **Trusted assumptions**: Nix and nextest execute declared profiles; traceability inputs accurately list coverage evidence; BLAKE3 content refs bind canonical artifacts.
- **Positive evidence**: each profile emits or preserves canonical metadata and traceability refs; release-readiness sees complete positive and negative coverage for distributed evidence requirements.
- **Negative evidence**: missing profile artifacts, missing positive or negative traceability, stale requirement refs, retry-only pass, skipped VM support, and undeclared variance deny or mark unavailable.
- **Canonical refs**: source/tree ref, flake/input ref, test binary ref, profile ref, shard ref, seed ref, topology ref, fault-plan ref, run receipt refs, traceability manifest ref, and variance refs.
- **Regeneration command**: documented `cargo nextest` profiles and Nix checks/apps for each matrix entry, plus release-readiness traceability validation.

## Functional core

Represent the matrix as data and validate evidence availability with a pure core over declared profiles, required artifact kinds, requirement coverage entries, retry policy, and unavailable/skipped states. Shell code wires nextest, Nix, and CLI commands.

## Retry boundary

Release and CI pass evidence must use zero retries. Exploratory reruns may exist to gather flake diagnostics, but their output is diagnostic or quarantine evidence until a reviewer binds an explicit remediation or exemption.
