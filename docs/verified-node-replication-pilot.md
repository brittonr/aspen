# Verified node-replication pilot

Molten evaluated [`verus-lang/verified-node-replication`](https://github.com/verus-lang/verified-node-replication) only as a possible **local multicore/NUMA data-structure** building block. Node replication here does not mean network replication, distributed consistency, Raft, consensus, Iroh transport, or fabric membership.

## Exact inputs

The typed source of truth is [`verification/verified-node-replication-pilot/profile.ncl`](../verification/verified-node-replication-pilot/profile.ncl). It pins:

- upstream source `eb8e27244a4f1b7714defe55aa495c6173a1c330` and its fixed-output source hash;
- the source tree's Verus gitlink `d8f073f31d72a5df2113b4ab9c011c1a27b38abf` and its fixed-output source hash;
- the historical Rust `1.76.0` context;
- Octet profile `octet-verus-0.2026.04.03.21dfcd2`, exact Verus release/binary, solver, and official Rust component identities;
- bounded probe timeout and saved-log size;
- trusted-boundary counts, promotion criteria, and non-claims.

Run the reproducible pilot with:

```bash
nix build .#checks.x86_64-linux.verified-node-replication-pilot \
  --no-link --print-out-paths -L
```

## Decision

The current decision is `blocked-verifier-internal-error`:

1. Without `-V new-mut-ref`, the selected verifier reports seven unsupported-feature diagnostics.
2. With that required feature flag, it reaches `Verus Internal Error: var_local_id failed` in `verified-node-replication/src/exec/rwlock.rs`.
3. The source contains 33 `#[verus::trusted]` markers, nine external-body markers, and one `assume` site. The trusted set includes the three top-level refinement theorems and public `Dispatch` and `NodeReplicatedT` traits.
4. No Molten Cargo manifest or lockfile admits the upstream crate.

The normalized diagnostics, marker inventories, canonical decision payload, and BLAKE3-bound decision are saved under [`cairn/archive/2026-07-11-pilot-verified-node-replication/evidence/`](../cairn/archive/2026-07-11-pilot-verified-node-replication/evidence/).

## Promotion gate

Runtime dependency work remains denied until all profile criteria pass: current-verifier compatibility, trusted-boundary acceptance, a NUMA-local adapter design, positive and negative concurrency tests, bounded NUMA benchmarks, rollback/removal testing, and scoped Octet/provenance/Valence/Cairn evidence.

Even a future successful verifier run would prove only named obligations over the exact upstream model and its explicit trusted boundaries. It would not prove Molten integration, whole-system behavior, distributed correctness, production performance, or release readiness.
