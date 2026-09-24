# Tasks: Bind metadata-free git source hashes

## Phase 1: Implementation

- [x] [serial] Add `scripts/git-source-hashes.sh` with `--write` and `--check`, plus `nix-prefetch-git` and `jq` in the dev shell. a[bind-metadata-free-git-source-hashes.binding]
- [x] [serial] Write revision-qualified metadata-free hashes for every Cargo git source into `crate-hashes.json`. a[bind-metadata-free-git-source-hashes.binding]
- [x] [serial] Regenerate both plans with the pinned unit2nix and confirm that only hashes, `inputsHash`, and `workspaceRoot` changed. a[bind-metadata-free-git-source-hashes.procedure]
- [x] [serial] Add the `git-source-hash-binding` flake check and update `docs/reproducible-dependencies.md` step 6. a[bind-metadata-free-git-source-hashes.drift-denied]

## Phase 2: Validation

- [x] [serial] Positive: `nix-store --realise --check` passes for each of the five fixed-output derivations. a[bind-metadata-free-git-source-hashes.fetch]
- [x] [serial] Positive: `scripts/git-source-hashes.sh --check` and `checks.x86_64-linux.git-source-hash-binding` pass. a[bind-metadata-free-git-source-hashes.binding]
- [x] [serial] Negative: `--check` fails against the old plan and the old `crate-hashes.json`. a[bind-metadata-free-git-source-hashes.drift-denied]
- [x] [serial] Build the previously blocked checks and record results in `evidence/verification.md`. a[bind-metadata-free-git-source-hashes.unblocked]
