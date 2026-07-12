## ADDED Requirements

### Requirement: Store capability roots are operational authority
r[molten.chunk_store.cap_std_operational_roots] Molten MUST require typed capability roots, or narrow filesystem ports backed by them, at every artifact, chunk, retention, local dataspace, and local exchange operation that opens, lists, reads, writes, renames, or removes local filesystem objects. A type alias or unused root constructor MUST NOT count as capability adoption while the effectful operation still accepts an ambient root path.

#### Scenario: In-root store operation uses its supplied authority
- GIVEN the outer shell has opened a declared local store root
- WHEN a store adapter reads or mutates a validated relative locator
- THEN the adapter MUST perform the operation through the supplied capability root
- AND it MUST NOT reopen the child through the ambient filesystem namespace.

#### Scenario: Alias-only integration is rejected
- GIVEN a module exposes a capability-root alias but its production operation joins an ambient path and calls `std::fs`
- WHEN capability-boundary validation runs
- THEN validation MUST fail and identify the operation as not yet converted.

### Requirement: Ambient store authority is confined to bootstrap shells
r[molten.chunk_store.cap_std_ambient_boundary] Molten MUST confine creation or opening of operator-selected ambient store roots to reviewed CLI, runtime, or adapter bootstrap shells. Reusable store logic MUST accept existing authority and MUST NOT call ambient root-open, canonicalization, or direct `std::fs` child operations.

#### Scenario: Explicit operator root is opened once
- GIVEN an operator supplies a local store root to a command
- WHEN the command enters the store subsystem
- THEN the outer shell MAY create and open that root with explicit ambient authority
- AND all descendant operations MUST receive capability-derived authority.

#### Scenario: Locator cannot trigger ambient reacquisition
- GIVEN a manifest or remote envelope contains a content ref, URL, ticket, absolute path, or parent traversal
- WHEN reusable store logic evaluates it
- THEN the value MUST NOT be passed to an ambient root-open or direct filesystem API
- AND invalid local locator use MUST deny before local bytes are accessed.

### Requirement: Capability-relative enumeration does not leak host paths
r[molten.chunk_store.cap_std_relative_enumeration] Molten MUST enumerate store directories through the capability root and return bounded, deterministically ordered logical names or typed relative entries. Store callers MUST reopen selected entries through the same root and MUST NOT use host paths obtained from ambient directory entries.

#### Scenario: Stable in-root listing passes
- GIVEN a capability-rooted directory contains valid store entries
- WHEN the adapter lists and consumes those entries
- THEN it MUST sort bounded logical names deterministically
- AND each consumed entry MUST be reopened relative to the original capability.

#### Scenario: Symlinked entry cannot become an ambient reopen
- GIVEN an enumerated entry is a symlink or is replaced before it is consumed
- WHEN the adapter attempts to read or remove it
- THEN capability-relative resolution MUST prevent escape from the declared root
- AND the adapter MUST NOT fall back to an entry host path.

### Requirement: Path-oriented backends receive capability-acquired handles
r[molten.chunk_store.cap_std_backend_handles] Molten MUST open fixed backend files, including Redb files, beneath the relevant capability root and pass an already-open file handle or capability-preserving backend into the storage engine whenever the engine supports that interface. Backend setup MUST NOT reconstruct an ambient path from the capability root.

#### Scenario: Redb index opens from an in-root file handle
- GIVEN the chunk index uses a fixed reviewed database leaf under a chunk root
- WHEN the index is created or reopened
- THEN Molten MUST acquire the file through the chunk capability
- AND pass that acquired handle to the Redb file or backend constructor.

#### Scenario: Backend leaf substitution is denied
- GIVEN an attacker substitutes a symlink or non-regular object for the backend leaf
- WHEN backend acquisition runs
- THEN the operation MUST deny before the backend can access an object outside the declared root.

### Requirement: Converted adapters have a scoped ambient-filesystem regression gate
r[molten.chunk_store.cap_std_regression_gate] Molten MUST maintain a syntax-aware blocking gate for converted store adapter scopes that rejects direct ambient filesystem calls and ambient root reacquisition. The gate MUST have positive fixtures for prohibited adapter calls and negative fixtures for reviewed outer-shell bootstrap and adversarial test setup.

#### Scenario: Ambient call in converted adapter fails
- GIVEN a converted store adapter adds a direct `std::fs` read, write, listing, or removal call
- WHEN the structural authority gate runs
- THEN the gate MUST fail with a scoped ambient-filesystem diagnostic.

#### Scenario: Explicit bootstrap remains permitted
- GIVEN a reviewed CLI shell opens the operator-selected top-level root and immediately delegates to a typed adapter
- WHEN the structural authority gate runs
- THEN the bootstrap fixture MUST pass without permitting the same call in store internals.

### Requirement: Capability-store conversion has positive and negative evidence
r[molten.chunk_store.cap_std_conversion_validation] Molten MUST verify capability-rooted artifact, chunk, retention, dataspace, exchange, enumeration, and backend-handle behavior with positive tests and negative tests for traversal, absolute paths, locator confusion, symlink escape, replacement races, wrong-root handles, non-regular entries, and missing authority.

#### Scenario: Complete conversion evidence passes
- GIVEN all targeted store effects consume operational capability roots
- WHEN focused tests and structural gates run
- THEN valid in-root workflows MUST pass
- AND every declared invalid or escaping workflow MUST deny before out-of-root access or mutation.

#### Scenario: Missing negative coverage blocks closeout
- GIVEN positive store workflows pass but one declared escape or authority-confusion class lacks executable coverage
- WHEN the change is evaluated for archive
- THEN closeout MUST remain blocked with the missing negative class identified.
