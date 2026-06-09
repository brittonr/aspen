# Proposal: Operator Dogfood Nix Release Check

## Summary

Add a first-class Nix check that runs `molten dogfood local-node` after the hermetic nextest check and stores the canonical dogfood report plus release gate receipt as build outputs.

## Motivation

The local dogfood workflow now exercises node startup/shutdown, repro gating, catalog/MCP discovery, and retention GC release evidence. It should be part of the release verification surface rather than only a unit/CLI test.

## Scope

- Add a Nix `checks.<system>.dogfood-local-node` derivation.
- Make the dogfood check depend on the existing `nextest` check output.
- Run `molten dogfood local-node` with an explicit temporary state root.
- Copy the canonical report, release gate receipt, summary, and nextest dependency marker into the Nix output.
- Document the check and evidence-only boundary.

## Non-goals

- Do not make dogfood receipts authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust.
- Do not replace nextest, Octet, Cairn validation, or subsystem gates.
