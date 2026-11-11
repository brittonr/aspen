# Authored-input audit for the installed native effect rail (2026-10-31)

Worktree `/home/brittonr/.cargo-target/aspen-native-effects-20261031`, branch
`cairn/route-native-effects-20261031`, base `6b39b39f2` (published mainline). This record covers
tasks 13, 18, 22 and 23 of this change; none of them is checked.

## What exists and compiles

The authored inputs from the earlier interrupted attempt were audited against the current
mainline API. They now compile as a `collaboration_native` test target:

- `tests/collaborationnative/producer/` (moved from `producer.rs` so nested modules resolve):
  `source.rs` (independently validated signed output, ELF check, retained artifact and
  provenance records), `profiles.rs` (execution/native/resource profiles and experimental
  mechanisms), `ports.rs` (fabric port descriptors and requirements including the commit port),
  `service.rs` (admitted manifest and native executable evidence), `execution.rs` (bounded
  execution template and callback context), `transport.rs` (three reserved loopback Iroh
  endpoints and peer bindings), `runtime.rs` (the admitted `native_runtime::Input`, the
  `collaboration-runtime.preserves` artifact and the `service-host-effects` configuration).
- `tests/collaborationnative/authority/` (moved from `authority.rs`): host operator trust and
  exact `load`/`install-effects`/`route-effect` credential producers, scope operator trust,
  scope holder credentials keyed by the staged effect, and the intent/staged-command producer.
- `tests/collaborationnative/producer/activation.rs` (new): advances the declared consistency
  group to `Active` through the public `canonical_group`/`plan`/`normalize`/`apply_outcome`
  sequence, mirroring the crate-private `native_runtime::config::group::activate`.

API drift fixed during the audit: `molten::reliability` → `molten::live_reliability`;
`fabric_transport::cross_process::*` is now re-exported at `fabric_transport::*`; the removed
`fabric_transport::protocol_value` is replaced by a canonical protocol identity record;
`REQUIRED_CONSISTENCY_NON_CLAIMS` is replaced by the two explicit non-claim strings the
consistency tests use; `DirectoryRoot::try_clone_dir` is crate-private, so the fixture creates
its node directory through the workspace path; `preserves_rail::record` takes a `'static` label;
`RelativePath::parse` takes `&str`.

## What is still missing

The driver that starts the three signed `node serve` processes and asserts the path is not
written. Its exact choreography, derived from the current source, is:

1. Per node: provision host operator trust and scope operator trust, write
   `service-host-startup` (`node-service-host-startup-v1` with the bundle locator and tick), sign
   `load` and `install-effects` credentials keyed by their request refs, then start
   `node serve --state-root <path>` and wait for `node-service-host-ready-v1`.
2. `submit` the staged command payload as durable work, then `deliver` it with a `route-effect`
   credential and the scope credential keyed by the staged effect ref.
3. Assert the deliver receipt, the committed scope state and the generation-fenced native
   completion callback.

Two live facts must be discovered by execution, not assumed:

- `restage` requires `observed logical_now == operator scope trust position`, and
  `logical_now` is the live applied index of the linearizable read. The operator trust can be
  re-published (it is a plain write), so the driver must learn or probe that index.
- The `route-effect` credential binds the exact effect including the native incarnation, which
  is known only after the service's Initialize callback.

Because the fixture is not yet driven, the test target only asserts that the authored inputs
exist; the fixture modules stay uncommitted until the driver makes them used code, so the tree
stays warning-clean.

## Not claimed

No native effect, commit, completion or process-loss evidence is claimed. The earlier
`2026-10-19` audit note that "no live scenario has been run" still holds for this change.
