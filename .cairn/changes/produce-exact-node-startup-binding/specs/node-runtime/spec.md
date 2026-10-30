## ADDED Requirements

### Requirement: Producer measures exact executed build
r[molten.startup_binding.producer] The producer MUST execute a reviewed same-source Nix candidate and Octet build profile and bind measured compiled executable bytes, source, complete build input closure, actual toolchain, policy, profile and complete retained gate cohorts. It MUST NOT substitute package version or matching caller references for those measurements.

#### Scenario: Executed candidate produces detached binding
- GIVEN one immutable source selected for both actual candidate compilation and real Octet runs
- WHEN the producer executes its strict gate and measures the realized outputs
- THEN a versioned detached binding identifies exact bytes without embedding an executable selfhash

#### Scenario: Strict gate does not pass
- GIVEN real Octet artifacts whose strict gate denies or whose files cannot be read
- WHEN the producer runs
- THEN no admissible passing producer output is emitted

### Requirement: Independent producer authentication
r[molten.startup_binding.authentication] Fresh startup MUST authenticate the complete producer output against an independently selected full public key authorized for the reviewed producer profile, verify actual content and actual executing candidate, and MUST reject unsigned, synthetic, stale, tampered, wrong-candidate and evidence-self-trusted output. Nix local-build or content-addressed trust alone MUST NOT suffice.

#### Scenario: Experimental real positive control
- GIVEN an actual successfully executed producer output and a separately provisioned explicitly experimental signer selected independently of evidence
- WHEN normal startup authenticates and measures that exact executable and complete cohort
- THEN source admission can pass without claiming production operator approval or compiler correctness

#### Scenario: Self-trust is not authority
- GIVEN a binding carrying its own key or only local-build/content-addressed trust
- WHEN fresh startup evaluates authentication
- THEN startup rejects before destructive restart preparation, lifecycle writes or activation and preserves lock and prior evidence

### Requirement: Versioned replay and operator refresh
r[molten.startup_binding.refresh] Incompatible binding or cohort rules MUST require a new version. Historical receipts MUST remain historical and MUST NOT acquire fresh producer authenticity during replay. Operator documentation MUST explain refresh, independent trust selection and explicit nonclaims.

#### Scenario: Candidate changes
- GIVEN executable, source, dependency, toolchain, policy, profile or gate artifact bytes have changed
- WHEN an operator requests fresh startup
- THEN the operator must obtain a new real producer output and independently select its authenticated binding instead of relabeling old evidence
