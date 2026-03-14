## ADDED Requirements

### Requirement: ADR directory structure

The system SHALL maintain Architecture Decision Records in `docs/adr/` with files named `NNN-<title>.md` where NNN is a zero-padded three-digit sequence number.

#### Scenario: New ADR creation

- **WHEN** a contributor creates a new ADR
- **THEN** the file is placed in `docs/adr/` with the next sequential number prefix

#### Scenario: ADR file listing

- **WHEN** a user lists `docs/adr/`
- **THEN** files sort chronologically by their numeric prefix

### Requirement: ADR format

Each ADR SHALL contain the following sections: Title (H1 with number and name), Status (one of: accepted, superseded, deprecated), Context, Decision, and Consequences.

#### Scenario: Complete ADR sections

- **WHEN** an ADR file is opened
- **THEN** it contains an H1 title with the ADR number, a Status line, and sections for Context, Decision, and Consequences

#### Scenario: Alternatives documentation

- **WHEN** an ADR documents a decision with considered alternatives
- **THEN** each alternative is annotated with (+) positive, (~) neutral, or (-) negative aspects

### Requirement: Retroactive foundational ADRs

The project SHALL include retroactive ADRs documenting at minimum these existing decisions: Iroh-only networking, vendored openraft, redb unified storage, FCIS pattern, Verus two-file architecture, Tiger Style resource bounds, Nickel for CI config, iroh-blobs content-addressed storage, ALPN-based protocol routing, and madsim deterministic simulation.

#### Scenario: Foundational decisions documented

- **WHEN** the ADR directory is checked for completeness
- **THEN** at least 10 ADRs exist covering the foundational architectural decisions listed above

#### Scenario: Retroactive ADR content

- **WHEN** a retroactive ADR is read
- **THEN** it explains the original context, the decision that was made, and the consequences observed since the decision was implemented
