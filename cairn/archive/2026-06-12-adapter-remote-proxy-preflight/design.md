# Design: adapter and remote-proxy preflight

## Adapter preflight

An adapter-backed actor is a local executor boundary around code or a process that is not native harness code, reviewed Steel, or reviewed Wasm. Before execution, an adapter preflight receipt binds:

- adapter manifest ref;
- executable/artifact/container/plugin ref;
- ABI and schema refs;
- sandbox profile ref;
- permission manifest ref;
- allowed hostcalls;
- resource profile;
- conformance suite refs;
- optional signer/provenance refs.

The adapter receives and emits only canonical Preserves envelopes. Any filesystem, network, process, clock, random, or environment access must be declared as explicit hostcall/effect authority and admitted by the runtime shell.

## Remote-proxy preflight

A remote-proxy actor represents a remote executor or actor endpoint. Preflight binds:

- local node identity ref;
- remote peer identity ref or explicit unknown-peer diagnostic mode;
- endpoint/protocol/schema refs;
- advertised actor contract and allowed hostcalls;
- capability attenuation/proof refs;
- transport profile;
- transcript/effect-log replay policy;
- trust/signature requirements.

Transport connection and peer identity are not authority. A remote endpoint can only request operations through canonical envelopes and local admission decisions.

## Determinism and replay

Local adapters can participate in deterministic gates only if their execution receipts and outputs are replayable from canonical inputs plus recorded effect responses. Remote proxies can participate in deterministic gates only if a verified transcript binds every inbound/outbound envelope, transport receipt, hostcall decision, and effect response.

If replay evidence is unavailable, the run may produce diagnostic artifacts but must be marked non-replayable and rejected by deterministic pass gates.

## Receipts

`<adapter-preflight-v1 ...>` and `<remote-proxy-preflight-v1 ...>` receipts bind manifests, schemas, sandbox/transport profiles, authority, conformance refs, and checks. Execution receipts bind actual input/output/transcript refs and resource usage. Gate receipts require matching preflight and replay evidence before accepting adapter/proxy actors as pass evidence.
