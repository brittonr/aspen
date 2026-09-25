# Portable startup evidence: verification only

Normal startup remains blocked. This component verifies a portable input snapshot. It does not prove tool execution or authorize a node.

## Read-only command

```sh
molten-node startup-evidence verify \
  --policy /absolute/operator/cohort.json \
  --bundle /absolute/evidence
```

Both paths are explicit operator read grants. The policy belongs outside the untrusted bundle. Its Nickel contract is `docs/node-startup-cohort.ncl`.

The policy binds exact source, executable, compiler, Octet, and descriptor identities. The executable identity must be the independently built `molten-node` binary, not the legacy root `molten` executable. There is no default approved cohort. The command never creates a policy from supplied evidence.

A successful report always contains:

```json
{
  "disposition": "verification-only",
  "execution_established": false,
  "startup_authorized": false
}
```

The report grants no lifecycle authority. Production and VM guards remain active.

## Lifecycle input wiring (blocked)

`molten-node run` accepts paired `--startup-policy` and `--startup-bundle` paths.
`molten-node serve` accepts the same pair only with `--content-config`, without `--live-iroh`.
Incomplete pairs and incompatible routes fail before state creation.

Both routes re-read the selected evidence before protected effects. They use the same descriptor-first verifier as the read-only command.
A passing snapshot still fails with `startup-evidence-real-cohort-not-approved`.
No real execution/build cohort is approved by this implementation.

Unit fixtures test wiring only. They cannot authorize production startup or content serving.
Missing serve-side evidence also fails in unit tests; it cannot use the test-only startup fallback.

## Bundle contract

`bundle.json` uses `molten.node-startup-bundle.v2`. It contains `cohort` and an ordered `members` array. Each member contains only `role`, `blake3`, and `bytes`. The trusted policy uses `molten.node-startup-cohort.v2`; older policy/bundle formats are not accepted.

The fixed member order is:

| Role | Filename |
| --- | --- |
| `cargo-manifest` | `Cargo.toml` |
| `dylint-config` | `dylint.toml` |
| `cargo-lock` | `Cargo.lock` |
| `flake-lock` | `flake.lock` |
| `rust-toolchain` | `rust-toolchain.toml` |
| `source-inventory` | `source-inventory.json` |
| `command` | `command.txt` |
| `status` | `status.json` |
| `summary` | `summary.txt` |
| `object-corpus` | `object-corpus-receipt.json` |
| `build-inputs` | `build-inputs.json` |

The source inventory is a sorted array of `{name, blake3, bytes}` records. Names identify approved source files. They grant no filesystem access. Context members must match their inventory records. Required source owners include all three local crate manifests and library roots, the separate binary, daemon bodies, content core, host state, and verifier.

`build-inputs.json` declares `{"schema":"molten.node-build-inputs.v1","executable_target":"molten-node","units":[{"package":"molten-core","target":"lib","source_paths":[...]}, ...]}`. Units are ordered by `(package, target)` and each unit has sorted, nonempty Rust source paths. The first-party `lib` units must contain their respective `src/lib.rs`; the `molten-node-runtime`/`bin/molten-node` unit must contain its binary source. A root `molten` unit is forbidden. All compiled dependency and generated Rust inputs must have their own units and source inventory records. The union of unit paths must exactly match *all* `.rs` records in the source inventory. An omitted declared compiler input denies even if the Octet status says zero findings. The raw compiler dependency outputs, source bytes, package graph, tool logs, and binary build must still be independently observed and matched to this declaration; this JSON cannot attest its own completeness or compiler execution.

Descriptor and policy limits are 32 KiB each. Members have an 8 MiB limit. The combined member limit is 32 MiB. The source inventory permits at most 32768 records.

## Verification order

1. Admit the independently selected policy.
2. Verify the descriptor digest before decoding member metadata.
3. Compare the descriptor cohort and running executable with the policy.
4. Read each fixed member once through the directory capability.
5. Compare the bounded build-input union with the complete claimed source inventory, then re-evaluate the strict gate from measured bytes and explicit source context.

Member reads reject symlinks, special files, excess bytes, and observed size/time changes. Nonblocking opens prevent a substituted FIFO from waiting for a writer. The root must be operator-owned. This is not hostile-parent or whole-filesystem snapshot protection. Exact hashes reject mixed snapshots that differ from approved bytes.

On Linux, the executable read uses `/proc/self/exe`. It refers to the running image, not a replaceable pathname. Other platforms fail closed in this initial adapter.
Unit tests supply a synthetic executable digest to the verifier's in-memory test seam because the all-targets test binary can exceed the production 512 MiB executable limit. Production verification always measures the running image; these tests do not prove a runnable approved cohort.

The evaluator preserves the existing Octet metadata formulas and strict checks. It adds summary/count consistency and actual corpus-path checks. Node startup and control-gate source scopes name `crates/molten-node-runtime/src/source_gate.rs`; the legacy `src/octet/gate.rs` remains in scope only for root CLI consumers. A replay command that names a file does not satisfy these additional checks. The evaluator performs no workspace, clock, network, or process access.

## Remaining evidence obligations

A digest preserves identity, not truth. Matching policy and bundle bytes cannot establish that Octet ran. They also cannot establish that the binary came from the declared source.

Before startup admission, the producer must retain actual pinned-tool execution, independently compare the build-input declaration to the compiler's complete dependency closure (including dependencies and generated Rust sources), and associate the measured `molten-node` executable with that build. The selected strict Octet package scope is `-p molten-node-runtime -p molten-core -p molten-node-host --all-targets`, not the old root executable. The operator must review those observations before selecting a cohort. Structural test data must never become an approved runtime cohort.

Prior desktop discovery observed the repository's `nightly-2026-05-26` compiler. Both pinned Octet inputs declare `nightly-2026-03-21`. Their roles and binary identities remain separate. Discovery is not gate acceptance.

No new VM transfer or native replay is claimed. No Stage0, compiler replacement, package rebuild, host listener, physical deployment, or release promotion is permitted by this verifier.

## Selected-source gate observation (2026-09-24)

The pinned `c9b06bcf565c51d4a77d210e61b69ae51db9df25` Octet CLI (`dbf4b36c…`) and lint library (`10919bdd…`), with the existing March-21 compiler and Dylint driver, ran the unmodified deny-all hook on the worktree's declared `-p molten-node-runtime -p molten-core -p molten-node-host --all-targets` scope. After correcting the Dylint library filename to the toolchain-qualified alias of the same pinned shared object, `cargo octet check --artifact-dir target/octet` exited **2** (`cargo` exit **101**). `target/octet/status.json` reported `integration-failure`, **2,293 errors**, zero warnings, and zero autofixes applied. All reported findings were in the broad `molten-core` package: 461 `assertion_density`, 344 `path_segment_repetition`, 312 `fragile_exhaustive_enum_match`, 253 `non_trait_imports`, and other categories. Compilation stopped there; zero findings reported for the host/runtime packages does **not** mean those packages passed. The package still compiles unrelated pure domains, so selecting the correct executable alone has not made the source cohort clean.

This was an uncommitted worktree observation before subsequent node-only transport-helper relocation, not an immutable source/build/binary association or an approved cohort. The exact source tree and compiler dependency closure still need independent retention and review. Normal startup, the VM, and replay remain denied. A `molten-node` Nix package derivation evaluation was also blocked by the locked `executable-extent-src` Radicle input missing from local storage; the public seed for the exact revision returned HTTP 500. No Nix package realization is claimed.
