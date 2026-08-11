# Proposal: Repair runtime-spine SAM service Tracey links

## Problem

The inherited runtime-spine queue still includes thirteen SAM service requirements.
Current service records, demand runtime, supervision logic, and focused tests directly support all thirteen, but Tracey has no direct links.

## Goal

Bind the thirteen proven SAM service requirements without changing runtime behavior or promoting broader runtime-spine claims.

## Scope

- review the thirteen SAM service requirements against current production code and tests;
- bind each requirement with direct implementation and verification markers;
- add a typed batch manifest and deterministic JSON export;
- regenerate the exact inherited baseline and grouped classification;
- validate manifest freshness, uniqueness, marker paths, and baseline removal;
- preserve explicit authority, logical-supervision, retention, and release non-claims.

## Non-goals

This change does not add ambient service authority or treat operating-system parentage as logical supervision.
It does not bypass retention policy.
It does not change runtime behavior or prove complete runtime-spine coverage, release readiness, or whole-system correctness.
