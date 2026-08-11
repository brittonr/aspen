# Proposal: Repair runtime-spine blob-ref job Tracey links

## Problem

The inherited runtime-spine queue still includes twelve blob-ref job requirements.
Current production code and focused tests directly support nine of them, but Tracey has no direct links.

## Goal

Bind the nine proven blob-ref job requirements without promoting broader replay, full status, or job-DAG integration claims.

## Scope

- review the twelve blob-ref job requirements against current production code and tests;
- bind nine requirements with direct implementation and verification markers;
- add a typed batch manifest and deterministic JSON export;
- regenerate the exact inherited baseline and grouped classification;
- validate manifest freshness, uniqueness, marker paths, and baseline removal;
- preserve unsupported requirements as explicit debt.

## Non-goals

This change does not establish blob-ref replay identity, every declared job status transition, or job-DAG integration.
It does not change runtime behavior.
It does not prove complete runtime-spine coverage, release readiness, or whole-system correctness.
