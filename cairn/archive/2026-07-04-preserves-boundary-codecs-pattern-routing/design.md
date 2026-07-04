# Design: Preserves boundary codecs and pattern routing

## Scope

This change deepens the Preserves spine. It covers typed/schema-backed boundary codecs, strict canonical byte admission, schema/value ref evidence, a bounded reusable Preserves pattern AST, and a fixture corpus that makes the Preserves contract executable.

## Proof checklist

- **Proof claim**: every adopted boundary rejects non-canonical or malformed data before semantic admission, and adopted typed codecs roundtrip to the same canonical Preserves refs as the existing public records.
- **Out of scope**: changing public record labels or field order without a separate compatibility/migration change; granting authority from schema validation; replacing semantic admission with schema checks.
- **Trusted assumptions**: BLAKE3 refs over packed canonical Preserves bytes remain the identity contract; existing schema ids remain normative until explicit migrations update them.
- **Positive evidence**: generated/typed codecs parse and unparse valid fixtures with unchanged canonical refs; strict canonical decode accepts canonical bytes; admitted patterns route expected assertions.
- **Negative evidence**: non-canonical packed bytes, wrong labels, unsupported versions, missing fields, malformed refs, missing schema refs, unsupported patterns, and ambiguous bindings deny before side effects.
- **Canonical refs**: schema artifact ref, input bytes ref, decoded value ref, typed DTO ref where emitted, pattern AST ref, route result ref, and boundary receipt ref.
- **Regeneration command**: focused Preserves boundary, schema, and runtime dataspace pattern tests.

## Functional core

Core functions accept immutable bytes, `IOValue`s, schema specs, typed DTOs, pattern ASTs, and candidate values. They return typed parse results, denials, routing matches, diagnostics, or receipt values. They do not read files, network state, clocks, environment variables, databases, or policy stores.

## Imperative shell

Shell code reads schema artifacts and fixtures, invokes core validation, writes receipts, and renders diagnostics. It remains responsible for filesystem and CLI concerns only.

## Boundary sequencing

External bytes follow this order:

```text
bytes
  -> strict canonical decode
  -> value/content ref check
  -> schema/typed codec validation
  -> semantic admission
  -> side effect or durable import
```

A later stage cannot recover from an earlier denial. Schema pass evidence never grants authority, provenance, policy, resource, transport, source-gate, retention, or execution trust.

## Migration strategy

Adopt boundary families incrementally. Each migrated family gets a typed codec wrapper, stable-hash tests against representative current fixtures, and negative tests before it becomes required by semantic admission.

## Non-goals

- No broad public schema rename in this change.
- No dynamic runtime schema compilation in admission paths.
- No unbounded pattern language or regex-like matching.
