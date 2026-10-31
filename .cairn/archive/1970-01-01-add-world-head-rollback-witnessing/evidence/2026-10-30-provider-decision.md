# Provider ownership decision and pinned consumer contract

## Ownership decision

- Selected owner: the dedicated `release-channel` repository, explicitly selected by the
  workspace operator (user) on 2026-10-29 after comparing a dedicated owner, a narrow
  Mantle extension and deferral. This is the user's choice, not an endorsement from
  Mantle, Cairn, a witness administrator or an OnixOS operator.
- Governance record: root workspace commit
  `87e08c928c63185e69ad1c0a30f09e8e3f8abe1c` ("Register the release-channel repository
  and record its ownership") on branch `cairn/release-channel-governance-20261029` of
  `/home/brittonr/git/OnixResearch`, editing
  `workspace-lifecycle/.cairn/changes/establish-governed-release-channels/design.md`
  section 7. Observed with `git show 87e08c9` on 2026-10-30. **That commit is local and
  not yet published**; the parent change's pilot, validation and archive tasks remain
  open there. This Molten change consumes only the recorded ownership decision and the
  published provider source, not completion of the parent change.
- Canonical repository: `git@github.com:OnixResearch/release-channel.git` (private),
  `main` = `3577429a05acfb442a177582c151dc5557db9ff6`, initial
  `fa389e19d5c9bd84cd75df914cc5418c70d92bfb` (local checkout
  `/home/brittonr/.cargo-target/release-channel-20261029`, `git log` shows exactly those
  two commits).

## Pinned consumer contract

| Fact | Value | Where |
|---|---|---|
| Source revision | `fa389e19d5c9bd84cd75df914cc5418c70d92bfb` | `Cargo.toml` (`release-channel-index`, `release-channel-witness`), `Cargo.lock`, `flake.nix` `release-channel-src`, `crate-hashes.json` (`release-channel-index` entry), `src/world_witness/provider.rs` `contract::SOURCE_REVISION` |
| Contract | `release-channel-index-v1` | `contract::CONTRACT_ID`; release-channel `governance/external-index.ncl` `protocol` |
| License | MIT (workspace `license = "MIT"`, root `LICENSE` "MIT License, Copyright (c) 2026 Release Channel contributors") | `contract::LICENSE`; `THIRD_PARTY_LICENSES.md` row added by this change |
| Provider identity | `ed25519:<hex key>` of the independently installed provider key | `provider::identity`, `provider::Trust::validate` |
| Operator pin check | any other revision is rejected before state is created | `world-witness-consumer-pilot` `index::policy`; executed negative in the pilot transcript (`3577429…` rejected) |

`3577429` changed only `crates/governed-pilot`, `crates/pilot` and `tools/consumer`
(observed with `git show --stat 3577429`); the `release-channel-index` and
`release-channel-witness` crates Molten compiles are identical at both revisions. The pin
stays at `fa389e1`; moving it requires explicit integration review and a coordinated
Cargo/Nix/unit2nix pin update.

The canonical Bounded HTTP fix `e0f43bf54bd24203a51fc1952489c17a70c68c5f` is pinned by
release-channel's workspace for `release-channel-transports`. Molten does not depend on
`release-channel-transports`; Molten's own `bounded-http` lock entry
(`5abc2b93e0f288aba251a0750ce2d716eb9757fc`) is unrelated to this consumer contract and is
unchanged by this change.

## Provider executable used by the pilot

`provider` was built from the exact Cargo checkout of the pin
(`~/.cargo/git/checkouts/release-channel-c3cb42956983fddb/fa389e1`, `git rev-parse HEAD` =
`fa389e19d5c9bd84cd75df914cc5418c70d92bfb`, clean apart from Cargo's `.cargo-ok` marker):

```
cargo build --locked --offline -p release-channel-index --bin provider
  (CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-witness-target-20261030/release-channel-fa389e1)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 09s
sha256 1e4e7fc2117249a00b55118a253defde38c65b06890e5dcbf62a6f2502a47e33  debug/provider
```

## Non-claims

- The provider is an authenticated single-provider high-water index, not a Merkle log,
  global consensus, or proof that the provider cannot roll back or fork.
- Same-host execution shares the administrative and physical failure domain with the
  consumer; it is explicitly experimental and is not administrator- or host-independent
  witnessing, provider honesty, production trust or release eligibility.
- Witness observations grant no branch authority and prove no world-commit truth.
- Publication of the provider repository is not independent witness administration.
