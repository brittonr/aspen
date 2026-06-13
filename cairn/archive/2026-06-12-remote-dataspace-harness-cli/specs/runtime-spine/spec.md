## ADDED Requirements

### Requirement: Remote dataspace CLI namespace
r[molten.remote_dataspace_harness_cli.remote_subcommand] The system MUST expose remote dataspace harness operations under `molten test remote`.

#### Scenario: CLI parses remote command
r[molten.remote_dataspace_harness_cli.remote_subcommand.parse]
- GIVEN a remote dataspace subcommand
- WHEN the CLI parses arguments
- THEN the selected handler receives typed command fields without using positional string dispatch

### Requirement: Envelope build command
r[molten.remote_dataspace_harness_cli.envelope_build] The system MUST provide a CLI command that builds canonical `remote-dataspace-envelope-v1` artifacts from explicit peer, actor, topic, operation, payload, content ref, capability ref, and evidence ref inputs.

#### Scenario: Build assertion envelope
r[molten.remote_dataspace_harness_cli.envelope_build.assert]
- GIVEN a payload file containing `<service-ready "db">`
- WHEN `molten test remote envelope build` is run for operation `assert`
- THEN the output file contains a canonical remote dataspace envelope whose ref is printed or available for later publish

### Requirement: Deterministic local publish/deliver commands
r[molten.remote_dataspace_harness_cli.publish_deliver_local] The system MUST expose CLI commands for deterministic local Iroh-shaped publish and deliver of remote dataspace envelopes.

#### Scenario: Publish then deliver locally
r[molten.remote_dataspace_harness_cli.publish_deliver_local.roundtrip]
- GIVEN a canonical remote dataspace envelope file
- WHEN it is published with `remote publish-local` and delivered with `remote deliver-local`
- THEN the delivered envelope ref matches the published envelope ref and transport receipts are emitted as canonical Preserves artifacts

### Requirement: Two-peer remote harness command
r[molten.remote_dataspace_harness_cli.run_two_peer] The system MUST provide a one-command deterministic two-peer remote dataspace scenario where peer A asserts `service.ready` and peer B observes it through the recorded delivery log.

#### Scenario: Two-peer run emits pass evidence
r[molten.remote_dataspace_harness_cli.run_two_peer.pass]
- GIVEN a transport root and output directory
- WHEN `remote run-two-peer` succeeds
- THEN it emits delivery log, admission receipt, gate receipt, and summary artifacts, and replay uses the recorded delivery log

### Requirement: Remote dataspace gate CLI
r[molten.remote_dataspace_harness_cli.gate_command] The system MUST provide a CLI command that creates a remote dataspace gate receipt only from replayable delivery logs, admission receipts, and turn-journal context refs.

#### Scenario: Non-replayable log is denied
r[molten.remote_dataspace_harness_cli.gate_command.non_replayable]
- GIVEN a non-replayable remote delivery log
- WHEN the gate command is run
- THEN it fails closed before emitting pass evidence

### Requirement: Remote service-ready example
r[molten.remote_dataspace_harness_cli.example_fixture] The system MUST include an example Preserves payload fixture for the remote service-ready scenario.

#### Scenario: Example parses
r[molten.remote_dataspace_harness_cli.example_fixture.parses]
- GIVEN `examples/remote-service-ready.preserves`
- WHEN it is parsed as Preserves
- THEN it yields the service-ready payload used by the CLI demonstration
