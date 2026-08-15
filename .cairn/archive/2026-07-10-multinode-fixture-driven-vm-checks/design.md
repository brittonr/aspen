# Design: multinode-fixture-driven-vm-checks

## Overview

Make VM shard execution consume a checked fixture export or validate its observed evidence against one. Nickel remains the human-authored contract language. Rust consumes exported data and performs pure validation; Nix remains the shell that invokes VM checks.

## Fixture-derived plan

A VM scenario fixture should provide or derive:

- scenario id and purpose;
- topology profile id and topology ref;
- execution profile id and command surface;
- expected artifact kinds;
- required receipt refs or classes;
- variance refs and unavailable policy;
- diagnostic log refs and evidence-only caveats.

## Validation flow

Before a shard can claim pass, the validator compares fixture metadata with observed VM evidence. Mismatches in topology, profile, command surface, expected artifact kinds, required child refs, unavailable policy, caveats, or source language deny.

## Boundaries

Fixtures declare scenario intent and evidence scope. They do not grant authority or replace subsystem validation. Generated or exported JSON/TOML should be derived from Nickel and treated as checked input, not source of truth.
