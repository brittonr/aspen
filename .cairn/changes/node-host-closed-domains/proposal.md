# Declare three deliberately closed node-host domains

## Status

Reviewed for scoped implementation in review.md. Conditional analysis annotations and tests are recorded in implementation.md.
This change does not approve deployment or close `node-startup-evidence`; source-gate policy and startup authority remain unchanged.

## Why

Three node-host enums describe fixed storage namespaces or acquired-file observations, not extensible dispatch protocols.
Their four exhaustive matches must force a policy decision when a variant is added.
The current strict Octet command diagnoses unmarked exhaustive local matches, while its documented sealed-domain contract supports this deliberate design.
Adding catch-all arms would conceal new cases; disabling the lint would weaken unrelated checks.

Octet's marker-admission defect is separately repaired and tested at `e4ecf888a0b419d0175d2fe1d748c24322ecce89`.
That repair is not approval to mark arbitrary enums or promote a runtime cohort.

## Proposed change

Declare only these existing domains deliberately closed:

1. `LocalStoreKind`: its eight kinds map to fixed local-store directories.
2. `NodeStateNamespaceKind`: its 14 kinds map to fixed capability-derived namespaces, retaining existing aliases and distinct kind identity.
3. `NodeStateFileObservation`: Missing, NonRegular, and Regular retain distinct absence, denial, and acquired-handle behavior.

Use the exact `octet::sealed_enum` attribute during Octet checking, supported by explicit documentation and mutation tests.
Do not use FSM markers: these classifications are not themselves state/event transition machines.
Do not mark `NodeStateEntryKind` or any additional enum under this proposal.

## Compiler compatibility boundary

Propose conditional tool registration and marker attributes under Dylint's existing injected `dylint_lib = "octet"` cfg.
Normal compilation must not acquire a new unconditional `register_tool` feature requirement.
Register that cfg with Cargo's expected-cfg mechanism without reducing warning or lint severity.
This is an explicit, reviewable analysis-time feature change, not an unstated assumption of compatibility.
The canonical command and existing compiler selections remain unchanged.

## Non-goals

No public renames, enum variant changes, new aliases, directory relocation, wildcard arms, blanket allows, repr changes, new provider policy, authority widening, filesystem behavior changes, compiler/toolchain acquisition, Stage0, or startup-policy relaxation.
No May-26 runtime build/binding, approved cohort, VM launch, physical deployment, or replay evidence is claimed.
Other 26 node-host findings remain outside this proposal; the current complete command still reports 30 errors.

## Acceptance

Requirements are in `specs/node-runtime/spec.md`; implementation and evidence work are in `tasks.md`.
Acceptance needs exact mapping/observation tests, future-variant mutation rejection, ordinary-build compatibility controls, and a fresh unchanged-command source-gate run.
A lower finding count alone is not acceptance. The expected removal of four enum findings must be explained by the reviewed domain declarations.
