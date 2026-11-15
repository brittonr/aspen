# Checked Job DAG routing and captured authoring

## Why
Issues #38 and #39 require port selection to mean actual selection, without changing v1 or replacing admission.

## What changes
Add a closed selected-sequence-v2 registry, exact implementation signatures, bounded routing envelopes, and a captured Nickel adapter into canonical JobDag admission. Keep runtime authority and existing scheduling owners.

## Non-goals
No live handles, ambient imports, arbitrary graph equivalence, feedback, or Choregraph replacement library. Shared reusable construction remains blocked on Choregraph #6.
