# Proposal: Repair runtime-spine Tracey links

## Problem

The grouped inherited-debt inventory assigns 442 uncovered requirements to `cairn/specs/runtime-spine/spec.md`.
Some requirements already have direct pure-core implementations and focused positive or negative tests, but no Tracey markers bind them.

## Goal

Repair one authority-coherent runtime-spine batch without generating blanket coverage.
Bind only requirements proven by specific shared Preserves rail functions and tests.

## Scope

- review the runtime-spine queue by source area;
- bind fourteen Preserves rail requirements to existing implementation and tests;
- record a typed direct-repair manifest;
- regenerate the exact baseline and grouped inventory;
- add a freshness gate for every reviewed repair;
- preserve all remaining requirements as explicit debt.

## Non-goals

This change does not claim complete runtime-spine coverage.
It does not claim that all Preserves trust boundaries use strict decoding.
It does not promote archive tasks or generated comments to implementation evidence.
It does not change runtime behavior.
