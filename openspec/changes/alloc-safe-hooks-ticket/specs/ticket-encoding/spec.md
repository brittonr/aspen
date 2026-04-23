## ADDED Requirements

### Requirement: Hook ticket encoder never substitutes an empty payload
ID: ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload

`AspenHookTicket` encoding SHALL either produce the correct serialized payload or fail loudly on impossible serializer bugs. It SHALL NOT replace a failed encoding with `Vec::new()` or any other default payload bytes.

#### Scenario: Hook ticket encoder fails loudly on serializer invariant break
ID: ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fails-loudly-on-serializer-invariant-break

- **GIVEN** `AspenHookTicket` uses a trait-imposed `to_bytes() -> Vec<u8>` path
- **WHEN** postcard serialization hits an impossible invariant break while encoding a bounded ticket payload
- **THEN** the process SHALL fail loudly with contextual diagnostics
- **AND** it SHALL NOT continue by minting an empty or default payload ticket

#### Scenario: Hook ticket encoder fail-loud proof is reviewable
ID: ticket.encoding.hook-ticket-encoder-never-substitutes-an-empty-payload.hook-ticket-encoder-fail-loud-proof-is-reviewable

- **GIVEN** the hook ticket encoder fallback removal is complete
- **WHEN** the change is prepared for review
- **THEN** `openspec/changes/alloc-safe-hooks-ticket/evidence/final-validation.md` SHALL include a deterministic source-audit command proving `impl Ticket for AspenHookTicket` uses an `expect` fail-loud path and no `Vec::new()` fallback
- **AND** `openspec/changes/alloc-safe-hooks-ticket/evidence/implementation-diff.txt` SHALL show the fallback removal in the reviewed diff
- **AND** `openspec/changes/alloc-safe-hooks-ticket/verification.md` SHALL map the encoder requirement to those artifacts
