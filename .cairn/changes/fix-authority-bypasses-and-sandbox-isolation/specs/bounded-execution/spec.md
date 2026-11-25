# Bounded execution deltas

## ADDED Requirements

### Requirement: Steel execution is strictly sandboxed with real runtime bounds and synchronous hostcall enforcement
r[molten.bounded_execution.steel_sandbox_and_real_bounds] The Steel executor MUST instantiate a sandboxed VM without ambient filesystem, process, or environment access, MUST enforce actual runtime step/fuel and memory limits rather than string-length estimates or substring blocklists, MUST enforce `STEEL_MAX_HOSTCALLS` synchronously before executing each hostcall, and MUST NOT interpolate unescaped callable names into script text.

#### Scenario: Ambient filesystem access or lambda infinite loop denies
- GIVEN a Steel script attempting ambient file I/O or anonymous `(lambda (x) (x x))` recursion
- WHEN the Steel executor runs it
- THEN ambient I/O MUST be unavailable and recursion MUST terminate deterministically upon real fuel exhaustion.

#### Scenario: Excess hostcall denies synchronously
- GIVEN a Steel script invoking `molten-hostcall` more than `STEEL_MAX_HOSTCALLS` times
- WHEN the excess call occurs
- THEN the hostcall handler MUST reject the call immediately before performing any additional effect.

### Requirement: Live process execution verifies input/executable digests and refuses duplicate retries
r[molten.bounded_execution.live_adapter_binding_and_no_replay] `fabric_execution::live::Adapter` MUST verify `stdin_bytes` against `stdin_ref`, measure the opened executable against `executable_identity_ref`, and reject retries for `(operation_ref, generation)` pairs already recorded as `Unknown` or `Terminal`.

#### Scenario: Mismatched executable bytes or duplicate retry denies
- GIVEN a live execution request whose executable bytes do not match `executable_identity_ref` or whose `(operation_ref, generation)` is already `Unknown`
- WHEN `Adapter::execute` runs
- THEN it MUST fail closed without spawning the process.
