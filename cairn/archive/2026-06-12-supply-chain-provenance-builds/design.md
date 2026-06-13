## Context

A Blake3 artifact id identifies bytes. It does not answer who built them, from what source, using which toolchain, with which dependencies, under which review, or whether the build is reproducible. Molten needs provenance because artifacts can be executable code, policy gates, schema definitions, migrations, or docs that influence decisions.

## Goals

- Represent artifact provenance as canonical evidence.
- Bind source refs, dependency closures, toolchain artifacts, build parameters, builder identity, signatures, review records, and test/transcript results to artifact ids.
- Support reproducible build verification and mismatch diagnostics.
- Gate installation/execution/use by provenance policy.
- Make provenance searchable and visible through catalog/MCP according to policy.

## Non-Goals

- Do not require all artifacts to have the same trust level.
- Do not claim content hashing alone is sufficient trust.
- Do not require fully reproducible builds before any sandboxed experiment can run.
- Do not let provenance bypass runtime sandboxing or effect admission.

## Provenance record

A provenance record should include:

- artifact id and kind,
- source refs and source hash,
- dependency closure hash,
- toolchain/compiler/builder artifact refs,
- build parameters and environment policy,
- builder principal/key refs,
- signatures/attestations,
- review records and approvals,
- transcript/test/evaluation-cache refs,
- vulnerability/advisory refs where applicable,
- policy decisions and receipts.

## Trust states

Policy can classify artifacts as:

- `unknown`
- `source_known`
- `builder_attested`
- `reviewed`
- `reproducible_verified`
- `sandbox_only`
- `policy_trusted`
- `denied`

Trust state is contextual: an artifact may be allowed for local tests but denied as a production policy predicate.

## Reproducible builds

A reproducible build record declares source, dependency closure, toolchain, build params, and expected output artifact id. Verification reruns or validates the build and emits a receipt. Mismatch diagnostics include expected/actual artifact ids and differing provenance inputs.

## Policy integration

Nickel contracts define static provenance requirements per artifact kind and environment. Steel predicates may perform reviewed dynamic trust checks, such as advisory lookup, only as admitted policy backends. Basalt enforces installer/executor authority. Octet/Valence evidence references provenance and replay data. Cairn validates receipts.

## Open Questions

- Which attestation format should be supported first?
- Should Nix derivation hashes be first-class toolchain/build refs for Rust/Wasm artifacts?
- What minimum provenance is required for Steel predicates that make policy decisions?
