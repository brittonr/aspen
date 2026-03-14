# 7. Nickel for CI Configuration

**Status:** accepted

## Context

Aspen's CI/CD system needs a configuration language for defining pipelines, stages, jobs, triggers, and executor requirements. Common choices are YAML (GitHub Actions, GitLab CI), HCL (Terraform), Starlark (Bazel), or a general-purpose language.

Nickel is a configuration language designed for generating correct configurations. It has a type system with contracts, merge semantics for composition, and first-class functions. It compiles to JSON, making it easy to consume from Rust via serde.

## Decision

Use Nickel as the CI pipeline configuration language. Pipeline definitions are `.ncl` files that are validated against a JSON Schema generated from Rust types via `schemars`.

The schema generation pipeline:

1. Rust types (`JobConfig`, `StageConfig`, `PipelineConfig`) derive `schemars::JsonSchema`
2. A build step exports the schema as JSON
3. Nickel contracts are generated from or aligned with the JSON Schema
4. User-written `.ncl` files are validated at pipeline load time

Alternatives considered:

- (+) YAML: universal familiarity, used by GitHub Actions / GitLab CI
- (-) YAML: no type system, stringly-typed, gotchas (Norway problem, implicit type coercion)
- (+) HCL: proven for infrastructure (Terraform)
- (-) HCL: limited composability, not designed for CI pipelines
- (+) Starlark: programmable, used by Bazel/Buck
- (-) Starlark: Python-like syntax unfamiliar to Nix users, limited ecosystem
- (+) Nickel: typed contracts, merge semantics, compiles to JSON, Nix-adjacent community
- (-) Nickel: small ecosystem, fewer developers familiar with it
- (+) Nix language itself: already in the ecosystem
- (-) Nix: evaluation requires the Nix daemon, heavyweight for config validation

## Consequences

- Pipeline configs get type-checked before execution via Nickel contracts
- Schema generation from Rust types keeps the config schema in sync with the implementation
- Nickel's merge semantics allow pipeline composition (base config + per-project overrides)
- Contributors need to learn Nickel syntax — mitigated by examples and schema documentation
- The `schemars` dependency is already used across 9 crates, so schema generation is not a new pattern
