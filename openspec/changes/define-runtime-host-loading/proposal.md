## Why

Aspen's runtime direction now distinguishes services, jobs, applications, plugins, adapters, and the generic Executioner, but the loading model is still implicit. Without an explicit host-loading contract, future implementation could collapse first-party services into unsafe native plugins, require everything to become WASM, or treat Hyperlight jobs as a separate scheduler instead of one runtime host.

Aspen needs a spec that says how native built-ins, optional external native processes, WASM modules, and Hyperlight units enter the runtime while sharing the same lifecycle, route, capability, and receipt model.

## What Changes

- **Runtime host taxonomy**: define `NativeBuiltIn`, `NativeProcess`, `Wasm`, and `Hyperlight` host kinds.
- **Native loading rule**: first-party services such as Forge, Executioner, snix/cache, and federation are linked into `aspen-node` as built-in service factories, not loaded as in-process native dynamic plugins.
- **Dynamic loading rule**: WASM and Hyperlight units are content-addressed artifacts verified by hash/signature before instantiation.
- **Capability boundary**: every host kind receives scoped runtime handles; no manifests or receipts may contain raw secrets, tickets, private keys, cluster cookies, or connection strings.
- **Reference integration**: use `../verified-logic/` when defining finite admission/capability predicates that can be machine checked, and use `../ucan/` as the likely capability-token/delegation reference when binding runtime units to scoped authority.

## Capabilities

### New Capabilities

- `runtime-host-loading`: Defines how native, WASM, and Hyperlight runtime units are resolved, started, supervised, and observed.
- `runtime-capability-binding`: Defines the shared host-independent capability binding contract used at load/start time.
- `runtime-verified-admission`: Establishes where verified-logic-backed predicates should be used for manifest and capability admission.

## Impact

- **Docs/specs**: Adds an active OpenSpec package for the runtime host-loading contract.
- **Future APIs**: Guides future `aspen-runtime-core` types such as `RuntimeHostKind`, `RuntimeArtifact`, `ServiceSpec`, `ExecutionRun`, `RuntimeCapabilityBinding`, and `RuntimeReceipt`.
- **Security**: Rejects in-process native dynamic plugins as the default extension model; prefers linked native built-ins, separate native processes, WASM, or Hyperlight depending on trust/isolation needs.
- **Verification**: Later implementation should include pure runtime-core tests, source-anchor docs tests, capability-admission tests, and verified-logic/UCAN bridge evidence where feasible.
