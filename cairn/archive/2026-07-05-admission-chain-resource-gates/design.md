# Design: admission chain resource gates

## Context

The admission chain adapts Kubernetes' validate/default/mutate/persist boundary to Molten. It is not a webhook interface. Admission runs through deterministic core checks and policy/evidence gates over canonical Preserves values.

## Admission phases

A resource admission attempt should bind these ordered phases:

- envelope decode and canonical identity check;
- schema/contract validation;
- request authority preflight for subject, scope, operation, and held capabilities;
- deterministic defaulting from reviewed defaults;
- deterministic mutation from reviewed mutation rules;
- final validation over the post-mutation candidate;
- Basalt/UCAN, Nickel, Steel/Trellis, and Octet evidence gates as applicable;
- commit-plan receipt construction.

Defaulting and mutation are data transformations, not side effects. They must be replayable and evidence-bound.

## Functional core

Pure admission cores accept the resource operation, prior resource summary, candidate resource, request authority summary, rule refs, policy refs, and evidence summaries. They return an admitted commit plan or denial diagnostics. The core never reads webhooks, files, environment variables, clocks, or stores.

## Imperative shell

The shell gathers rule and policy materials, resolves authority summaries, computes refs, persists admitted candidates, and publishes dataspace events. If a phase denies, the shell emits denial evidence and performs no state mutation.

## Status isolation

A status operation may update observed-state refs and conditions only. It cannot advance desired generation, change desired refs, mutate finalizers, or change authority-bearing metadata unless an explicit future requirement admits that operation.
