# Project Delta: Semantic Module Layout

### Requirement: Source splits use semantic boundaries
r[molten.modularity.semantic_modules.named_boundaries] Rust source modules SHOULD prefer semantically named submodules over ordinal `include!` shards when the code is repository-owned and manually reviewed.

#### Scenario: Named module reveals review boundary
- GIVEN a large repository-owned module selected for modularity cleanup
- WHEN the module is split or reorganized
- THEN each new source file name identifies a domain responsibility such as model, codec, admission, receipts, store, runner, shell, or tests
- AND existing public module paths remain available unless a separate public API change owns the break

#### Scenario: Ordinal shard expansion is blocked
- GIVEN a repository-owned module still using ordinal body shards
- WHEN new manually reviewed behavior is added to that module
- THEN the behavior is placed in a semantic module or the change records an explicit generated-code or staged-migration exemption

### Requirement: Semantic splits preserve functional core boundaries
r[molten.modularity.semantic_modules.functional_core] Semantic module refactors MUST keep deterministic core logic separate from filesystem, process, network, clock, environment, and CLI rendering effects.

#### Scenario: Pure decision moves behind in-memory API
- GIVEN parsing, validation, admission, or receipt-decision logic is moved during a semantic split
- WHEN focused tests exercise the moved logic
- THEN the tests can call the core with in-memory inputs and observe structured outputs without standing up adapters or CLI commands

#### Scenario: IO leakage is rejected
- GIVEN a module marked as a functional core after the split
- WHEN reviewers inspect that module
- THEN filesystem traversal, process execution, network IO, wall-clock reads, environment reads, and direct stdout or stderr rendering are absent or explicitly moved to the shell

### Requirement: Remaining ordinal shards are explicit exemptions
r[molten.modularity.semantic_modules.exemptions] Remaining ordinal shards MAY exist only when they are generated, externally constrained, or staged for later migration, and the owner SHOULD record the exemption near the entry point or in the change evidence.

#### Scenario: Generated shard remains reviewable
- GIVEN a generated or machine-partitioned shard remains after cleanup
- WHEN reviewers inspect the module boundary
- THEN the exemption identifies why semantic naming was not applied and what stable generated input or review artifact owns the content
