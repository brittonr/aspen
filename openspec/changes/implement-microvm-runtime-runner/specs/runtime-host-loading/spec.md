## ADDED Requirements

### Requirement: MicroVM Runtime Runner [r[runtime-host-loading.microvm-runner]]
Aspen MUST provide a node-local microVM runner contract for isolated runtime units that require a VM or microVM host boundary.

#### Scenario: Runner capability is advertised [r[runtime-host-loading.microvm-runner.capability]]
- GIVEN an Aspen node can launch a supported microVM engine such as Firecracker, Cloud Hypervisor, Uhyve, QEMU microvm, or equivalent
- WHEN the node reports runtime runner capabilities
- THEN it SHALL advertise the supported engine, virtualization backend, resource limits, supported guest artifact profiles, and runner version

#### Scenario: Assignment fails closed without compatible runner [r[runtime-host-loading.microvm-runner.fail-closed]]
- GIVEN a runtime unit requires a microVM host boundary
- WHEN the scheduler or node admission evaluates an assignment
- THEN admission SHALL fail closed unless the selected node advertises a compatible microVM runner and sufficient declared resources

#### Scenario: Guest artifacts are prepared before launch [r[runtime-host-loading.microvm-runner.artifact-prep]]
- GIVEN a microVM runtime unit declares kernel, initrd, rootfs, disk, or guest-image artifacts
- WHEN the runner prepares the unit
- THEN it SHALL verify content identity before launch and SHALL record the verified artifact identities in the launch receipt

#### Scenario: Launch bindings deny ambient authority [r[runtime-host-loading.microvm-runner.launch-bindings]]
- GIVEN a microVM unit requests mounts, block devices, network interfaces, vsock channels, environment-like metadata, or capability handles
- WHEN the runner prepares the launch
- THEN it SHALL attach only declared and authorized bindings and SHALL deny undeclared devices, host paths, sockets, networks, secrets, and ambient host access before boot

#### Scenario: Runner records lifecycle receipts [r[runtime-host-loading.microvm-runner.receipts]]
- GIVEN a microVM unit starts, stops, fails, times out, or is killed
- WHEN the runner observes the lifecycle transition
- THEN it SHALL emit secret-safe receipts containing unit identity, assigned node, engine, attempt, lifecycle state, resource summary, artifact identities, and redacted handle summary

#### Scenario: Logs and outputs become artifacts [r[runtime-host-loading.microvm-runner.outputs]]
- GIVEN a microVM unit produces serial logs, stdout/stderr streams, disk outputs, or declared result artifacts
- WHEN the unit exits or checkpoints output
- THEN the runner SHALL persist bounded logs and outputs as Aspen-approved artifacts or explicit receipt fields without leaking raw secrets
