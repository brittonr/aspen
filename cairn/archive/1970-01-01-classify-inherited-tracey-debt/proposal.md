# Proposal: Classify inherited Tracey debt

## Problem

Molten has an exact baseline of 1,981 accepted requirements without direct source references.
The baseline prevents hidden growth, but it does not group the debt by specification and source area.
Unstructured review risks false implementation claims and repeated searches.

## Goal

Produce a deterministic classification inventory for every remaining baseline entry.
Use a conservative default when implementation, replacement, obsolescence, or invalidity is not established.
Repair only source links that production logic and tests directly support.

## Scope

- add a pure classifier with a thin filesystem shell;
- fail on missing or duplicate accepted requirement definitions;
- group the exact inventory by specification and source area;
- record typed metadata and deterministic identities;
- repair three verified source links;
- keep all other entries explicit and unclaimed.

## Non-goals

This change does not prove that every accepted requirement is implemented.
It does not mark requirements as replaced, obsolete, or invalid without explicit lifecycle evidence.
It does not remove accepted requirements or generate blanket source markers.
