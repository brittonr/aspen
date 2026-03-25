# snix-glue patches

Vendored from `https://git.snix.dev/snix/snix.git` at revision `e20f82dd6fdebe953fb71bb2fde2f32841015c47`.

## Changes

### Fetch::Git implementation (`src/fetchers/mod.rs`)

Replaced the `Fetch::Git()` stub with a full variant containing `url`, `rev`, `ref`, `shallow`, `submodules`, `all_refs`, and `exp_nar_sha256` fields. Implemented:

- `Debug::fmt` — redacts URL credentials via `redact_url`
- `store_path()` — computes CA store path when `exp_nar_sha256` is known (same pattern as `Fetch::Tarball`)
- `ingest()` — clones via `git` subprocess (bare clone + `git archive` or `--recurse-submodules`), ingests checkout into castore, computes and verifies NAR hash
- rstest `#[case]` entries for `fetch_store_path` with and without expected hash

### builtins.fetchGit implementation (`src/builtins/fetchers.rs`)

Replaced the `NotImplemented("fetchGit")` stub with a full implementation:

- Argument parsing: accepts string (URL) or attrset with `url`, `rev`, `ref`, `submodules`, `shallow`, `allRefs`, `narHash`, `name`
- Rejects unknown keys with `UnexpectedArgumentBuiltin`
- Fetch-or-cache: checks PathInfoService for cached paths when `narHash` is provided
- Returns `Value::Attrs` with `outPath` (with Nix context), `rev`, `shortRev`, `lastModified`, `lastModifiedDate`, `revCount`, `narHash`, `submodules`
- `format_seconds_since_epoch`: pure integer arithmetic date formatter (no chrono dependency)

### Cargo.toml

- Added `tempfile` as a runtime dependency (was dev-only upstream)
- Added `[workspace]` table to allow standalone `cargo test`

## Rebasing on snix upgrades

The patch surface is isolated to two files. When upgrading snix:

1. Check if upstream has implemented `Fetch::Git` / `builtins.fetchGit` — if so, drop this patch
2. Otherwise: vendor the new revision, re-apply the changes
3. Run `cargo test --lib -- fetch_store_path builtins::fetchers::tests` in the vendor dir to verify
