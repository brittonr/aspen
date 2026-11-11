# Design: Explicit installed native effect composition

## Ownership and boundaries

The governing operational contract is [admitted native effect runtime composition](../../../docs/native-effect-runtime.md), interpreted relative to the repository root (`docs/native-effect-runtime.md`). This change owns the `molten.system_extension.native_effect_*` requirements below; its tasks do not relabel a different active change's requirements as native-effect proof.

The system-extension pure core retains exact effect/port/generation laws. Existing `Supervisor::route_effects` persists native intent before delegate effects and produces canonical completions; `deliver_effect_completion` publishes exact completion bytes and invokes the real generation-fenced native message callback. `node_service_hosting::NativeRuntime` owns explicit bounded per-member `NativeEffectRouter` registration, invokes current router admission, routes actual effects and delivers each completion. Root logical work remains Unknown unless every approved effect has its actual successful completion callback and no further unhandled effects.

The public loader owns `node_service_host::effects::{config,authority}` and the concrete ScopeRouter. `collaboration_scope::native_runtime` owns pure runtime admission, actual scope-resource construction, the bounded retained worker, fresh scope authorization, actual Raft/application progress and committed result materialization. Source-gate verification never substitutes for either operator or scope mutation authority.

## Canonical configuration and admission order

The optional operator-owned `service-host-effects` artifact is a bounded canonical configuration (32 bindings, 1 MiB). Each binding contains exact service/manifest/port binding/key/operation, group, generation, policy, resource and installation refs plus an owned relative runtime-artifact locator and content ref. Unknown protocols, duplicate services, malformed/traversing locators and altered artifacts deny. An absent configuration installs no router and grants no permission.

The existing independently provisioned operator trust and actual UCAN/Basalt verifier enforce exact `install-effects` and `route-effect` requests. The installation request covers the current bundle, entire configuration, source-validation observation and logical tick. The routing request additionally covers the exact selected binding and actual typed effect, including schemas, request, generation/incarnation and byte accounting. Holder nonce reservations remain durable and no-replace. Tokens, signatures and keys are not projected into status.

After source and ordinary bundle-load admission, installation authority precedes scope resources. The loader validates the actual Running supervisor's manifest, admitted port binding, generation, policy and executable resource; verifies runtime artifact bytes; and runs the domain factory's pure `admit` against the actual Dispatcher. Its derived installation/group/policy/resource must match configuration before `Runtime::open` may create scope storage, transports or worker resources. A fresh unactivated native service cannot be implicitly activated by effect configuration.

## Actual scope operation and completion

The retained domain runtime uses the actual ScopeEffectPort and active ScopedLiveReplica with ScopeApplication, not a mock sender or model state. Before enqueueing a command it obtains the owning fresh quorum/current-scope observation and re-stages the exact mutation through independent current scope UCAN/Basalt/revocation services. A historical staged Decision alone is insufficient. The bounded worker must report real successful startup before installed-host readiness can be emitted.

A queue receipt means queued, not committed. Internal queued and externally committed schemas stay distinct. Only the actual committed replica/application transition, with exact command/request/group/generation/state observation, permits a materialized committed result. NativeRuntime delivers that completion through the existing native callback mechanism, preserving the original approved-effect list and exact completion inventory. Further effects emitted by a completion callback remain unhandled unless separately admitted; they cannot be hidden by a successful parent callback.

## Lifetime, process loss and uncertainty

Both one-shot public operations and InstalledHost invoke the explicit composition after reconstructing actual native supervisors. The actual node serve/run-loop owns InstalledHost, therefore the concrete router and scope runtime, through its full wait/loop. The scope worker uses bounded channels and waits, owns only its resources and closes only its own worker/resources. No ambient fallback, spawn-and-assume-ready, implicit proposal retry or service autoactivation is permitted.

Native effect intent, scope proposal acceptance, committed application state and completion callback are separate boundaries. Failure after an effect may have occurred remains durable Unknown/reconciliation-required. Reopening does not requeue a predecessor's uncertain effect; the concrete router rejects unresolved predecessor native effects. Configuration from an old generation cannot silently rebind to a new generation. Actual loss/recovery observes the owners' journals rather than manufacturing a successful output.

## Concurrency and dependencies

1. This native proposal/specification precedes implementation claims. No lifecycle gate is run by workers.
2. Public canonical configuration/current operator authority and the actual NativeRuntime route/completion hook may be implemented concurrently after their exact interface contract is fixed.
3. The scope factory/current authority/retained-worker source may proceed concurrently with those two slices. It depends on the concrete actual scope application/consistency APIs, not on a production source artifact existing already.
4. Public loader/InstalledHost composition depends on all three source interfaces. The current collaboration installed-host fixture depends on that coherent composition and the genuine materialized callback producer.
5. Integrated builds, real baseline/negative/crash proof, source-policy checks, review, lifecycle validation, synchronization, archive and publication are parent-owned serial gates after source integration and the genuine signed source/admission cohort exists.

Dependencies on startup/source-binding, durable node hosting and collaboration application APIs do not constitute proof that those separate changes or production prerequisites are complete. The current consumer is the installed `molten.collaboration-scope` service; future unrelated effects require their own governed explicit composition rather than a dynamic plugin registry.
