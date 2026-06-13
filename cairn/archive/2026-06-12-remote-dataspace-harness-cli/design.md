## Context

Remote dataspace records now exist in `src/remote_dataspace.rs`, but users need a stable artifact-oriented CLI path. The CLI should follow the existing `test chunk`, `test chain`, and `test job` pattern: read/write Preserves files, print refs, and fail closed with canonical evidence where applicable.

## Goals

- Provide a minimal CLI for the deterministic remote dataspace path.
- Use only canonical Preserves artifacts as file boundaries.
- Make the two-peer `service.ready` scenario one command.
- Add a gate command that validates replayability and required refs for remote dataspace gate evidence.

## Non-Goals

- Do not run a long-lived live Iroh daemon in this change.
- Do not make live network timing deterministic.
- Do not grant authority from CLI flags alone; explicit evidence refs remain required for admitted delivery.

## CLI Shape

```sh
molten test remote envelope build \
  --from-peer peer:a --from-actor producer --to-peer peer:b \
  --topic services --operation assert \
  --payload payload.preserves --out envelope.preserves

molten test remote publish-local \
  --transport-root target/remote-transport \
  --envelope envelope.preserves --node peer:a \
  --receipt-out publish.preserves

molten test remote deliver-local \
  --transport-root target/remote-transport \
  --topic services --envelope-ref blake3:... --receiver-peer peer:b \
  --out delivered-envelope.preserves --receipt-out deliver.preserves

molten test remote run-two-peer \
  --transport-root target/remote-transport \
  --out target/remote-demo

molten test remote gate \
  --delivery-log target/remote-demo/delivery-log.preserves \
  --admission-receipt target/remote-demo/admission-receipt.preserves \
  --turn-context-ref blake3:... \
  --receipt-out target/remote-demo/gate-receipt.preserves
```

The first implementation may use a fixture helper for evidence refs in `run-two-peer`; later production commands should accept explicit peer bootstrap, capability, policy, resource, and authority artifacts or refs.

## Example Fixture

`examples/remote-service-ready.preserves` should contain the payload value used by the default two-peer scenario:

```preserves
<service-ready "db">
```

## Validation

The CLI lifecycle test should build or run the scenario, verify that all outputs parse as Preserves, confirm the gate receipt is classified as `remote-dataspace-gate-receipt`, and ensure replay is from recorded delivery logs rather than live transport bytes.
