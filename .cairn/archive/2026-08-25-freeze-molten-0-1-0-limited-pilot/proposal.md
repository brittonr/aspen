# Freeze the Molten 0.1.0 limited-pilot candidate

## Why

Molten has candidate-bound release gates, but it has no frozen `0.1.0` candidate or fresh evidence set for that candidate. The README still cites older evidence whose Nix outputs are absent. A pilot release must identify one immutable source and preserve the exact receipts reviewed for that source.

## What Changes

- Freeze commit `a4f111690b6962f04d9320fd93d09c7dd1ad2fd0` and its Git tree as the Molten `0.1.0` limited-pilot source.
- Derive one domain-separated BLAKE3 candidate reference from the frozen Git commit and tree.
- Generate fresh Rust, nextest, Nix, Cairn, Octet, VM, dogfood, bundle, promotion, export, profile, pilot-decision, and candidate-gate evidence.
- Preserve positive and denial evidence without converting fixture, diagnostic, or warning-only output into stronger claims.
- Publish scoped pilot release notes and tag only the verified frozen candidate.

## Impact

This change affects release evidence, accepted operator requirements, release documentation, and publication metadata. It does not change runtime behavior. It authorizes only a limited internal pilot and excludes general production claims.
