# Design: Pin the Octet toolchain in the dev shell

## Context

`flake.nix` already takes `octet-toolchain` (`github:OnixResearch/octet?rev=fc38f593…`) for the Verus pilot. The dev
shell did not export its `cargo-octet`, so the README sequence depended on ambient installs.

## Decisions

### Decision: The catalogued flake pin is authoritative

**Choice:** Put `octet-toolchain.packages.${system}.cargo-octet` in `devShells.default.packages`.

**Rationale:** The owner decided that the exact catalogued pin is the Octet authority. `mkShell` puts `packages`
ahead of the inherited `PATH`, so the pinned tool shadows any ambient install inside the shell. Its config and profile
hashes match Molten's recomputation, because the gate reports `status-config-current` and `status-profile-current` as
passing.

### Decision: Document the true state, defer enforcement

**Choice:** README states the measured counts and the deny receipt, and names the burn-down series. The hook and
flake-check enforcement land after the series reaches zero.

**Rationale:** Enforcing now would block every commit. Documenting a clean state would be false.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the dev shell hunk in `flake.nix`, README:741, and
the strict-sequence evidence in `evidence/verification.md`.

## Failure behavior

If the pin changes or the input disappears, dev shell evaluation fails. The strict gate still denies `warning-only`,
which this change does not alter.

## Risks / Trade-offs

- Operators who relied on the newer ambient tool now see the pinned tool's slightly different output. The pinned
  output is the reproducible one.
