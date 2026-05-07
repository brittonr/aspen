# Hyperlight runtime architecture docs

- Change: `implement-hyperlight-runtime-runner`
- Task: update runtime architecture docs/source anchors
- Started: `2026-05-07T02:44:28Z`
- Completed: `2026-05-07T02:49:26Z`

## Documentation updates

- Updated `docs/runtime-applications.md` to define the portable Hyperlight runner/profile boundary.
- Added operator-facing anchors for:
  - `HyperlightRuntimeProfile`;
  - `HyperlightImage`;
  - ABI/artifact profile matching;
  - node-local runner capability/version matching;
  - declared host-call bindings;
  - `RuntimeHostKind::Hyperlight` admission;
  - bounded runner resource policy;
  - denial of ambient files, sockets, devices, routes, environment, network, secrets, and undeclared host calls.

## Verification

- Source-anchor assertion over `docs/runtime-applications.md` printed `hyperlight runtime docs anchors present`.
- `openspec validate implement-hyperlight-runtime-runner --strict`
- `git diff --check`
