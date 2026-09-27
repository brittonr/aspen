# Kache Nix Rust Specification

## Purpose

Defines the `kache-nix-rust` capability.

## Requirements

### Requirement: Kache comes from onixpkgs

r[molten.onixpkgs_kache.source]
Molten MUST take the kache package from the pinned onixpkgs overlay and the wrapper helpers from `onixpkgs.lib.kacheNixRust`.
Molten MUST NOT pin `onix-core` for kache.

#### Scenario: The flake is locked
- GIVEN the Molten flake inputs
- WHEN the lock file is inspected
- THEN it contains a rev-pinned onixpkgs node whose nixpkgs follows Molten's nixpkgs and no onix-core node

#### Scenario: The wrapped toolchain is evaluated
- GIVEN kache is enabled for the unit2nix Rust toolchain
- WHEN `molten-kache-rust` is evaluated
- THEN its rustc wrapper invokes the onixpkgs kache package

### Requirement: The overlay does not change unrelated derivations

r[molten.onixpkgs_kache.isolation]
Applying the onixpkgs overlay MUST NOT shadow a nixpkgs package that Molten uses.
Packages, checks, and dev shells that do not use kache MUST keep their derivation paths for a fixed flake source.

#### Scenario: A non-kache derivation is compared
- GIVEN the default package, a non-kache check, or the dev shell and a fixed flake source
- WHEN its derivation path is compared with the onixpkgs overlay and with an overlay that adds only the former kache
- THEN the paths are equal

#### Scenario: An overlay name matches a nixpkgs package
- GIVEN an onixpkgs attribute that also exists in nixpkgs
- WHEN the Molten flake is audited
- THEN no Molten derivation references that attribute

### Requirement: Wrapper behavior stays under contract

r[molten.onixpkgs_kache.contract]
The kache wrapper contract check MUST build against `onixpkgs.lib.kacheNixRust` with a fake kache package.
Only the kache package may differ between the former and current wrapped toolchains.

#### Scenario: The wrapper contract check runs
- GIVEN the contract check with a fake kache
- WHEN it builds
- THEN disabled mode, missing-cache failure, cache-directory export, key salt, and rustdoc compatibility hold

#### Scenario: The wrapped toolchains are compared
- GIVEN `molten-kache-rust` before and after the switch
- WHEN the derivations are diffed
- THEN the only difference is the kache package moving from 0.6.0 to 0.16.0
