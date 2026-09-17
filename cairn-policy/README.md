# Molten lifecycle policy source

The accepted runtime policy is `generated/cairn-policy.json`.
The `cairn-policy-export` Nix package reproduces that file without changing its bytes.

The export composes two declared inputs:

- Cairn's published Nickel policy at `OnixResearch/cairn`, revision `695124d459574ba7aeba6097310d237f393c243c`.
- Molten's typed `consumer.ncl` function, which adds the `src` Rust trace root to the `cairn-default` profile.

Nix pins the producer source in `flake.lock`.
The consumer function retains all other policy fields and profiles.
It does not read an ambient sibling checkout.
Cairn maintainers own the base policy. Molten maintainers own the trace-root addition and the byte-comparison gate.

## Verification

```sh
nix build .#cairn-policy-export --no-link --print-out-paths
nix build .#checks.x86_64-linux.contract-export-drift-gate --no-link -L
```

The drift gate compares the exported policy with the accepted JSON byte for byte.
It also exercises accepted exports and rejected Nickel fixtures.
The consumer fixtures reject missing profiles and a non-array profile value.

## Legacy fixtures

The local `default.ncl`, `contracts.ncl`, and `structured-lifecycle-contracts.ncl` remain inputs for the existing legacy fixtures.
They do not reproduce the accepted runtime policy.
The inherited `refresh_command` describes the upstream export, not the complete Molten customization.

Do not overwrite `generated/cairn-policy.json` with a direct export of the legacy default or the upstream policy.
Such an export loses accepted policy fields or the Molten trace root.
A policy update requires review of the producer revision, consumer function, and resulting policy differences.
Passing the export gate proves byte agreement, not policy correctness or lifecycle completion.
