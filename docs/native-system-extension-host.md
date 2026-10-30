# Native system-extension host

r[impl molten.system_extension.native_host.profile] r[impl molten.system_extension.native_host.callback_protocol] r[impl molten.system_extension.native_host.executable] r[impl molten.system_extension.native_host.execution] r[impl molten.system_extension.native_host.durability] r[impl molten.system_extension.native_host.intent] r[impl molten.system_extension.native_host.effects] r[impl molten.system_extension.native_host.effect_completion] r[impl molten.system_extension.native_host.effect_completion_value] r[impl molten.system_extension.native_host.ingress] r[impl molten.system_extension.native_host.recovery] r[impl molten.system_extension.native_host.operator] r[impl molten.system_extension.native_host.neutrality] r[impl molten.system_extension.native_host.validation] r[impl molten.system_extension.native_host.nonclaims] r[impl molten.system_extension.native_host.value_protocol] r[impl molten.system_extension.native_host.value_materialization] r[impl molten.system_extension.native_host.value_publication] r[impl molten.system_extension.native_host.value_intent] r[impl molten.system_extension.native_host.semantic_state] r[impl molten.system_extension.native_host.value_validation]

Molten can host an admitted system extension as one bounded process per callback. The host remains workload-neutral.

## Profile and executable admission

The materializing profile is `native-host-local-pilot-v2`. It uses:

- execution profile `native-process`;
- ALPN `molten/system-extension/native/v3`;
- framing `preserves-packed-materialized-values-v2`;
- a cleared process environment;
- one process for each callback;
- named callback, value, diagnostic, instance, operation, port, and policy bounds;
- mandatory materialized values without a v1 fallback.

Installation binds exact executable bytes, artifact kind, target, dependency closure, materialization, provenance, source gate, policy, authority, resources, state schema, manifest, and execution profile.

A path or executable file does not grant authority. Missing evidence denies before instance publication or process start.

The node-host capability owner exposes retained `Executable`/`Workspace` handles,
not public raw directory cloning. `DirectoryRoot::open_executable` rejects a
symlink/nonregular, nonexecutable or oversized artifact; bounded measurement and
the Linux process descriptor path refer to that same retained inode.
`Scope::process_workspace` retains the selected owned workspace through callback
completion. Replacing an artifact's pathname cannot substitute another file for
that retained execution capability. Optional supplied endpoint/profile roots can
use noncreating `existing_namespace`/`existing_subdir` acquisition: absence does
not prepare a directory or manufacture a capability. These mechanisms do not
prove executable trust, sandboxing or effect authority.

The public `node service-host` commands load real canonical admission bundles,
validate the actual startup source gate, verify separate current load/operation
authority, retain measured executable capabilities, and reconstruct the existing
durable journal rather than resetting native state. See the
[public admission and credential bootstrap contract](system-extension-runtime.md#public-node-service-host-admission).

## Callback protocol

The host writes one canonical packed Preserves envelope to standard input. The child writes one canonical packed Preserves outcome to standard output.

The envelope binds:

- manifest, executable, instance, extension, and service identities;
- generation, callback kind, sequence, event, and deadline;
- exact payload and prior-state reference-and-byte values;
- policy, resource, and port references;
- the exact v2 framing profile.

The outcome binds exact output, effect-request, next-state, and checkpoint values. Each value includes its BLAKE3 reference and bytes.

The decoder rejects missing bytes, identity drift, malformed values, non-canonical frames, trailing data, oversized data, ambient effects, and unsupported schemas.

`molten-native-extension-fixture` is an independent callback producer. The host decoder is the independent consumer.

## Bounded execution

`system_extension::Process` invokes callbacks only through `ExecutionFabricPort`. It does not call `std::process::Command`.

The executor persists callback intent before value reads or process execution. It materializes payload and state through `NativeCallbackValuePort`.

After execution, it admits all returned values and effect metadata. It persists each publication intent before the value port can publish bytes.

A definite publication rejection becomes terminal. Uncertain publication remains unknown and blocks automatic retry and dependent provider routing.

`InMemoryCallbackValuePort` is a conformance adapter only. A deployment adapter must provide bounded reads, durable exact publication, and explicit acceptance uncertainty.

The process adapter clears the environment and applies exact input, output, timeout, polling, teardown, and process-group bounds.

## Durable state and recovery

The native instance record contains:

- manifest, executable, profile, and state-schema references;
- lifecycle state and active generation;
- resource use, callback sequence, and event sequence;
- latest semantic state and lifecycle checkpoint references;
- unresolved and completed callback, publication, ingress, and effect operations;
- evidence references and ingress state.

`system_extension::Durable` stores canonical instance records through the existing Redb durability adapter. Restart reloads the latest exact instance record.

Startup classifies unresolved work as not started, running observed, terminal, unknown, or stale. Unknown work remains reconciliation-required and does not retry automatically.

A recovered running instance first records host loss. Before the next external recovery callback, the native shell persists the owning lifecycle planner's fresh incarnation and closed-ingress fence. Callback intent therefore cannot be journaled against the predecessor's durable incarnation. Restart retains unresolved older operations rather than granting permission to replay them.

## Public node recovery hosting

The separately governed `host-node-recovery-groups` prerequisite composes these mechanisms in `node_service_hosting`; it does not add a supervisor to the test harness. `node_service_host` loads exact canonical admitted bundles and exposes `node service-host install/status/supervise/submit/readmit/deliver/upgrade/reconcile`. Startup source admission and independent current UCAN authority precede loader effects. Pure request previews grant no permission. The actual node serving shell retains admitted supervisors, journals and measured executable/workspace handles for its lifetime and emits a flushed canonical `node-service-host-ready-v1` observation only from that current process.

The host's ledger-capability Redb store atomically retains declarations, restart-window history, native generation/incarnation observations, logical work/current claims and lifecycle action intents. Installation performs an initial checkpoint before returning. Stop/restart order comes from the existing pure recovery planner; replacing a node does not reset its group budget. A pending group fences its members without fencing an independent sibling.

Ordinary readmission never changes a logical work item's generation. Delivery checks the exact stored current claim before the owning pure work-admission law and actual native ingress. Unknown/deferred outcomes and callbacks with unrouted approved effects retain their logical work instead of being reported complete or automatically repeated. Status omits payload bytes and exposes canonical member fences, native recovery inventories, admitted logical-time attempts, pending/terminal groups, work disposition/current claims and migration lineage.

Upgrade records drain, checkpoint and target-generation intents separately. Checkpointing a drained native member does not reopen ingress. Before the target Recover callback, the native shell publishes the exact admitted source/target migration receipt and target fence; a receipt alone is not activation or transformation proof. Successful materialized recovery and the owning activation transition are required before ingress reopens. Explicit migrated readmission additionally checks that actual durably referenced receipt, executing manifest/schema/generation and current authority for the exact target payload; it retains the source presentation/payload lineage. Old-generation work remains rejected.

After interruption, reconciliation reads the real native journal. An exact unchanged pre-action record proves the executor never committed a callback intent and permits a fresh attempt. A matching completed postcondition advances the action cursor. A committed but ambiguous callback remains fenced and reconciliation-required; it is not blindly repeated.

### External-cohort recovery proof

`tests/nativesystemextension/recovery.rs` is the actual public-node twin. It constructs no test-owned supervisor. It runs the independently source-admitted Nix node ELF, uses genuine issuer/holder-signed exact UCAN invocations, installs real callback artifacts, waits for that child's canonical hosted-ready observation, SIGKILLs that same serving node, and reopens its actual journals through public operations. Its observations use the same composition property evaluator as the deterministic track. Upgrade checks materialized transformation bytes, restarts the activated serving node again and checks retained generation/state/checkpoint and migrated durable work. Wall time bounds supervision only; property judgments use admitted logical ticks and canonical observations.

The explicit external tests are ignored in ordinary runs and fail, rather than simulate or silently skip, when selected without their independently provisioned artifacts. `MOLTEN_TEST_SOURCE_TRUST` selects the actual independent startup trust JSON. `MOLTEN_TEST_SERVICE_HOST_COHORT` selects a directory containing canonical `admission.preserves` plus every relative admitted executable artifact. The bundle must declare the shared two-member composition group, the independent sibling in its own singleton group, initial checkpoint/recovery callbacks, an exact successor and a native per-instance restart budget of at least eight (so it cannot mask a broken group budget).

The three files in `tests/fixtures/service-recovery-faults/` define concrete fault-build source mutations: omitted delivery fencing, lost history on actual reopen, and copied rather than transformed native Recover state. Apply them only to distinct fault-build source trees and independently build/admit/sign each real package/cohort. The broken live rail requires `MOLTEN_TEST_{FENCING,STORM,MIGRATION}_BROKEN_SOURCE_TRUST` and matching `_BROKEN_COHORT` directories. Never reuse the good source-gate receipt for mutated source, invent a gate run or treat these files as execution evidence.

Run the actual rails explicitly with `cargo test --locked -p molten --test nativesystemextension recovery::live_public_node_composition -- --ignored --exact` and `cargo test --locked -p molten --test nativesystemextension recovery::live_public_node_broken_variants_fail_shared_properties -- --ignored --exact`. Source/fault cohorts and recorded successful runs are external acceptance prerequisites; implementation and command documentation do not claim that they have run.

### Authoring the actual experimental cohorts

`molten-service-host-cohort` is the executable producer; a preexisting unexplained admission bundle is not required. Its `create` command takes an independently provisioned source trust JSON, the exact matching signed Nix output, explicit issuer and holder Ed25519 raw 32-byte seed files, an independently reviewed Basalt policy JSON and an existing owned output-parent capability. It exclusively creates one new bounded child name and never overwrites an existing cohort. Private seed material is not copied into the cohort or printed; public keys and signed capabilities are retained as authority evidence.

```sh
molten-service-host-cohort create \
  --source-trust "$TRUST" --signed-output "$SIGNED_OUTPUT" \
  --issuer-key "$ISSUER_SEED" --holder-key "$HOLDER_SEED" \
  --policy "$EXPERIMENTAL_POLICY" --output-parent "$COHORT_PARENT" --name correct
```

The selected output must actually contain `bin/molten-native-extension-fixture` from that candidate. The producer calls the distinct `verify_selected_node_output_with_root` verifier: real independent signatures/NARs, frozen source binding, closure, cohorts, retained policy/profile and gate checks, plus immutable selected-node ELF measurement. It emits selected-output-only authoring evidence, never a current-process startup token. The authored expected startup validation reference is checked again against the actual executing node ELF by normal public-node loading.

The producer streams and measures the real callback ELF into its owned artifact capability, retains actual source/provenance/materialization/schema/profile artifacts, signs real UCAN native execution and cohort/load credentials, and constructs all raw `ExtensionInput` fields through actual admission owners. The bundle has the shared two-member group, a separate singleton sibling, initial checkpoint/recovery callbacks, restart budget sixteen, actual live Redb namespaces and an exact successor retaining the original instance/storage/value roots. It emits `admission.preserves`, `authoring-receipt.preserves`, operator/source trust, the callback artifact, canonical evidence, `install-main.preserves`, `install-sibling.preserves`, `status.preserves`, `supervise.preserves`, `upgrade.preserves` and a non-activated `startup-selector.preserves` template. It does not construct a Supervisor, start a service, run a conformance test or grant production admission.

The explicit policy must authorize the `node-service-host` contract and `molten://node-service-host/` resource family for `execute`, `author-cohort`, `load` and the intended public operations (`install`, `status`, `supervise`, `submit`, `readmit`, `readmit-migrated`, `deliver`, `upgrade`, `reconcile`). The producer evaluates that actual policy before issuing each capability; it does not embed an unconditional allow policy. The profile's supplied conformance references are genuine selected-source validation artifacts, explicitly **source-admission evidence only**, not fabricated runtime conformance runs. Actual runtime evidence is produced by the subsequent live rails. Experimental trust expires after one hour and is scoped to the new owned cohort, not production roots.

For manual operation, use the actual node's `--preview-load` and `--preview` outputs. `authorize` accepts one exact text (default) or packed preview, the same independent source/output/key/policy inputs and a fresh supplied 32-byte invocation nonce file. It re-verifies the selected output and current cohort trust, signs the actual operation tuple and writes its canonical credential. It does not execute or report that operation successful:

```sh
molten-service-host-cohort authorize \
  --cohort "$COHORT_PARENT/correct" --source-trust "$TRUST" --signed-output "$SIGNED_OUTPUT" \
  --issuer-key "$ISSUER_SEED" --holder-key "$HOLDER_SEED" --policy "$EXPERIMENTAL_POLICY" \
  --request "$ACTUAL_PREVIEW" --request-format text --nonce-file "$FRESH_NONCE"
```

Every public invocation needs both its exact operation credential and a fresh load invocation; `--preview` consumes its load nonce, so refresh load authorization before actual execution. A new nonce does not make unknown work retryable. For the integrated positive test, set `MOLTEN_TEST_SOURCE_TRUST="$TRUST"` and `MOLTEN_TEST_SERVICE_HOST_COHORT="$COHORT_PARENT/correct"`. For each concrete fault source patch, the parent producer builds and signs that distinct source/output, provisions its independent trust selection and runs the **same** `create` command with that fault output and a different owned name. Export each matching broken trust/cohort pair to the negative rail. No gate receipt or source identity is copied from the good build to a mutant.


## Effects

The callback outcome is validated before any effect is visible. Every effect request body publishes before routing. Each effect uses one active manifest binding.

The service persists effect intent before adapter routing. A lost result remains unknown.

A terminal effect observation returns through a generation-fenced `message` callback. Version two binds the exact optional provider output value.

A materializing native profile requires bounded output bytes whose BLAKE3 identity matches the effect output reference. Missing or changed bytes block callback delivery.

The provider effect remains terminal when value admission fails. The host does not retry it or infer workload success. The extension decides its semantic transition.

Public installed collaboration effects additionally require the separately
governed [admitted native effect runtime composition](native-effect-runtime.md).
Its explicit `service-host-effects` configuration and independent current
`install-effects`/`route-effect` grants bind the actual active service, port,
scope group, generation, policy and resource. Without that concrete admitted
router, approved effects remain unrouted/unknown. A proposal queue acknowledgment
never substitutes for actual scope commit and native completion-callback proof.

## Ingress

`NativeServiceIngressPort` accepts only the exact endpoint, peer, service, manifest, generation, authority, policy, resource, transport, ALPN, framing, and payload cohort.

The host commits ingress intent before publishing the exact payload bytes. It dispatches only after identity-checked publication succeeds.

Transport delivery does not prove payload publication or callback acceptance.

`system_extension::Client` is the local pilot client. A future Iroh adapter can implement the same application-owned port.

## Operator workflow

The service shell exposes install, start, request, status, checkpoint, restart, recover, drain, stop, and remove operations.

Drain stops new ingress before the callback. Removal fails when ingress, resource use, unresolved work, or terminal lifecycle evidence is incomplete.

Status uses claim level `local-live-materialized-values-pilot`. It reports canonical operator state and unresolved recovery classes. It omits host paths, process identifiers, backend handles, environment values, and secrets.

## Offline evidence

The artifact index links executable, callback envelope, callback receipt, execution receipt, instance state, semantic state, value publication, effect, checkpoint, and lifecycle evidence.

Offline verification rejects duplicate members, malformed references, missing roles, broken parent links, identity drift, and missing non-claims.

The separate-process integration test runs install, activation, accepted and rejected ingress, effect routing, completion callback, checkpoint, restart recovery, drain, stop, removal, and tamper denial.

## Non-claims

Local pilot evidence does not prove:

- sandboxing or hermeticity;
- executable trust, value meaning, value durability, or callback correctness;
- effect success or transport delivery;
- distributed availability;
- production readiness.
