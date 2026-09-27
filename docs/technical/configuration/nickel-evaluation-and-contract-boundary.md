# Nickel Evaluation and Contract Boundary

This article distinguishes Nickel authoring checks, embedded static normalization, and Molten admission. It assumes familiarity with configuration exports and content references. It complements the [Technical companion](../README.md); the [Nickel toolchain cohort](../../nickel-toolchain.md) and governing subsystem contracts remain authoritative.

## One cohort, several responsibilities

Molten reviews the command-line evaluator and embedded evaluator as one cohort: Nickel CLI 1.17.0 at upstream revision `1320a983e6c3d1e2fb53dd2464b084b4903b1426`, with `nickel-lang` 2.2.0, core 0.18.0, parser 0.3.0, and vector 0.2.0. These are compatibility inputs, not interchangeable version suggestions. The [`nickel-toolchain-cohort` check](../../../flake.nix) checks the CLI version, Cargo lock entries, upstream revision, a positive production export, selected negative fixtures, and a missing-import failure.

The useful invariant is narrower than “all configuration is correct”: the reviewed evaluation surfaces are exercised against shared acceptance and rejection examples. A matching version string alone does not establish semantic equivalence for every Nickel expression, nor does fixture success establish policy completeness.

There are two distinct consumers. The [fixture inventory](../../nickel-contract-fixture-inventory.md) describes repository-owned authoring modules whose checked JSON or Preserves exports feed Rust consumers. Separately, the [harness normalization implementation](../../../src/harness/parts/schema/p020/body.rs) embeds Nickel through `nickel_lang::Context`, calls `eval_deep_for_export`, then `expr_to_json`. Thus “Rust consumes checked exports” must not be broadened into “no Rust code evaluates Nickel.” The production-profile contract's comment is scoped to that profile consumption boundary, not to every harness path.

## Evaluation is not admission

The embedded helper accepts source text and returns exported JSON or a converted Molten error. Contract choice, source construction, and subsequent interpretation belong to its callers. No filesystem mutation, network send, or capability grant follows merely from the helper returning successfully.

The [policy preflight path](../../../src/harness/parts/schema/p019/body.rs) makes this layering concrete. `policy_preflight_material` starts with a Molten policy snapshot, computes its canonical reference, constructs Nickel source, hashes the source as a Preserves string, exports JSON, and hashes that JSON string separately. It then creates a Basalt contract envelope with explicit backend, contract identity, version, source hash, and schema identities. Envelope validation is another boundary, not an implied consequence of Nickel evaluation.

Readback of this evidence checks three different relationships:

1. The recorded source reference agrees with the actual source text.
2. The recorded export reference agrees with the actual JSON text.
3. Re-evaluating the source produces exactly the recorded export text.

These checks separate integrity of a supplied representation from the relationship between source and export. Canonical hashing of a string is not a promise that arbitrarily reformatted JSON receives the same identity. Nor does a source hash show that the policy author chose the right deny rules.

## Worked reasoning: a substituted export

Consider an illustrative review artifact containing source text for policy A and JSON copied from policy B. Both strings may be individually well formed, and both may have correctly recomputed content references. Hash checks alone cannot detect that substitution: they only bind each reference to its own bytes or value representation.

The normalization comparison closes that particular gap. `parse_nickel_source_evidence` evaluates the supplied source and compares the resulting JSON with `export-json`; disagreement is rejected. If someone instead changes the source and export coherently, this comparison can succeed. The remaining question is whether the resulting policy reference, contract envelope, and downstream admission context are the expected ones. This is why the source/export check cannot stand in for policy approval or runtime authority.

A production profile illustrates a different boundary. Its contract rejects placeholder candidate references and requires the profile's source-gate input list to equal the candidate input. Those are structural and relational authoring checks in [the production contracts](../../production-profile-contracts.ncl). They do not fetch candidate contents or attest that a reviewed release contains those contents.

## Diagnostics and review

`nickel_error` converts formatted Nickel text into `MoltenError::invalid_harness`, falling back to debug formatting if text formatting fails. That local helper is error conversion, not evidence of complete redaction. The toolchain guidance explicitly keeps diagnostic handling within existing bounded error and redaction paths and forbids treating secret-like diagnostic text as release evidence. Reviewers should trace the eventual output consumer before assuming the message is operator-safe.

Suggested verification, not executed for this documentation, is to run the existing cohort and contract-export checks in the pinned environment. For normalization changes, inspect both positive source/export agreement and a coherently rehashed but mismatched source/export pair. Review canonical artifacts before rendered logs, following the [proof workflow](../../proof-workflow.md).

## Limits and non-claims

The described code establishes explicit in-memory normalization and evidence relationships. It does not establish unrestricted evaluator equivalence, policy soundness, live shell correctness, freshness of referenced objects, or production readiness. This article adds no new contract and does not claim that any suggested verification command has run.

## Sources

- [Nickel toolchain cohort](../../nickel-toolchain.md)
- [Contract fixture inventory](../../nickel-contract-fixture-inventory.md)
- [Proof workflow](../../proof-workflow.md)
- [Cohort and export checks](../../../flake.nix)
- [Embedded evaluator and error conversion](../../../src/harness/parts/schema/p020/body.rs)
- [Policy source/export evidence](../../../src/harness/parts/schema/p019/body.rs)
- [Production profile contracts](../../production-profile-contracts.ncl)
