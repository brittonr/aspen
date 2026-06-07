# Runtime Spine Delta: Supply-Chain Provenance Builds

### Requirement: Provenance records bind supply-chain evidence
r[molten.provenance.record_model] Molten MUST represent provenance records as canonical artifacts that bind artifact identity, source refs, dependency closure, toolchain refs, build parameters where available, builder identity, signatures/review/test/source-gate refs, policy refs, and trust state.

#### Scenario: Provenance record has explicit evidence
- GIVEN an artifact with source, dependency, builder, review, test, source-gate, and policy refs
- WHEN provenance is materialized
- THEN Molten emits a canonical provenance record
- AND the artifact content hash alone is not treated as provenance trust.

### Requirement: Provenance trust states are contextual
r[molten.provenance.trust_states] Molten MUST distinguish unknown, source-known, builder-attested, reviewed, reproducible-verified, sandbox-only, policy-trusted, and denied trust states by evaluation profile.

#### Scenario: Trust state depends on profile
- GIVEN a sandbox-only provenance record
- WHEN node-control evaluates it
- THEN Molten denies that trust state for node-control
- AND local test profiles may admit it when policy allows.

### Requirement: Content addressing is not trust
r[molten.provenance.hash_not_trust] Molten MUST document and enforce that content addressing proves identity but not origin, review, build reproducibility, authority, or policy trust.

#### Scenario: Hash-only evidence is insufficient
- GIVEN an artifact ref with no provenance evidence
- WHEN install or run admission evaluates it
- THEN Molten denies provenance admission
- AND no side effect is authorized by the hash alone.

### Requirement: Provenance is visible through catalog summaries
r[molten.provenance.catalog_view] Molten SHOULD expose provenance records and receipts through catalog or MCP summaries with visibility filtering and redaction.

#### Scenario: Provenance summary is read-only
- GIVEN a provenance record or receipt in the ledger
- WHEN an operator views it through read-only tooling
- THEN Molten summarizes its trust state, decision, and artifact refs
- AND the summary is not itself pass evidence.

### Requirement: Provenance gates installation and execution
r[molten.provenance.install_policy] Molten MUST gate artifact installation and execution by provenance policy for the artifact kind and environment before side effects.

#### Scenario: Missing provenance denies install
- GIVEN an install request with no admitted provenance
- WHEN node-control dispatch evaluates the request
- THEN Molten emits a denying provenance receipt
- AND the registry is not mutated.

### Requirement: Stronger provenance is required for sensitive artifacts
r[molten.provenance.policy_artifacts] Molten MUST require stronger provenance for policy predicates, migration recipes, production executables, and other safety-critical artifacts than for local diagnostic artifacts.

#### Scenario: Sensitive artifact needs stronger evidence
- GIVEN a production policy artifact with only low-trust provenance
- WHEN install admission evaluates it
- THEN Molten denies the artifact for production use
- AND diagnostics identify the missing stronger provenance evidence.

### Requirement: Provenance decisions emit receipts
r[molten.provenance.receipts] Molten MUST emit canonical receipts for provenance evaluation, approval, denial, trust-state changes, and build verification decisions.

#### Scenario: Provenance denial is receipted
- GIVEN mismatched or missing provenance evidence
- WHEN admission evaluates the evidence
- THEN Molten emits a canonical denying provenance receipt
- AND diagnostics bind the affected artifact ref.

### Requirement: Remote sync validates provenance before execution
r[molten.provenance.remote_sync] Molten MUST validate provenance requirements during remote artifact sync before execution or installation on the receiver.

#### Scenario: Remote artifact lacks provenance
- GIVEN a remotely synced executable artifact without admitted provenance
- WHEN the receiver considers execution
- THEN Molten denies execution before side effects
- AND records provenance diagnostics.

### Requirement: Provenance install tests cover fail-closed behavior
r[molten.provenance.install_tests] Molten MUST include tests proving artifacts missing required provenance are denied in production or node-control install policy.

#### Scenario: Missing install provenance test
- GIVEN a node-control install without provenance
- WHEN tests run
- THEN they prove no registry mutation occurs
- AND a denying provenance receipt is emitted.

### Requirement: Sandbox provenance tests cover restricted profiles
r[molten.provenance.sandbox_tests] Molten MUST include tests proving low-trust artifacts may run only under restricted sandbox or local-test profiles when policy admits them.

#### Scenario: Sandbox-only denial test
- GIVEN sandbox-only provenance for a node-control artifact
- WHEN tests run
- THEN they prove node-control denies it
- AND local test evaluation remains explicit.

### Requirement: Provenance properties cover monotonic trust boundaries
r[molten.provenance.property_tests] Molten SHOULD include property tests for provenance-context monotonicity and the invariant that hashes alone do not grant trust.

#### Scenario: Stronger evidence is monotonic
- GIVEN generated provenance contexts with increasing evidence strength
- WHEN property checks compare admission outcomes
- THEN adding valid evidence does not silently weaken diagnostics
- AND hash-only contexts remain denied.

### Requirement: Reproducible build records bind expected artifacts
r[molten.provenance.build_record] Molten MUST represent reproducible build records as canonical Preserves artifacts that bind the expected artifact ref, source refs, dependency closure ref, toolchain refs, build params, builder ref, policy refs, and evidence refs.

#### Scenario: Build record is canonical
- GIVEN explicit source, dependency closure, toolchain, build parameter, builder, policy, evidence, and expected artifact refs
- WHEN an operator materializes a provenance build record
- THEN Molten emits a `provenance-build-record-v1` artifact
- AND the artifact ref is derived from canonical Preserves bytes.

### Requirement: Build records carry Nix derivation evidence refs
r[molten.provenance.nix_refs] Provenance build records SHOULD carry explicit Nix derivation or toolchain evidence refs when such evidence is available, without treating those refs as authority or policy grants.

#### Scenario: Nix refs are recorded as evidence
- GIVEN a build record command with one or more Nix derivation refs
- WHEN the record is emitted
- THEN the `provenance-build-record-v1` artifact contains those refs under the build provenance evidence boundary
- AND downstream authority, policy, resource, execution, and source-gate checks remain separate.

### Requirement: Build verification emits canonical receipts
r[molten.provenance.verify_build] Molten MUST verify build records by comparing the expected artifact ref with the actual artifact ref and emitting canonical `provenance-build-verify-receipt-v1` receipts.

#### Scenario: Matching artifact verifies
- GIVEN a build record whose expected artifact ref equals the actual artifact ref
- WHEN an operator runs build verification
- THEN Molten emits a passing build verification receipt
- AND the receipt binds the build record ref and actual artifact ref.

### Requirement: Mismatched builds diagnose expected and actual refs
r[molten.provenance.mismatch_diagnostics] Build verification receipts MUST diagnose mismatched expected and actual artifact refs before any caller treats the build as reproducibly verified.

#### Scenario: Mismatch denies
- GIVEN a build record whose expected artifact ref differs from the actual artifact ref
- WHEN an operator runs build verification
- THEN Molten emits a denying build verification receipt
- AND the receipt diagnostics name both expected and actual artifact refs.

### Requirement: Build verification tests cover pass and mismatch
r[molten.provenance.repro_tests] Molten MUST include focused tests for build verification pass receipts, mismatch denial receipts, and CLI summaries.

#### Scenario: CLI build verification is covered
- GIVEN CLI coverage for build-record and verify-build commands
- WHEN the test suite runs
- THEN it proves matching artifacts pass
- AND mismatching artifacts deny with canonical summaries.
