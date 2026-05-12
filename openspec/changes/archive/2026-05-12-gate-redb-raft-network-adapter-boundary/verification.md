# Verification

## Evidence

- `evidence/raft-network-compile.txt`: `cargo check -p aspen-raft-network --no-default-features`, default, and `--features sharding` all pass after making `aspen-sharding` optional.
- `evidence/raft-network-trees.txt`: captures minimal/default/sharding normal dependency graphs for `aspen-raft-network`.
- `evidence/raft-network-forbidden-scan.txt`: default/minimal graphs contain no root Aspen, `aspen-raft`, handlers, cluster bootstrap, app binaries, SQL/secrets/hooks/CI/forge crates; the sharding graph contains `aspen-sharding` only under `--features sharding`.
- `evidence/runtime-compatibility.txt`: `cargo check -p aspen-raft`, `cargo check -p aspen-cluster`, and `cargo check -p aspen-rpc-handlers` pass.
- `evidence/redb-raft-kv-readiness.{json,md}`: readiness checker output for the Redb Raft KV adapter family.

## Task Coverage

- Evidence: `raft-network-compile.txt`, `raft-network-trees.txt`, `raft-network-forbidden-scan.txt`, `runtime-compatibility.txt`, and `redb-raft-kv-readiness.{json,md}` cover the adapter boundary, compatibility, and readiness checker artifacts.
- Phase 1 captured the pre/post compile and tree evidence, then identified `aspen-sharding` as the remaining dependency that was documented as feature-gated but still mandatory.
- Phase 2 made `aspen-sharding` optional behind the existing `sharding` feature and recorded negative boundary plus runtime compatibility evidence.
- Phase 3 updated extraction policy/docs and runs strict OpenSpec validation, rustfmt, readiness checker, and diff hygiene before archive.
