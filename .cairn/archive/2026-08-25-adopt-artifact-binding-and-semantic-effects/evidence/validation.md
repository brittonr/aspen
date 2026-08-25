# Artifact binding and semantic effect adoption closeout

Date: 2026-08-25

## Closeout context

This active package restates an adoption that was already implemented, synchronized, and archived at `.cairn/archive/1970-01-01-adopt-artifact-binding-and-semantic-effects`.
The active delta requirements are byte-for-byte equal to the requirement blocks in these accepted specifications:

- `.cairn/specs/live-artifact-binding/spec.md`
- `.cairn/specs/semantic-operation-adoption/spec.md`
- `.cairn/specs/generation-retirement/spec.md`

Only the titles and purpose text differ.
The closeout did not reinterpret or replace the accepted requirements.

## Producer identity

Molten still consumes these immutable revisions:

- `artifact-binding-core` from `OnixResearch/onix-artifact` revision `c932138d880ddf4c2967f4c024b489b5c0022bf1`;
- `kamacite-core` from `OnixResearch/kamacite` revision `d76fe4abe543724d8fc0ac4b362187caf2e27622`.

Cargo, Cargo lock, Nix inputs, Nix lock, the typed release profile, and both generated unit2nix plans identify these revisions.
The current build compiled both producer crates from their pinned remote revisions.

## Focused behavioral validation

Fresh current-head tests passed:

- 14 `molten-core` live-binding and semantic-operation tests;
- 4 canonical adoption artifact and mapping-fixture tests;
- 19 effects tests under the current `actions` compatibility module;
- 6 library and 2 CLI system-extension tests;
- 8 library and 1 CLI retention tests;
- 20 library and 2 CLI protocol tests;
- 5 positive and negative `molten-release-policy` tests.

These tests cover exact and stale transitions, old-work/new-work pinning, denied implicit nested resolution, complete and incomplete retirement, cycles, shared and exclusive attribution, stable pin paths, exact semantic handlers, behavior drift, directional compatibility, replay-only denial, semantic replay/cache re-keying, canonical Preserves round trips, effect handles, system-extension readback, retention non-authority, and protocol-session gates.

Workspace formatting and all-target Clippy with `-D warnings` passed.
The direct Octet run completed with the unchanged workspace baseline of 5,833 warning findings.
This warning-only run is not strict Octet acceptance evidence.

## Release dependency gate repair

The current Nix release-dependency check exposed two pre-existing fail-closed packaging drifts:

1. the generated release-policy plan omitted its executable target;
2. the typed release profile omitted the pinned `schema-identity-core` and `schema-identity-conformance` dependencies.

The closeout declares the executable target, regenerates both unit2nix plans, adds one immutable `schema-identity-src` Nix input, and adds both typed profile rows.
`flake.lock` was regenerated with `nix flake update schema-identity-src`; it was not edited manually.

The focused release-dependency Nix check passes with 11 dependency rows and 2 archive receipts.
Its report BLAKE3 is `eb7919e929036705515c9e4949a9b5e615121ac5fb2f9f07c3e82a4431e1fa9b`.

The `checks.x86_64-linux.molten` Nix rail passed 1,411 tests with no failures or skips.
Its CI test-run receipt is `blake3:c3155ef340d6df841c9075df1052cc8c9a1fd5db8430a7843eba462c5968d08f`.

## Cairn evidence

Strict Cairn validation passed.
The current gate receipts are:

- proposal: `c7e7f446e8ff2b485d73bc51c543cc858e8bfe1b43f310d64e11b502b6eead98`;
- design: `81b716647f857009b6a7449d458fe3f3957ecf272310920e20358639d56c79ef`;
- tasks: `6269325f0077c8edd5b6d91939e81f22807904b7d63ffe43e677524aa01f7fa8`.

The delta specifications now target the accepted capability names and use explicit `MODIFIED` operations.
Sync preflight classified all 14 requirements as `already_applied` and made no accepted-spec changes.
The dry-run plan is `0324637f25b254244a01cb6879d6930ecd048a3afedfb16fd54ab85ce7280962`.
The executed no-op sync receipt is `76ab108c93eaf71be92cb80dd7875647140da7a0c35c18c2ecb5d14d5a31f4ea`.
Strict validation passed after sync.

The archive dry-run passed with plan `7e62db11fea1351bef6815b84a7c8810cdb040ac7ce697b298dd8130048da36a`.
Archive execution moved the package to `2026-08-25-adopt-artifact-binding-and-semantic-effects`.
The archive receipt is `812027972af93167fce6da5e792facfe70b53c3801303b4306aa2b8e0535489f`.
Strict validation passed after archive execution, with 12 active changes remaining.

## Non-claims

This evidence does not prove producer correctness, compatibility truth, handler behavior, host authorization, atomic publication, root-observation truth, remote-holder completeness, retention clearance, garbage-collection eligibility, deletion authority, deployment safety, or release eligibility.
