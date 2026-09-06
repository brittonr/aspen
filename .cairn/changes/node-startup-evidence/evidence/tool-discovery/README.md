# Existing Octet tools: no approved exact match

This observation follows implementation commit `8e98d752dd3a28e2ffd9a5980e79206a5c2761b1`.
The execution and startup tasks remain open. No Octet gate or VM ran during this search.

## Goal and bounds

The goal was an existing tool set for Octet `fc38f59330b626961d166febfdf1a5aa6575460f`, followed by real strict execution.
Source identity, tool identity, execution, and runtime approval remain separate obligations.

The search covered the worker and desktop Nix stores, with a 100-entry bound per initial directory scan.
It found 14 worker and 32 desktop package outputs: 35 unique output paths and 13 unique source roots.
These counts include wrappers. Neither initial scan reached its result bound.
A separate desktop search covered known Cargo build locations to depth four, with a 20-result bound.
Commands had finite timeouts. Review was single-agent and correlated.

This is not proof that no matching tool exists elsewhere.

## Pinned reference

Offline Nix evaluation used the exact Git revision and disabled import from derivation.
It did not build or install tools. The measured full source matched the unchanged Molten lock:

`sha256-O0jnGZ9cr7bsyZqYTmBEUY0TvRnvMalI19b7eGZu42I=`

The pinned flake's source filter produced:

- Source: `psxg34hk9hb8y0fd9z9vsnksaviz3zak-source`.
- Filtered NAR: `sha256-JI1n3oa2gsnjWkYIcUOrRHwntvs+GGWNgLb4KrxuEZ4=`.
- Wrapper derivation: `ay8f1k5ay04pywljlvgh7hjz3c71jw5i-cargo-octet-0.1.0.drv`.
- Wrapper output: `n8bkxc9p155a27xnx0iqm3l694s8ckkp-cargo-octet-0.1.0`.
- Engine output: `hxd7wylwz756id11hksv9h5ywjjs66f2-cargo-octet-0.1.0`.

The expected library output is `d5kynl0p00wailfm1swkm3rbc7bm7hg8-octet-0.1.0`.
The expected wrapper, engine, and library outputs were absent on both inspected hosts.
A derivation path is a build description, not evidence that its output exists.

## Candidate results

Ten source roots had a different `cargo-octet/src/engine.rs` digest.
Three roots matched that file. Full comparisons rejected treating this one-file match as exact source identity:

| Source root | Observation |
| --- | --- |
| `vakpasllh9w1rcpj4zqkks6wbn35v5gp-source` | 131 changed files against the pinned filtered source. |
| `qvf4q81wh6l18xvygnjdfzcgvk7nkr8a-source` | 11 changed policy/fixture/documentation files. |
| `0nkyn1af7i35n59bc6yd92igzhjmf6ax-source` | 11 changed policy/fixture/documentation files. |

The two desktop trees contain matching Rust sources in this comparison. Their different files include proof-rail manifests and Verus toolchain profiles.
Octet's embedded-source list excludes these changed directories. That narrower match does not establish the requested full pinned input identity.
No policy exception, source-equivalence rule, or changed expected hash was introduced to admit these tools.

Both desktop source exports were restored only into private diagnostic directories.
Their measured NAR hashes matched the remote observations. No source archive or restored tree is published here.
Git file comparisons do not cover empty directories. The distinct whole-tree NAR hashes remain recorded separately.

A desktop debug executable was also found. Its BLAKE3 is:

`1d199ebd501e459780070237d53211b2e4e0882d47e1a82df86313e89c06f5b1`

It contains the path string `cargo-octet/src/wasm_artifact.rs`, which is absent from the pinned source.
This is a source-marker observation, not a complete binary provenance proof. The debug executable was not admitted or executed.

## Route status

1. The default wrapper has mismatched recorded source: rejected.
2. Alternative packaged tools have mismatched complete source: not admitted.
3. The exact pinned flake evaluates offline, but its outputs are absent on the inspected hosts: blocked.
4. The debug executable lacks an accepted source/binary binding: blocked.

The strongest result is a measured pinned reference plus bounded candidate rejection.
Real strict gate execution and normal startup remain unproved.

## Next decision

An already-approved matching tool set can resolve this blocker.
Otherwise, the next action needs approval to build only the pinned Octet CLI and lint library with the existing March-21 compiler and driver.
The existing March-21 compiler ran and reported commit `ac7f9ec7da74d37fd28667c86bf117a39ba5b02a`. The expected driver executable is present.
`existing-compiler-and-driver.log` records both binary digests. Presence is not a new driver acceptance test.
A full wrapper build must not silently build missing compiler or Verus toolchain dependencies.
Rust compilers, the package toolchain, Mantle, and Darkhttpd remain excluded from replacement or rebuild.

## Preserved boundaries

No product code, manifests, lockfiles, expected hashes, source gates, or startup guards changed.
No toolchain replacement, Stage0, package build, host listener, physical deployment, release, or default promotion occurred.
No lifecycle task was marked complete. No lifecycle sync, archive, or main-branch integration occurred.

Logs record observations, not startup receipts. Logs with private diagnostic paths are excluded from public output.
Their digests remain in `private-log-digests.txt`. The original task outputs and both restored source trees remain private.
The failed lookup for a nonexistent pinned `nix/packages.nix` remains in `lookup-error.log`; the source filter actually lives in `flake.nix`.
