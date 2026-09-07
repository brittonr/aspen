# Portable startup evidence: verification only

Normal startup remains blocked. This component verifies a portable input snapshot. It does not prove tool execution or authorize a node.

## Read-only command

```sh
molten node startup-evidence verify \
  --policy /absolute/operator/cohort.json \
  --bundle /absolute/evidence
```

Both paths are explicit operator read grants. The policy belongs outside the untrusted bundle. Its Nickel contract is `docs/node-startup-cohort.ncl`.

The policy binds exact source, executable, compiler, Octet, and descriptor identities. There is no default approved cohort. The command never creates a policy from supplied evidence.

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

`node run` accepts paired `--startup-policy` and `--startup-bundle` paths.
`node serve` accepts the same pair only with `--content-config`, without `--live-iroh`.
Incomplete pairs and incompatible routes fail before state creation.

Both routes re-read the selected evidence before protected effects. They use the same descriptor-first verifier as the read-only command.
A passing snapshot still fails with `startup-evidence-real-cohort-not-approved`.
No real execution/build cohort is approved by this implementation.

Unit fixtures test wiring only. They cannot authorize production startup or content serving.
Missing serve-side evidence also fails in unit tests; it cannot use the test-only startup fallback.

## Bundle contract

`bundle.json` uses `molten.node-startup-bundle.v1`. It contains `cohort` and an ordered `members` array. Each member contains only `role`, `blake3`, and `bytes`.

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

The source inventory is a sorted array of `{name, blake3, bytes}` records. Names identify approved source files. They grant no filesystem access. Context members must match their inventory records. Required source owners include the daemon bodies, content core, and new verifier.

Descriptor and policy limits are 32 KiB each. Members have an 8 MiB limit. The combined member limit is 32 MiB. The source inventory permits at most 32768 records.

## Verification order

1. Admit the independently selected policy.
2. Verify the descriptor digest before decoding member metadata.
3. Compare the descriptor cohort and running executable with the policy.
4. Read each fixed member once through the directory capability.
5. Re-evaluate the strict gate from measured bytes and explicit source context.

Member reads reject symlinks, special files, excess bytes, and observed size/time changes. Nonblocking opens prevent a substituted FIFO from waiting for a writer. The root must be operator-owned. This is not hostile-parent or whole-filesystem snapshot protection. Exact hashes reject mixed snapshots that differ from approved bytes.

On Linux, the executable read uses `/proc/self/exe`. It refers to the running image, not a replaceable pathname. Other platforms fail closed in this initial adapter.

The evaluator preserves the existing Octet metadata formulas and strict checks. It adds summary/count consistency and actual corpus-path checks. A replay command that names a file does not satisfy these additional checks. The evaluator performs no workspace, clock, network, or process access.

## Remaining evidence obligations

A digest preserves identity, not truth. Matching policy and bundle bytes cannot establish that Octet ran. They also cannot establish that the binary came from the declared source.

Before startup admission, the producer must retain actual pinned-tool execution and complete source/binary association. The operator must review those observations before selecting a cohort. Test data must never become an approved runtime cohort.

The desktop has the repository's `nightly-2026-05-26` compiler. Both pinned Octet inputs declare `nightly-2026-03-21`. Their roles and binary identities remain separate. Discovery is not gate acceptance.

No new VM transfer or native replay is claimed. No Stage0, compiler replacement, package rebuild, host listener, physical deployment, or release promotion is permitted by this verifier.
