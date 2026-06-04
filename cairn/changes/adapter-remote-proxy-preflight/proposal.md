# Change: adapter-remote-proxy-preflight

## Why

The actor registry reserves adapter-backed and remote-proxy actor kinds, but they intentionally remain fail-closed. Molten needs a preflight model that can enable local adapters and remote proxies without turning plugins, processes, transports, or peers into ambient authority or bypassing deterministic replay.

## What

- Define adapter preflight receipts for local adapter manifests, executable/artifact refs, ABI/schema refs, sandbox profiles, permission manifests, and conformance suite refs.
- Define remote-proxy preflight receipts for peer identity, endpoint/protocol refs, advertised actor contract, capability attenuation, transport profile, and replay transcript policy.
- Require all adapter/proxy interactions to use canonical actor-input, hostcall request/decision, actor-output, effect, and receipt envelopes.
- Treat remote execution as deterministic only when a verified transcript/effect log is available; otherwise mark it non-replayable and exclude it from deterministic gates.
- Bind transport/process success separately from trust, authority, and gate acceptance.
- Add negative suites for missing/tampered manifests, undeclared permissions, unknown peers, transcript mismatch, and ambient side-channel attempts.

## Impact

Adapters and remote proxies can be introduced incrementally while retaining Molten's fail-closed evidence model. This also prepares the bridge between local harness actors, plugin-host ABI work, and future Iroh-backed distributed actors.
