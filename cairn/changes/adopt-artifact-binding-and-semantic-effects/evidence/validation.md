# Artifact binding and semantic effect adoption validation

Date: 2026-07-27

## Baseline

Task 875 passed the pre-change `molten-core`, effects, retention, system-extension, and protocol-session test filters. Task 871 passed repository lifecycle validation plus proposal, design, and tasks gates before implementation.

The original Aspen checkout stayed at `935729eeb4a37b3e71e77a42d4c4e7417311fd33`. Its modified `README.md` and untracked Raft, eBPF, and adoption changes were not changed.

## Producer identity

Molten consumes these exact remote producer revisions:

- Artifact `artifact-binding-core`: `c932138d880ddf4c2967f4c024b489b5c0022bf1`;
- Kamacite `kamacite-core`: `d76fe4abe543724d8fc0ac4b362187caf2e27622`.

Cargo manifests, `Cargo.lock`, Nix inputs, `flake.lock`, the typed release profile, and both generated unit2nix plans identify those revisions. Sibling paths remain explicit development overrides only.

The release-dependency validator passed with nine rows and report BLAKE3 `9393adbcce6ca1bfb23adb2a21ea845e1296fcc3827ce673d76813ad03fa60c2`.

## Behavioral evidence

The focused pure-core suite passes 14 positive and negative tests. It covers:

- exact producer source agreement and source drift;
- successful successor planning, stale compare-and-swap, and missing product gates;
- one-snapshot old-work/new-work resolution;
- denied implicit nested lookup and unsupported late-binding profiles;
- complete retirement and incomplete or uninstrumented profiles;
- shared and exclusive attribution;
- cycles, duplicate roots and edges, and stable pin paths;
- exact semantic handler and all-surface identity binding;
- behavior-key drift and name-only fallback rejection;
- directional replay compatibility, reverse denial, and replay-only live-use denial;
- retained Molten policy, capability, and provenance gates;
- replay, transcript, cache, job, remote-execution, and upgrade identity re-keying.

The focused root suite passes canonical Preserves construction and parsing, strict semantic binding, governed positive and negative mapping fixtures, duplicate-field rejection, and wrong-artifact rejection.

Task 938 passed full workspace tests after the core adoption. Task 945 passed focused post-change tests and strict workspace all-target Clippy. Task 947 passed the repository `cargo octet check` commit hook.

The typed Nickel semantic fixtures export successfully. The artifact-auth source-agreement receipt was regenerated because the reviewed dependency graph, locks, release profile, and unit2nix plans changed. Its focused Nix check passed after the receipt contract and deterministic projection were updated together.

## Lifecycle evidence

Accepted-spec synchronization passed with receipt `9cda4f79cb6ed137a40a41e1839c892f43fc2278c033924453839a3b974b3de2`.

The focused traceability profile covers 14 of 14 requirements. Its receipt is `59eb198c6bfce7cd03d3f10331503856cb7a9555da0a02e14aac0d0a65c6f549`.

Task 990 passed `nix flake check -L` on `x86_64-linux`. Nix reported `all checks passed`, including full nextest, source agreement, unit2nix, dogfood, release evidence, and promotion checks. Other systems were omitted as incompatible by the flake.

The archive receipt is recorded after the archive command completes.

## Publication blocker

Remote Aspen `main` is `4734913c1c230a35fc121b948cdf54ac0619dd2a`. It is not a descendant of the reviewed Molten base `935729eeb4a37b3e71e77a42d4c4e7417311fd33`. This adoption can publish to a feature branch, but it cannot move remote `main` by fast-forward. No force push or history replacement is authorized.

## Non-claims

This evidence does not prove producer correctness, compatibility truth, handler behavior, host authorization, atomic publication, root-observation truth, remote-holder completeness, retention clearance, garbage-collection eligibility, deletion authority, deployment safety, or release eligibility.
