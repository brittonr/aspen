## ADDED Requirements

### Requirement: Remote Fork Contract [r[remote-execution.fork]]

Aspen MUST provide a portable remote fork contract for submitting an admitted execution closure to a selected execution handler.

#### Scenario: Fork returns remote handle [r[remote-execution.fork.returns-handle]]

- GIVEN a caller submits a valid execution closure and input handle to a remote execution handler
- WHEN fork admission succeeds
- THEN Aspen MUST return a remote handle containing submission identity, closure hash, input handle, handler/backend identity, and initial status

#### Scenario: Fork rejects denied capability [r[remote-execution.fork.denied-capability]]

- GIVEN a closure requires capabilities not granted to the caller or target handler
- WHEN fork admission runs
- THEN Aspen MUST reject the fork before starting runtime execution
- AND the failure receipt MUST include a redacted capability denial summary

### Requirement: Remote Await Contract [r[remote-execution.await]]

Aspen MUST provide an await contract that resolves remote handles to bounded result state and output handles.

#### Scenario: Await successful result [r[remote-execution.await.success]]

- GIVEN a remote handle refers to a completed successful execution
- WHEN await is called
- THEN Aspen MUST return success status, output handle, closure hash, and receipt identity without requiring log scraping

#### Scenario: Await failed result [r[remote-execution.await.failure]]

- GIVEN a remote handle refers to a failed execution
- WHEN await is called
- THEN Aspen MUST return typed failure status, bounded diagnostic category, and receipt identity

#### Scenario: Await missing handle [r[remote-execution.await.missing-handle]]

- GIVEN no known execution exists for a remote handle
- WHEN await is called
- THEN Aspen MUST return a typed not-found result rather than blocking indefinitely

### Requirement: Remote Timeout and Cancellation [r[remote-execution.timeout-cancel]]

Aspen MUST model timeout and cancellation behavior for remote executions.

#### Scenario: Await times out [r[remote-execution.timeout-cancel.await-timeout]]

- GIVEN a remote execution has not completed before the caller's bounded await deadline
- WHEN await reaches the deadline
- THEN Aspen MUST return a timeout result without losing the durable remote handle

#### Scenario: Cancel remote execution [r[remote-execution.timeout-cancel.cancel]]

- GIVEN a remote execution is pending or running and the caller has cancellation authority
- WHEN cancel is requested
- THEN Aspen MUST request cancellation through the handler and record a cancellation receipt with final state when known

### Requirement: Remote Handlers [r[remote-execution.handlers]]

Aspen MUST support multiple remote execution handlers that share the same fork/await contract.

#### Scenario: Local deterministic handler [r[remote-execution.handlers.local]]

- GIVEN a valid closure supported by the local deterministic handler
- WHEN fork and await run in a test or development context
- THEN the handler MUST execute without requiring a live cluster and MUST emit the same logical receipt fields as the product handler where applicable

#### Scenario: Product JobManager handler [r[remote-execution.handlers.jobmanager]]

- GIVEN a valid closure supported by Aspen's JobManager/WorkerPool path
- WHEN fork and await run against a live or integration-test Aspen runtime
- THEN execution MUST route through the product job orchestration path and receipts MUST correlate remote handle, job ID, worker/runtime target, closure hash, and output handle

### Requirement: Remote Execution Receipt Correlation [r[remote-execution.receipt-correlation]]

Aspen MUST correlate remote execution handles with job/runtime receipts.

#### Scenario: Operator can diagnose remote handle [r[remote-execution.receipt-correlation.diagnose]]

- GIVEN an operator has a remote execution handle
- WHEN they inspect or diagnose it
- THEN Aspen MUST expose bounded receipt handles for submission, admission, execution, output, cancellation, or failure without exposing raw secrets
