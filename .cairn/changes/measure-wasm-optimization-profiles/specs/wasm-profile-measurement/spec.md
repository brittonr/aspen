# Wasm Profile Measurement Specification

## Purpose

Make the reviewed Wasm optimization profiles measurable from reviewed source, so
a profile choice can rest on recorded numbers instead of intuition.

## Requirements

### Requirement: Reviewed source measures every profile [r[wasm_profile_measurement.tool]]

The repository MUST provide a diagnostic program that measures each reviewed
`molten.wasm.optimization.*.v1` configuration on a named, in-source workload and
reports per-phase samples. The program MUST state its build profile and MUST NOT
grant capability, register a runtime backend, or gate any release.

#### Scenario: Operator measures one workload

- GIVEN a built diagnostic program and a named workload
- WHEN the operator runs it with a sample count
- THEN it reports compile and instantiate+execute `median`, `p90` and `max` for
  each of the four reviewed profiles

#### Scenario: Program is absent from runtime paths

- GIVEN the default build
- WHEN the workspace is built without examples
- THEN no runtime path depends on the diagnostic program

### Requirement: Samples are reported, never a verdict [r[wasm_profile_measurement.reporting]]

The program MUST report the workload name, sample count and instances per sample
with the samples, and MUST NOT emit a pass/fail result, a recommendation, a
receipt, or a release claim.

#### Scenario: Report carries no decision

- GIVEN a completed run
- WHEN its output is read
- THEN it contains sample statistics and workload metadata only

### Requirement: Aggregation stays pure and tested [r[wasm_profile_measurement.aggregation]]

Sample aggregation and percentile rounding MUST be pure functions of the recorded
durations and MUST be covered by tests.

#### Scenario: Percentiles are reproducible

- GIVEN a fixed list of durations
- WHEN aggregation runs
- THEN the median, p90 and max depend only on that list

### Requirement: Runs are recorded with their scope [r[wasm_profile_measurement.evidence]]

A recorded measurement MUST bind the source commit, binary image, workload name,
sample counts and raw numbers, and MUST state which decision it does and does not
support.

#### Scenario: Evidence reviewed later

- GIVEN a recorded run
- WHEN a reviewer checks a claim about profile performance
- THEN the record names the exact commit, image and workload behind the numbers
