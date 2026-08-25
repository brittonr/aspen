# Design

## Context

`validate_release_profile` is a pure core function. Existing tests call it directly, but no imperative shell accepts reviewed inputs or emits its canonical receipt.

## Architecture

The functional core keeps tier, reference, freshness, stack-provenance, policy-hash, and placeholder policy. It gains one optional candidate content reference. Release tier requires a valid, non-placeholder candidate reference and records it in the canonical validation value.

The CLI shell accepts explicit scalar inputs, constructs `ReleaseProfileInput`, invokes the core, and writes the canonical Preserves value. It exits unsuccessfully when the validation decision is `deny` after preserving the diagnostic receipt.

The Nix check invokes the packaged CLI with a valid fixture. It also runs invalid fixtures for a missing candidate reference and a placeholder source-gate reference. These fixtures prove command behavior only; they do not constitute candidate release evidence.

## Boundaries and Non-Claims

- The core performs no file, process, environment, clock, or network access.
- The CLI owns argument parsing, output, and process exit behavior.
- The Nix check proves deterministic validator wiring for declared fixtures.
- A passing validation receipt does not prove artifact truth, source-gate success, deployment success, or release eligibility.

## Validation

Run focused tests before and after the core change. Run formatting, Clippy, the focused Nix check, strict Cairn validation, sync, archive, and the broad Nix rail before integration.
