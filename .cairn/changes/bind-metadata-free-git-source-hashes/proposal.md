# Proposal: Bind metadata-free git source hashes

## Why

`nix flake check` fails on `origin/molten` (`4e31cee55`) with fixed-output hash mismatches for
`z3kGDT73ja8rewvGLwJFLF5MTx1ih-a0de793` (durable-authority-state: `replay-ledger-core`, `revocation-view-core`) and
`z4ARxEt93T6gJTrTZpUyKHMAemPiv-5abc2b9` (`bounded-http`, `bounded-http-core`). Eighteen checks are blocked behind those
fetches, including `nextest`, `clippy`, the NixOS VM checks, and the release gates. Three more sources fail on any clean
builder: ChaosControl `b8c440e`, content-identity `7f55597`, and Choregraph `b3e08e1`. They pass locally only because
stale store paths that still contain `.git` exist from an earlier prefetch.

The root cause is in plan generation. Pinned unit2nix `d4883180` prefetches missing git hashes with
`nix-prefetch-git --fetch-submodules --leave-dotGit`. Its `fetch-source.nix` then builds with `pkgs.fetchgit` without
`leaveDotGit`. Every hash unit2nix computes itself covers a tree that still contains `.git`, which is also
nondeterministic, so no clean fetch can reproduce it. `crate-hashes.json` held only a few keys, several without a
revision, so most sources fell through to that prefetch.

Content integrity holds. The `a0de793` tree from the seed and from the sibling `durable-authority-state` checkout is
the same tree (`e786c62b`), and both hash to `sha256-++GX+…`.

## What Changes

- Record a revision-qualified, metadata-free SRI hash for every Cargo git source in `crate-hashes.json` (16 sources).
  Drop the unqualified keys, which would apply to any revision.
- Add `scripts/git-source-hashes.sh`. `--write` prefetches each Cargo.lock git source without `.git`. If a host is
  unreachable, it falls back to the matching `flake.lock` `narHash` for the same revision. `--check` rejects missing,
  unqualified, or non-SRI entries, entries that disagree with a same-revision `flake.lock` input, and any generated plan
  hash that differs from `crate-hashes.json`.
- Regenerate `build-plan.json` and `release-policy-build-plan.json` with the pinned unit2nix. The two plans differ from
  the committed ones only in git source hashes, `inputsHash`, and the absolute `workspaceRoot` path.
- Add the `git-source-hash-binding` flake check. Add `nix-prefetch-git` and `jq` to the dev shell. Record the procedure
  and the two exact regeneration commands in `docs/reproducible-dependencies.md` step 6.

## Impact

- **Files**: `crate-hashes.json`, `build-plan.json`, `release-policy-build-plan.json`, `scripts/git-source-hashes.sh`,
  `flake.nix`, `docs/reproducible-dependencies.md`.
- **Testing**: `nix-store --realise --check` on each of the five fixed-output derivations; the new
  `git-source-hash-binding` check with positive and negative inputs; the eighteen previously blocked checks.

## Out of Scope

- Accepted specifications do not change. Source revisions, URLs, `Cargo.lock`, and `flake.lock` stay the same. Only
  the identities of already-pinned sources are corrected, and the generation procedure is made to produce them.
- The upstream unit2nix fix (drop `--leave-dotGit` in `src/prefetch.rs`) belongs to the unit2nix owner. It needs a
  separate pin bump.
