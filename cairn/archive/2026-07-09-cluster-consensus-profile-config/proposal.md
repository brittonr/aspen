## Why

Clusters should be able to declare the control-plane consensus semantics they need in configuration, instead of relying on the default Raft manifest helper. The existing engine registry already admits or denies profiles, but operators need a typed cluster-facing selection value that feeds the manifest and fails closed before runtime construction.

## What Changes

- Add a cluster consensus profile config value that names the algorithm profile, optional profile version, placement evidence, and required evidence refs.
- Keep Raft as the default production-admitted profile when config omits an explicit selection.
- Build control-plane manifests from the config-selected profile through the existing profile validation and engine registry admission path.
- Add positive and negative tests for Raft selection, experimental profile manifesting, unknown profile denial, and runtime config validation.

## Impact

- **Files**: consensus manifest/profile builders, runtime startup config validation, consensus tests, and lifecycle specs.
- **Testing**: focused consensus/config tests plus formatting and Cairn validation.
- **Safety**: cluster config can select an engine profile, but production runtime still denies unknown, disabled, experimental, or evidence-incomplete engines through the existing registry/admission gates.
