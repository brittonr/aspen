# Negative Contract Fixture Design

A negative contract fixture demonstrates a rejected configuration class, not merely a command that exits unsuccessfully. This article explains how to reason about that distinction using Molten's existing Nickel fixtures. It assumes basic Nickel record and contract syntax. See the [Technical companion](../README.md); the [fixture inventory](../../nickel-contract-fixture-inventory.md) defines the maintained fixture families and their coverage.

## Start from a valid construction path

Production fixtures share a [profile template](../../production-profile-fixtures/profile-template.ncl). Its `make` function fills fields from explicit overrides or reviewed defaults, then attaches `Contracts.ProductionProfileExport`. It supplies a deterministic conformance-only candidate reference and makes the default source-gate input list agree with that reference. The comment explicitly denies using this fixture candidate as release evidence.

This structure gives a negative fixture a controlled baseline. A case can alter the intended dimension while inheriting a coherent candidate reference, required adapters, layout, and metadata. The [valid fixture](../../production-profile-fixtures/valid.ncl) exercises the same constructor with an explicit candidate. A test that mutates an unrelated ad hoc record would have a weaker connection to the authoring path operators actually use.

The constructor is not a magical deep merge. For example, overriding `resource_limits` supplies that nested record as a whole. A negative fixture that unintentionally omits another required limit can fail before reaching its intended numeric boundary. Reviewing the fixture's entire overridden subrecord is therefore part of reviewing its claim.

## Scalar domains and relational domains

The [shared prelude](../../nickel-contract-prelude.ncl) defines positive integers as numbers that are integral and greater than zero. It validates BLAKE3 references against a lowercase hexadecimal pattern and distinguishes safe relative directories from absolute paths. These are lexical and value-domain predicates; a matching reference does not prove the object exists, and a matching path does not prove filesystem ownership or mount safety.

The [production contract](../../production-profile-contracts.ncl) adds relationships that individual fields cannot establish: layout directory names must be distinct, store bytes must be at least receipt bytes, recovery time must be at least delivery latency, profile identity must match the nested profile name, and source-gate inputs must equal the candidate singleton. It also rejects selected placeholder candidate references and requires the full declared adapter set.

A good negative matrix distinguishes these classes. Fractionality and zero are different failures even though both violate positive-integer admission. A valid-looking but mismatched candidate tests relational binding rather than reference spelling. Duplicate identities test a collection invariant, not mere array nonemptiness. This decomposition explains what a denial contributes without pretending each fixture proves the whole contract.

## Worked reasoning: a fractional queue limit

The checked-in [fractional-limit fixture](../../production-profile-fixtures/negative/fractional-limit.ncl) supplies `max_queue_depth = 1024.5` while retaining positive integral receipt/store sizes and coherent latency/recovery values. Following the predicates shows the intended failure: `std.number.is_integer` is false for the queue value, so the resource-limit record cannot satisfy the production export contract.

An illustrative weaker fixture would supply that fractional queue value but omit the candidate input in a path that requires customization. It might still fail, yet the observed nonzero exit would not demonstrate the queue contract. The existing fixture template avoids that particular confounder by supplying a conformance candidate. This does not remove the need to inspect diagnostics: missing imports, parser errors, or a broken shared template could also make a negative export fail.

The positive control matters for precisely this reason. The [Nix checks](../../../flake.nix) export positive production fixtures and compare expected exports in addition to evaluating negatives. A globally broken evaluator should not be mistaken for a successful negative matrix simply because every bad input failed.

## What the gate actually observes

`contract-export-drift-gate` defines positive and negative fixture helpers. A positive helper records failure when export fails. A negative helper records failure when export succeeds. The latter does not assert the exact diagnostic text or independently distinguish a contract rejection from another evaluator error. That is an explicit limit of exit-status evidence, not a reason to invent stronger guarantees.

The same check compares regenerated Cairn policy and plugin envelope JSON with checked-in generated files, and compares exported production resource limits with an expected JSON file. Export drift and negative admission are separate obligations: freshness of generated data cannot be inferred from one negative case, and byte agreement of a positive export cannot establish that invalid input is rejected.

The inventory explains diagnostic locality: scalar and required-field checks can identify a field, while whole-record predicates can report at the enclosing contract for cross-record relationships. Review should preserve the actual invariant rather than weaken it to obtain prettier blame. Exact wording is not the contract's semantic boundary.

## Verification and evidence limits

Suggested verification, not executed here, is to use the pinned Nickel cohort and existing contract-export drift check, inspecting positive controls alongside negative exits. For a changed fixture, review the intended mutation, baseline validity, import closure, and whether export forces the relevant contracted value. Where coverage feeds release decisions, follow the [proof workflow](../../proof-workflow.md): logs are diagnostic, while canonical verification receipts record target, command, toolchain, expected coverage kind, and decision.

The inventory declares no repository-owned fixture exempt from execution. This article neither adds an exemption nor reports an execution. A rejected Nickel export proves no absence of runtime mutation unless the applicable runtime boundary is separately exercised. It grants no authority, freshness, provenance, retention clearance, or deployment trust.

## Sources

- [Nickel contract fixture inventory](../../nickel-contract-fixture-inventory.md)
- [Nickel toolchain cohort](../../nickel-toolchain.md)
- [Proof workflow](../../proof-workflow.md)
- [Shared prelude predicates](../../nickel-contract-prelude.ncl)
- [Production contracts](../../production-profile-contracts.ncl)
- [Production fixture template](../../production-profile-fixtures/profile-template.ncl)
- [Valid production fixture](../../production-profile-fixtures/valid.ncl)
- [Fractional-limit negative fixture](../../production-profile-fixtures/negative/fractional-limit.ncl)
- [Cohort, fixture, and export-drift checks](../../../flake.nix)
