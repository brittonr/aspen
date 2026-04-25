# Crate Extraction Readiness Report

- Candidate family: `blob-castore-cache`
- Passed: `false`
- Checked candidates: 11

## Failures

- aspen_cache: direct forbidden dependency `aspen-rpc-handlers`
- aspen_cache: direct dependency `aspen-rpc-handlers` is not in allowed reusable dependencies

## Warnings

- aspen_blob: cargo tree failed for `aspen-blob`
- aspen_cache: cargo tree failed for `aspen-cache`
- aspen_coordination: cargo tree failed for `aspen-coordination`
- aspen_coordination_protocol: cargo tree failed for `aspen-coordination-protocol`
- aspen_kv_types: cargo tree failed for `aspen-kv-types`
- aspen_raft_kv: cargo tree failed for `aspen-raft-kv`
- aspen_raft_kv_types: cargo tree failed for `aspen-raft-kv-types`
- aspen_redb_storage: cargo tree failed for `aspen-redb-storage`
