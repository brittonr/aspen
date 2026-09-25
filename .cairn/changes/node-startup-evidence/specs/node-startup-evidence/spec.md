# Node startup evidence

## ADDED Requirements

### Requirement: Exact portable inputs [r[molten.startup_evidence.inputs]]

The verifier MUST admit an exact operator-selected cohort before reading evidence members. It MUST keep source, build compiler, Octet tools, runtime binary, and evidence identities distinct. Evidence metadata MUST NOT select arbitrary paths or authorize itself.

#### Scenario: Wrong cohort

GIVEN a bundle whose descriptor differs from the approved digest
WHEN verification starts
THEN verification MUST deny before any evidence-member read or node-state effect.

### Requirement: Explicit strict evaluation [r[molten.startup_evidence.strict]]

The verifier MUST reconstruct strict Octet gate results from complete, bounded, measured input bytes for the separate `molten-node` executable and its selected first-party package set. It MUST derive metadata from the selected source context, not ambient cwd. The build-input declaration MUST bind the complete Rust source inventory claimed for the binary, including declared compiler dependencies, and MUST reject an omitted source even when Octet counts are zero. Synthetic receipts, warning budgets, incomplete membership, and caller-asserted clean counts MUST NOT replace the full checks. A build-input declaration MUST NOT be mistaken for independent proof that the compiler ran, the declared closure is exhaustive, or the resulting executable matches it.

#### Scenario: Stale source metadata

GIVEN status metadata for a different Cargo or dylint snapshot
WHEN portable verification evaluates the bundle
THEN it MUST reject the stale context without changing the workspace or node state.

#### Scenario: Missing compiled source in the build declaration

GIVEN an otherwise clean strict snapshot whose declared compiler input union omits one inventoried Rust source
WHEN portable verification evaluates the bundle
THEN it MUST deny before any normal-node startup or listener effect.

### Requirement: Verification is not startup authority [r[molten.startup_evidence.scope]]

A verification-only report MUST NOT authorize startup, claim actual tool execution, or promote VM/package/release support. Normal startup MUST remain blocked until a separately evidenced real execution and lifecycle admission route exists.

#### Scenario: Structural fixture passes

GIVEN internally consistent test bytes and an explicitly selected test cohort
WHEN the pure verifier accepts their structure and identities
THEN its result MUST remain verification-only and MUST NOT start a node or listener.
