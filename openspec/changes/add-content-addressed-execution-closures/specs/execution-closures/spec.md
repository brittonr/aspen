## ADDED Requirements

### Requirement: Content-Addressed Execution Closure Manifest [r[execution-closures.manifest]]

Aspen MUST define a versioned execution closure manifest whose canonical bytes are addressed by BLAKE3 and whose identity is independent of mutable job, service, or pipeline names.

#### Scenario: Closure manifest has stable identity [r[execution-closures.manifest.stable-identity]]

- GIVEN a closure manifest with artifact identity, dependency graph, schema hashes, runtime target, capability requirements, and provenance
- WHEN Aspen serializes it canonically and computes its BLAKE3 digest
- THEN the digest MUST be the closure hash used by admission, transfer, execution, and receipts

#### Scenario: Name does not change closure identity [r[execution-closures.manifest.name-independent]]

- GIVEN two job or service names reference the same canonical closure manifest
- WHEN their closure hashes are computed
- THEN both references MUST resolve to the same closure hash

### Requirement: Closure Dependency Transfer [r[execution-closures.dependency-transfer]]

Aspen workers MUST fetch missing closure dependencies by immutable hash before claiming a closure is executable.

#### Scenario: Worker fetches missing dependencies [r[execution-closures.dependency-transfer.fetch-missing]]

- GIVEN a worker is assigned a closure whose dependency graph includes blobs not present locally
- WHEN the worker prepares execution
- THEN it MUST request missing dependencies over the existing Aspen blob/Iroh path
- AND it MUST verify received bytes against their hashes before execution

#### Scenario: Missing dependency blocks execution [r[execution-closures.dependency-transfer.missing-blocks]]

- GIVEN a closure dependency cannot be fetched or fails hash verification
- WHEN the worker evaluates the closure
- THEN execution MUST be rejected before starting the runtime target
- AND the receipt MUST record a bounded missing-or-invalid dependency diagnostic

### Requirement: Closure Admission Uses Capability Requirements [r[execution-closures.capability-admission]]

Aspen MUST evaluate closure-declared capability requirements before dispatching or executing the closure.

#### Scenario: Required capability admitted [r[execution-closures.capability-admission.admitted]]

- GIVEN a closure declares required effects or capability handles
- WHEN the scheduler or worker has valid authorization for those requirements
- THEN admission MAY continue and the receipt MUST include a redacted capability proof summary

#### Scenario: Required capability denied [r[execution-closures.capability-admission.denied]]

- GIVEN a closure requests a capability not granted to the job, service, or caller
- WHEN admission runs
- THEN Aspen MUST reject execution before dependency fetch side effects beyond metadata validation
- AND the receipt MUST NOT expose raw tokens, tickets, keys, or secrets

### Requirement: Closure Execution Receipts [r[execution-closures.receipts]]

Aspen MUST emit bounded receipts for closure execution attempts that include immutable closure identity and enough handles to reproduce or inspect the proof boundary without log scraping.

#### Scenario: Successful closure receipt [r[execution-closures.receipts.success]]

- GIVEN a closure executes successfully through a supported worker
- WHEN the execution receipt is serialized
- THEN it MUST include schema version, closure hash, dependency root or dependency hash list handle, runtime target, input schema hash, output schema hash, input handle, output handle, status, and redacted capability summary

#### Scenario: Failed closure receipt [r[execution-closures.receipts.failure]]

- GIVEN closure admission, dependency fetch, runtime start, or runtime completion fails
- WHEN the execution receipt is serialized
- THEN it MUST include the closure hash when known, failed boundary, bounded diagnostic category, and redacted follow-up handles
