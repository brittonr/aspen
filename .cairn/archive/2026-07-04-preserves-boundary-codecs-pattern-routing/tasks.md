# Tasks: preserves-boundary-codecs-pattern-routing

## Phase 1: Typed boundary codecs

- [x] [serial] r[molten.preserves_boundary_codegen.typed_codecs] Add typed/schema-backed codec wrappers for the next adopted Preserves boundary families while preserving canonical record labels and field order.
- [x] [parallel] r[molten.preserves_boundary_codegen.strict_decode] Centralize strict canonical byte admission for external Preserves bytes before schema or semantic validation.
- [x] [parallel] r[molten.preserves_boundary_codegen.schema_ref_evidence] Bind schema artifact refs, decoded value refs, and codec decisions into boundary validation receipts or diagnostics.

## Phase 2: Pattern routing

- [x] [serial] r[molten.preserves_boundary_codegen.pattern_ast] Define a bounded canonical Preserves pattern AST shared by dataspace routing, policy-visible matching, and tests.
- [x] [serial] r[molten.preserves_boundary_codegen.pattern_routing] Replace equality-only local Observe routing where adopted patterns are enabled, while preserving deterministic initial assertion and retraction delivery.

## Phase 3: Fixtures and validation

- [x] [parallel] r[molten.preserves_boundary_codegen.fixture_corpus] Add canonical positive and negative fixtures for strict decode, typed codec roundtrip, schema ref binding, and pattern routing.
- [x] [serial] r[molten.preserves_boundary_codegen.no_schema_authority] Add denial tests proving schema pass evidence does not grant authority, policy, provenance, resource, transport, source-gate, retention, or execution trust.
- [x] [serial] r[molten.preserves_boundary_codegen.typed_codecs] r[molten.preserves_boundary_codegen.strict_decode] r[molten.preserves_boundary_codegen.pattern_routing] Run focused Preserves boundary and runtime dataspace tests.
