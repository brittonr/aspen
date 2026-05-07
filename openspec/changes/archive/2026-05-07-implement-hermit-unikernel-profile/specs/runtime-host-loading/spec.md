## ADDED Requirements

### Requirement: Hermit Unikernel Runtime Profile [r[runtime-host-loading.hermit-profile]]
Aspen MUST model HermitOS-style unikernels as verified guest artifacts that run under a VM or microVM host boundary rather than as OCI containers or native host processes.

#### Scenario: Hermit artifact identity is explicit [r[runtime-host-loading.hermit-profile.artifact-identity]]
- GIVEN a runtime unit declares a HermitOS-style unikernel
- WHEN the runtime resolves the artifact
- THEN it SHALL identify the Hermit application image, target architecture, guest ABI/profile, and content hash separately from the host runner

#### Scenario: Uhyve launch profile is capability-gated [r[runtime-host-loading.hermit-profile.uhyve]]
- GIVEN a Hermit guest is assigned to a Uhyve launch profile
- WHEN node admission evaluates the assignment
- THEN admission SHALL require a compatible Uhyve runner capability and SHALL verify the guest image before launch

#### Scenario: Loader or QEMU path verifies loader artifacts [r[runtime-host-loading.hermit-profile.loader-qemu]]
- GIVEN a Hermit guest uses a loader, QEMU microvm, or equivalent boot path
- WHEN the runner prepares the launch
- THEN it SHALL verify loader, boot profile, and guest image identities separately and SHALL record those identities in the receipt

#### Scenario: Hermit boot inputs do not carry secrets [r[runtime-host-loading.hermit-profile.secret-boundary]]
- GIVEN a Hermit guest requires configuration, capability handles, or runtime inputs
- WHEN the launch profile creates boot arguments, environment-like metadata, serial output, or receipts
- THEN it SHALL NOT include raw tokens, tickets, private keys, cluster cookies, connection strings, or other secret material

#### Scenario: Hermit input channels are explicit [r[runtime-host-loading.hermit-profile.input-channels]]
- GIVEN a Hermit guest requires configuration or capability handles
- WHEN the profile maps inputs into boot arguments, loader metadata, virtio/vsock channels, or host ABI shims
- THEN every input channel SHALL be declared and authorized before launch
- AND undeclared host filesystem, network, secret, device, and ambient access SHALL be denied by default

#### Scenario: Hermit serial output is bounded and redacted [r[runtime-host-loading.hermit-profile.serial-logs]]
- GIVEN a Hermit guest writes serial or console output
- WHEN the runner captures logs
- THEN it SHALL bound log size, redact known secret-bearing fields, and persist logs as Aspen-approved artifacts or redacted receipt fields
