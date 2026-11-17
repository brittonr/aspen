## Why

Issue #23 remains blocked by Radicle-only source acquisition despite public immutable provider revisions. Anonymous HTTPS Git probes now advertise the exact executable-extent and hardened-runtime pins. Choregraph's Cargo source also needs matching policy and Nix coordinates.

## What Changes

- Use the existing public HTTPS seeds for the exact hardened-runtime and executable-extent revisions, without changing provider code.
- Align the Choregraph policy and Nix source with its existing public Cargo mirror.
- Select Bounded Exec's catalog-advertised GitHub source only after anonymously acquiring the exact existing commit and matching its metadata-free source identity.
- Investigate the catalog-advertised Bounded Tree Radicle publication/bootstrap without inventing mirrors, changing pins or admitting local sibling source as public evidence.
- Resolve every reachable required default-source coordinate, including Durable File Publication's unchanged pin, and acquire the complete current merged Rust unit-graph source cohort anonymously rather than stopping at the first unavailable endpoint. Observe the complete fixed Nix input inventory, retaining unavailable exact sources explicitly. Override the executable-extent verifier's nested Cairn Artifact input from SSH to the anonymously verified exact HTTPS commit, without changing provider revisions or NAR identity.
- Regenerate Cargo/Nix locks, source hashes and both unit graphs through their owning tools, preserving source-trust and dependency checks.
- Ship owner-approved exact snapshots/fixtures for the unavailable VM Cohort and extent-producer inputs, preserving failed route observations and implementation identities rather than claiming route recovery.
- Scope the pinned tool's public SSH-to-HTTPS rewrite to the build environment, and retain actual clean hosted Cargo/README, default Nix build and full-check success before archive.

## Impact

- **Files**: dependency manifests and Nickel source policy, generated locks/hashes/plans, acquisition documentation and current-dependency notices.
- **Testing**: bounded anonymous Git acquisition, source/dependency positive and negative checks, and exact candidate build rails where the environment permits.
- **Owner / consumer**: Aspen release-dependency owner; fresh public Cargo/Nix consumers need repeatable credential-free acquisition, not shared owner credentials.
