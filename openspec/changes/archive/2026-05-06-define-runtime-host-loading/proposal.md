## Why

Aspen's runtime direction now distinguishes services, jobs, applications, plugins, adapters, and the generic Executioner, but the loading model is still implicit. Without an explicit host-loading contract, future implementation could collapse first-party services into unsafe native plugins, require everything to become WASM, or treat Hyperlight jobs as a separate scheduler instead of one runtime host.

Aspen needs a spec that says how native built-ins, optional external native processes, WASM modules, and Hyperlight units enter the runtime while sharing the same lifecycle, route, capability, and receipt model.

## What Changes

- **Runtime host taxonomy**: define `NativeBuiltIn`, `NativeProcess`, `Wasm`, `Hyperlight`, `OciContainer`, and later `MicroVm` host kinds.
- **Artifact profiles**: distinguish host boundary from artifact shape, including built-ins, native binaries, WASM modules, Hyperlight images/programs, OCI images, Linux guest images, and HermitOS-style unikernels launched through Uhyve or Hermit loader/VM paths.
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
- **Verification**: Implementation includes pure runtime-core tests, source-anchor docs tests, capability-admission tests, and verified-logic/UCAN bridge evidence.

## Verification Expectations

- Cover `r[runtime-host-loading.host-taxonomy]` and host-kind scenarios with portable `aspen-runtime-core` model types and `docs/runtime-applications.md` source anchors.
- Cover `r[runtime-host-loading.native-built-in.forge-start]` with a built-in factory shape that models Forge as `BuiltIn("forge")` without dynamic native plugin loading.
- Cover `r[runtime-host-loading.lifecycle.common-fields]` with runtime unit declarations that include artifact identity, host kind, capabilities, resources, route ownership, lifecycle status, and receipts.
- Cover `r[runtime-host-loading.capability-bindings.secret-redaction]` with redaction helpers and negative tests that reject raw secrets in receipts.
- Cover `r[runtime-host-loading.ucan-delegation.reference-reviewed]` and `r[runtime-host-loading.verified-admission.reference-reviewed]` with review notes for `/home/brittonr/git/ucan` and `/home/brittonr/git/verified-logic`.
- Validate with `cargo test -p aspen-runtime-core`, `cargo test --test runtime_host_loading_docs_test`, `openspec validate define-runtime-host-loading --strict`, `scripts/openspec-preflight.sh define-runtime-host-loading`, and `git diff --check`.
