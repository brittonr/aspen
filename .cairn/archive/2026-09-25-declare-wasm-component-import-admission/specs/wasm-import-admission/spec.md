# Wasm Import Admission Specification

## ADDED Requirements

### Requirement: The import manifest is complete and declared

r[aspen.wasm_import_admission.manifest] Every Wasm component admitted by the shared Molten runtime MUST declare its complete import table, and admission MUST reject any component whose manifest is missing, partial, or unreadable.

#### Scenario: Complete manifest is supplied

- GIVEN a caller supplies a complete typed import manifest for a component
- WHEN manifest validation runs
- THEN the core MUST return deterministic admission facts or typed blockers
- AND it MUST perform no file, process, or runtime instantiation effect.

#### Scenario: Manifest is partial

- GIVEN a component manifest omits one or more actual imports
- WHEN manifest validation runs
- THEN the core MUST reject the component before instantiation.

### Requirement: Imports match the declared WIT world

r[aspen.wasm_import_admission.surface] A component MUST export and import exactly the interfaces of its declared WIT world, and any drift or undeclared ambient capability MUST fail admission before component instantiation or execution.

#### Scenario: World and imports match exactly

- GIVEN a component matches the declared WIT world with no extra imports
- WHEN world admission runs
- THEN the runtime MAY proceed to instantiation.

#### Scenario: Component imports an ambient capability

- GIVEN a component imports WASI or any undeclared host capability
- WHEN world admission runs
- THEN the runtime MUST reject before instantiation.

### Requirement: Admission is a pure core decision

r[aspen.wasm_import_admission.admission] Manifest-shape validation, world matching, import-set comparison, and receipt payload construction MUST be pure deterministic decisions over typed artifact facts supplied by the shell.

#### Scenario: Shell supplies complete artifact facts

- GIVEN the shell supplies complete bounded artifact and tool-identity facts
- WHEN admission evaluation runs
- THEN the core MUST return a deterministic verdict
- AND it MUST perform no host, network, clock, or runtime effect.

### Requirement: Receipts bind import and tool identities

r[aspen.wasm_import_admission.evidence] Admission receipts MUST bind the import manifest, WIT world, import-set identity, extraction tool and version, verifier identity, and verdict while excluding raw artifact bytes, logs, and runtime traces beyond declared bounds.

#### Scenario: Receipt candidate carries raw bytes beyond bound

- GIVEN a receipt candidate packs raw artifact bytes or runtime traces beyond bounds
- WHEN receipt validation runs
- THEN validation MUST reject or redact the payload before persistence.

### Requirement: Non-claims are preserved

r[aspen.wasm_import_admission.nonclaims] Passing import admission MUST NOT claim sandbox completeness, component correctness, semantic equivalence, guest safety, production readiness, or release eligibility.

#### Scenario: Passing admission is promoted to sandbox proof

- GIVEN a passing admission receipt is labeled as sandbox containment or component correctness
- WHEN non-claim validation runs
- THEN the evidence MUST fail.

### Requirement: The rail has positive and negative fixtures

r[aspen.wasm_import_admission.fixtures] The import-admission rail MUST include positive exact-manifest, exact-world, and exact-import-set fixtures plus negative drifting-world, undeclared-import, tool-drift, overclaim, and malformed-receipt fixtures.

#### Scenario: Rail is proposed for product use

- GIVEN manifests, worlds, receipts, docs, and fixtures are complete
- WHEN focused Cargo, octet, Cairn, and Nix validation runs
- THEN every positive fixture MUST pass
- AND every negative fixture MUST fail at its declared boundary.
