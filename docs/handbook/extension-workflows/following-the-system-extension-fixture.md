# Following the system-extension fixture

Mode: Walkthrough

This walkthrough follows the checked-in `run_executable_system_extension_fixture` from manifest inputs to operator artifacts. Use it to establish what the deterministic fixture actually exercises before using its evidence in an integration review. It is not a native-process deployment tutorial. For lifecycle theory, see [Generation-fenced lifecycle](../../technical/extensions/generation-fenced-lifecycle.md); return to the [Handbook](../README.md) for other practical workflows.

**Execution status:** the path and commands below are source-checked, not executed for this documentation batch. No successful build, fixture run, or runtime readiness is claimed.

## 1. Choose the actual execution path

The CLI is reached through the `system_extension_port` alias in `src/main.rs`, the root `SystemExtension` variant, and `src/cli/runtime/system_extension.rs`. The fixture implementation is split into two included source files; the small `fixture.rs` wrapper is not the implementation itself.

Choose `sandboxed-component` or `in-process-native`. Although the CLI value enum also contains `native-process`, the deterministic fixture explicitly rejects it. The separate-process integration test is a different path, not another mode of this tutorial.

The sandboxed fixture constructs a Wasmtime **core module** exporting an integer identity function. It gives each probe invocation fuel and no imports. Rust `EchoExecutor` still constructs the callback outcomes. Consequently, this fixture does not demonstrate the production `molten.wasm.component.v1` WIT/materialization pipeline described in the [component runtime contract](../../wasm-component-runtime.md).

## 2. Follow admission inputs

`fixture_manifests` obtains system-tier admission, builds the fixture transport descriptor, and admits initial, upgrade, and rollback manifests. The fixture declares initialize, start, request, health, checkpoint, recover, drain, and shutdown callbacks. Its required transport operation is `send-envelope` on `molten.fabric.transport.session` version `v1`.

The envelope allows two concurrent callbacks, two queued events, 4,096 in-flight bytes, four effect requests, and one restart attempt. Its overload policy is upstream backpressure. These are fixture settings, not deployment recommendations.

The repeated-letter BLAKE3-looking references in this source are synthetic fixture inputs. They must not be copied into an installation as capability, provenance, implementation, or policy evidence. Canonical Preserves and BLAKE3 define artifact identity; Rust struct layout and descriptive names do not.

## 3. Trace invocation and effect release

The host activates, invokes health, and dispatches its first request. That request must return `HostDispatchResult::Executed` before the fixture proceeds. The returned receipt carries approved typed effects, which are then passed separately to `route_approved_effects`.

The fixture transport adapter checks the binding and operation, increments its route counter, and returns a reference-only output. This demonstrates the generic host's routing boundary. It does not demonstrate network delivery or the native v2 materialized-output requirement. A successful callback and an effect-completion receipt answer different questions.

## 4. Trace continuity and the deliberate failure

Next the host checkpoints and uses the resulting checkpoint reference for upgrade and rollback. Each transition uses a newly constructed executor and produces a status snapshot. After rollback, one request succeeds; the next request reaches the executor's deliberate second-request failure.

`fail_and_recover` requires a failed dispatch and a `Failed` phase, invokes bounded restart, requires a return to `Running`, and dispatches a post-recovery request. Finally, the fixture drains and shuts down. Its conformance input requires the declared lifecycle and continuity callbacks to have been exercised. These source assertions describe intended observable boundaries, not results observed in this writing session.

## 5. Produce isolated artifacts when your environment is ready

Use a fresh output directory: `write_artifacts` creates directories and writes files, but does not promise exclusive creation or protect earlier evidence from overwriting. Set `EXTENSION_FIXTURE_OUT` to a new location before running this guarded recipe.

Command provenance: [Cargo default binary](../../../Cargo.toml), [main alias](../../../src/main.rs), [root command](../../../src/main/root/parts/command/p000/body.rs), [profile and option declarations](../../../src/cli/runtime/system_extension/command.rs), and [artifact/readback operations](../../../src/cli/runtime/system_extension/ops.rs). Source-checked; not executed here.

```sh
: "${EXTENSION_FIXTURE_OUT:?Set a fresh output directory}"
if test -e "$EXTENSION_FIXTURE_OUT"; then
  printf '%s\n' 'Choose an unused fixture output directory.' >&2
else
  cargo run -- system-extension run-fixture \
    --profile sandboxed-component --out "$EXTENSION_FIXTURE_OUT" &&
  cargo run -- system-extension show \
    --status "$EXTENSION_FIXTURE_OUT/status.preserves"
fi
```

Do not infer completion from a partially populated directory: artifact writes are sequential, not one atomic export.

## 6. Interpret the output boundary

`manifest.preserves` records the initial admitted manifest. The upgraded, rolled-back, recovered, and final status files capture different lifecycle points. `evidence/` contains indexed lifecycle, callback, effect-completion, migration, and readiness artifacts as present in the run; indices are positions, not durable operation identities.

`show` accepts a bounded fixed status schema and prints selected safe fields. It is not a generic receipt verifier. Giving it a callback artifact is a type error, not evidence that the callback failed. Preserve the artifact set and actual command result together. Neither the final stopped status nor a readiness artifact proves production readiness, durable external effects, or extension semantics.

## Sources

- [Handbook](../README.md)
- [System-extension runtime](../../system-extension-runtime.md)
- [Generation-fenced lifecycle](../../technical/extensions/generation-fenced-lifecycle.md)
- [Fixture execution and executor](../../../src/system_extension/parts/fixture/p000/body.rs)
- [Fixture manifests and recovery checks](../../../src/system_extension/parts/fixture/p001/body.rs)
- [CLI artifact writing and status parsing](../../../src/cli/runtime/system_extension/ops.rs)
- [Wasm component runtime](../../wasm-component-runtime.md)
