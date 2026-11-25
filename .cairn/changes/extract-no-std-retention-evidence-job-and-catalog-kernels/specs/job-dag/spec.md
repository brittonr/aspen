# Job dag deltas

## ADDED Requirements

### Requirement: JobDag graph validation, topological scheduling, and output-ref routing live in a no_std kernel
r[molten.job_dag.no_std_job_dag_kernel] Pure JobDag acyclicity validation, checked signature and output-reference routing, ambient-token scanning, and deterministic topological planning MUST live in a `#![no_std]` + `alloc` crate (`molten-job-dag-core`) separated from Nickel evaluation, `std::fs`, and `cap_std` shells in `src/job/`.

#### Scenario: JobDag planning and routing execute without Nickel or std
- GIVEN a canonical JobDag definition with node signatures and output-reference edges
- WHEN `molten-job-dag-core` validates and plans execution under `#![no_std]`
- THEN it MUST detect cycles, mismatched output references, and ambient tokens deterministically without linking `nickel-lang` or `std::fs`.
