## 1. Implementation

- [ ] I1 Update tasks template and instructions with bounded-task guidance and examples. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes]
- [ ] I2 Update tasks template and instructions with oversized-task split guidance and prerequisite-order guidance. [covers=openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged,openspec-governance.task-size-and-ordering.dependency-order-explicit]
- [ ] I3 Add tasks-gate parsing for requirement/scenario fan-out and bounded implementation task acceptance. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes]
- [ ] I4 Add configured fan-out threshold and warning/fail severity policy. [covers=openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged]
- [ ] I5 Add emitted diagnostics and split guidance for oversized implementation tasks detected by the fan-out policy. [covers=openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged]
- [ ] I6 Add tasks-gate exception handling for explicitly labeled broad integration verification tasks. [covers=openspec-governance.task-size-and-ordering.integration-verification-may-be-broad]
- [ ] I7 Add tasks-gate dependency-order and prerequisite-note checks. [covers=openspec-governance.task-size-and-ordering.dependency-order-explicit]

## 2. Verification

- [ ] V1 Add fixtures for bounded, oversized, integration, out-of-order, and ambiguous-dependency task lists. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes,openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged,openspec-governance.task-size-and-ordering.integration-verification-may-be-broad,openspec-governance.task-size-and-ordering.dependency-order-explicit]
- [ ] V2 Run tasks gate on fixtures and save transcripts. [covers=openspec-governance.task-size-and-ordering]
- [ ] V3 Review or test the updated task template/instructions and save evidence that bounded, oversized, and prerequisite guidance is present. [covers=openspec-governance.task-size-and-ordering.bounded-implementation-task-passes,openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged,openspec-governance.task-size-and-ordering.dependency-order-explicit]
