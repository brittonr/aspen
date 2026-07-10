# Testing Harness Delta: Iroh Experiment Adoption Validation

## ADDED Requirements

### Requirement: Iroh experiment adoption fixtures validate trust boundaries
r[molten.iroh_experiments.adoption_validation] Molten MUST validate adopted Iroh experiment patterns with positive and negative fixtures that cover locator discovery, deterministic traversal sync, resumable chunk fetch, optional pkarr pointers, remote byte-source readback, HTTP3-over-Iroh readback, and non-authority boundaries.

#### Scenario: Positive fixture covers adopted path
- GIVEN a signed locator, deterministic traversal descriptor, verified fetched bytes, and local admission receipts
- WHEN the adoption fixture runs
- THEN it emits passing receipts for locator import, missing-set planning, fetch verification, admission, and readback or import
- AND the final report binds the canonical refs for each boundary.

#### Scenario: Locator-only import denial is covered
- GIVEN a tracker result, pkarr pointer, or peer announcement without fetched and verified bytes
- WHEN the negative fixture attempts artifact import
- THEN the fixture denies before registry mutation
- AND diagnostics state that locator evidence is hint-only.

#### Scenario: Hash mismatch denial is covered
- GIVEN a sender or remote byte source returns bytes whose digest does not match the expected canonical content ref or supported external digest
- WHEN the negative fixture evaluates the response
- THEN it emits a deny receipt
- AND no bytes are exposed, installed, pinned, or executed.

#### Scenario: HTTP transport authority regression is covered
- GIVEN an HTTP3-over-Iroh session reaches the optional readback adapter
- WHEN the request lacks canonical gateway visibility or capability evidence
- THEN the fixture denies before response bytes are exposed
- AND diagnostics state that HTTP transport evidence is not authority.

### Requirement: Iroh experiment design references are documented
r[molten.iroh_experiments.reference_docs] Molten SHOULD document `n0-computer/iroh-experiments` as a design reference and identify which patterns are adopted, deferred, or rejected in Molten terms.

#### Scenario: Reference docs separate adopted and rejected boundaries
- GIVEN an operator or reviewer reads the Iroh adoption documentation
- WHEN they compare Molten behavior to upstream experiments
- THEN the docs state that content discovery, deterministic traversal, optional pkarr pointers, HTTP3-over-Iroh readback, and remote byte-source hints are references only
- AND the docs state that Preserves receipts, BLAKE3 identities, capability/policy/resource gates, and deterministic replay remain Molten's normative boundaries.

#### Scenario: Deferred patterns remain non-blocking
- GIVEN HTTP3-over-Iroh or remote byte-source adapters are not yet implemented
- WHEN validation runs for the core discovery and traversal sync slice
- THEN the deferred adapter requirements can remain unimplemented without weakening locator hint-only or deterministic sync gates.
