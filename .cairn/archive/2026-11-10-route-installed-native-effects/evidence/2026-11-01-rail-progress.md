# Native effect rail progress (2026-11-01)

Worktree `/home/brittonr/.cargo-target/aspen-native-effects-20261031`, rebased onto
`35478ffb0`. The authored inputs and a real three-process driver now exist and compile; the rail
has not passed yet.

## Executed so far

- The driver prepares three node roots from one independently signed producer output
  (`/nix/store/wmgh5x3nqk4r1wx24xlyzrhs8nmdigis-…`, signed with the experimental key, trust
  `molten-producer-builds/trust-codec/good.trust.json`), starts three `node serve` processes and
  drives `status`, `submit` and `deliver` through the public service-host commands with exact
  host credentials.
- First real run (before the codec fix) failed in the node at bundle decode:
  `content ref must start with blake3:, got molten.fabric.transport.command.v1`. That is a
  product defect, fixed and published separately as `admit-fabric-schema-labels-20261031`.
- Second run failed because the driver pre-signed a `load` request built from authoring-time
  validation evidence; the node derives the current-process validation reference instead. The
  driver now reads the exact load request from a real `--preview-load` and signs that, and uses
  its `state_ref` for the `install-effects` request.

## Runs after the codec fix

- Node startup admitted the signed selection only after the driver ran `node init` and installed
  `source-gate-trust.json` before `node run`; a harness that wrote the trust after initial startup
  was denied.
- `serve` admits a live `status` operation at the declared startup tick, so the operator must
  authorize that exact request before the process starts.
- The operator window was one hour. Every public operation re-authenticates the signed source, so
  the window expired between the first preview and the delivery; the driver now uses the twin's
  twelve-hour operator window.
- Membership exists only after the bundle's admitted recovery group is installed: the manifest now
  declares one recovery group and the driver invokes `install` before `status`.
- The rail now reaches the installed native extension. `install` starts the member and the node
  reports `system-extension start callback failed`. The callback binary accepts a synthetic
  Initialize and Start envelope locally (both exit 0, 1082-byte outcomes), so the failure is in the
  node's own start-callback dispatch, not in the extension input.
- The signed output predates worktree `HEAD`: it was built at `ea61f046309a` and later
  `746218e60`; the worktree is at `f3318e932`. Diagnostics added to the worktree executor do not
  appear because the node process is the prebuilt store binary.

## Rebuilt, signed output and the real defect

A current output was built and signed through the repository's own path
(`nix build path:.#packages.x86_64-linux.molten-startup-source-binding`, then
`nix store sign --key-file <experimental secret>` and a trust record over the measured
`binding.json`, `policy` and `profile`). The gate passed inside the build
(`octet gate pass receipt=blake3:1b15a40e…`). Signing stays an operator effect; the key is the
explicitly experimental one, not production approval.

Against that output the rail reached the installed extension and failed at the start callback.
Reporting the executor's own code in the failure receipt (a product change: the receipt already
recorded `decision`, and now records the executor's bounded code and detail) named the branch:
`decision=executor-failed diagnostic=native-callback-process-denied`.

Root cause: the lifecycle authority's first incarnation is zero
(`plan_lifecycle_transition`, `Absent -> Install` sets `incarnation: 0`, and `BeginStart` does not
advance it), while the collaboration bridge treated a zero incarnation as an absent predecessor and
denied it as stale. Initialize therefore succeeded and Start exited non-zero, which the host
reported as a process denial. The bridge now rejects only a zero *generation*: absence is carried by
a missing state value, not by a zero field, and the authority's first incarnation is legitimate.
Regression tests cover both directions
(`the_first_incarnation_is_zero_and_survives_the_start_callback`,
`an_absent_generation_is_still_stale`).

## Further harness corrections found by the real rail

- Execution request limits must match the admitted profile: a request asking for 8 MiB of stdout and
  stderr needs 16 MiB of diagnostic budget, and the grant now carries 24 MiB.
- Scope trust publication is content-addressed and idempotent; a repeated holder nonce made two
  identical credentials and the second publication failed with `EEXIST`.
- Delivery retries must advance the logical tick; a repeated tick reuses holder nonces and the node
  refuses the second load as replay.
- Durable work needs a current claim: the driver now invokes `readmit` and presents that exact
  `claim_ref` in the delivery.

## Where the rail stops now

The rail executes, against the rebuilt and signed output:

1. three `node init` + `node run` + `node serve` processes, each admitted by the signed selection;
2. `--preview-load` and the exact load credential, then the load-authorized preview;
3. `install` of the bundle's one admitted recovery group, which starts the native member
   (Initialize and Start callbacks both accepted);
4. `status` (member generation/incarnation observed), `submit` (durable work), `readmit` (current
   claim) and `deliver` (accepted, receipt returned at tick 6).

It then polls `status` and the work stays `unknown`; the last four-hour run ended with
`work blake3:f8ee8ded… remained unknown within 12 status observations`, and the serve log tail
holds only the bound-endpoint and ready observations (no error).

`unknown` is the host's pre-dispatch work state; it is only replaced when the callback dispatch is
executed with no approved effects or with completions covering every approved effect
(`servicehosting/work.rs`). The delivered callback returns exactly one typed `commit-staged` effect,
so durable completion now depends on that effect being routed through the Raft-backed commit port.
That routing outcome is not visible in the delivery receipt, which records only the acknowledgement.

## Why the commit effect never completes

Two separate causes were found, in this order.

**Fixture: wrong selector path.** The node reads its effect configuration from the literal state-root
path `service-host-effects` (`servicehost/effects/config.rs::SELECTOR`); the fixture wrote
`collaboration-effects.preserves`. `effects::install` returns `Ok(())` when the selector is absent, so
no router was ever registered and every approved effect became `NativeEffectDisposition::Unrouted`.
That is why work stayed `unknown`: `servicehosting/work.rs` only completes work for
`NotRequested` with no effects or `Completed` covering every approved effect.

**Fixture ordering, not a demonstrated product ordering defect.** With the selector present before
activation, startup fails at
`effect configuration differs from actual active service/port/generation/policy/resource`.
The earlier conclusion that startup was the only installation caller was incorrect:
`servicehost/command.rs::invoke` also installs configured routers for non-preview `deliver` commands.
Startup restores a previously activated supervisor through `Supervisor::from_recovered`.
The accepted `native_effect_installation` requirement explicitly requires a Running supervisor,
and `native_effect_lifetime` requires admission failures to prevent readiness. Silently skipping
non-Running members would weaken that boundary, so the attempted deferral has been removed.
The fixture now activates each admitted recovery group through the actual public `install` command,
then publishes `service-host-effects`, signs startup requests, and starts the serving process.
No product lifecycle or authority check is bypassed by this ordering.

The delivery receipt also dropped `RuntimeDelivery::effects` entirely, so an operator could not tell
`unrouted` from `completed`; it now carries `node-service-host-effect-outcome-v1` with the
disposition and completion refs.

## Continuation admission observations

The corrected activation-before-selection fixture was executed with
`MOLTEN_TEST_SOURCE_TRUST=/home/brittonr/.cargo-target/trust-20261102i.json` and the real ignored
`collaborationnative` test. This is the existing signed incarnation-fixed binary, not a newly
measured output containing the delivery-outcome schema changes.

- First execution compiled and reached runtime admission, then node 0 exited before readiness
  with `collaboration native runtime: missing measured fabric binding` (test exit 101).
  The producer incorrectly selected the first seven sorted manifest bindings with `take(7)`.
  It now selects the exact transport, durable-log, snapshot, timer, entropy, membership and
  placement port IDs; schema admission still checks exact versions and measured binding refs.
- The next compiled execution passed that check, then node 0 exited before readiness with
  `collaboration native runtime: runtime identities or bounded resources differ from admitted host`
  (test exit 101). The runtime input requested 60,000 ms although admission caps it at 30,000 ms.
  The producer now requests 30,000 ms; native callback execution limits are unchanged.

Neither execution proves installed readiness, effect routing, a commit, or durable completion.

The subsequent 30,000 ms execution compiled, passed the driver's three hosted-readiness waits,
and reached delivery. Delivery failed because an exact `service-host-credential-….preserves`
file was missing (test exit 101). The one-shot deliver path reopens the runtime and requires
`install-effects` for its own tick and current load validation ref; the fixture had signed only
the startup installation. Delivery authorization now signs that separate exact grant. The fixture's
speculative increasing-position delivery loop was also removed: a possibly performed effect must
remain Unknown, not be retried under guessed logical positions.

A new source-binding candidate was built successfully with Nix and its genuine Octet source gate:

- output: `/nix/store/wxsjkgij4mr0fanjxxildjgb83mfknfl-molten-node-source-binding-v1`;
- gate: `blake3:b3197a979bdb754481772a7180e09458840f6bda8ae74911d12c64f0550c13c2`;
- producer summary: clean, zero findings, zero warnings, zero errors;
- signature: existing experiment-only key, `nix store sign` reported one signature added;
- measured selection: `/tmp/molten-effect-routing-trust.json`.

This proves candidate build/source admission only, not native effect completion or release approval.

The separately authorized delivery reached actual fabric routing but failed. The old signed binary
reported only `system-extension bound fabric-port routing failed`. The dispatcher had discarded
the typed `FabricPortError`; it now uses the existing `Failure::from` conversion. The system-extension
library regression selection passed all 13 tests after that change.

A newly built and experimentally signed candidate with that conversion passed its source gate:
`/nix/store/78ywndx4zr28r739fayy454d8g6xmiyw-molten-node-source-binding-v1`, gate
`blake3:dc18308f93871bde44575dd03e934c2a304d58bb94016db8cb0a31429069ea1d`.
Its executed rail failed inside the collaboration worker with
`malformed observation: collaboration mutation denied or completion uncertain` (test exit 101).
The operator route gate was therefore passed, but no commit or completion was demonstrated.

The worker now preserves only the closed canonical `collaboration-scope/*` denial codes; arbitrary
backend strings and strings that merely begin with a canonical code remain redacted. Both privacy
regressions passed. The first candidate containing those regressions was correctly denied by the
source gate: primary `path_segment_repetition` finding in the new test name, despite a clean library
summary. The offending test was renamed; no denied output was signed or used as an admitted source.

Source inspection also found a missing input dependency: the fixture supplied staged-effect bytes
but never published the canonical proposal bytes addressed by its `command_ref`. It now publishes
the actual proposal value through `CapabilityCallbackValuePort` on each application replica before
submission. This is input provisioning, not a staged authorization decision or fabricated commit.
The fixture passed focused Clippy with warnings denied after that publication change.

## Retained-owner boundary and currentness continuation (2026-11-05)

The next admitted, experimentally signed source output was
`/nix/store/4acys0772a2hfvdj7b98zk61h7qvrj4r-molten-node-source-binding-v1`, gate
`blake3:187364a924703b1fbc6f594a9e62a3c754f5812906cc66059de2d3921e9683ff`.
That output predates the retained stdio control and currentness-observation APIs below.

The real fixture exposed a liveness false positive: retained ready text could outlive its node.
The driver now checks its owned child before accepting readiness, retains a finite 12-hour
listener budget, and kills/reaps only its owned child on failure. The actual exited-child
regression `driver::exited_child_cannot_supply_readiness` passed. With genuinely live owners,
the old one-shot operation path then failed at tick 3 with:

```text
load-authorized operation preview denied at tick 3:
error: invalid harness artifact: durable adapter error: Database already open. Cannot acquire lock.
```

This disproves the one-shot fixture's store-ownership assumptions. The replacement is explicit
local stdio control through the actual retained `Owner`, not a second Supervisor or a test-owned
runtime. The actual network listener remains owned while commands execute; EOF cancels and joins
owned activity. Startup/source, operation, effect-install, route, and scope mutation authorities
remain separate. The default serving path is unchanged.

Source inspection identified a second missing input: staged mutation currentness must equal the
actual quorum-read state, term, applied index, trust reference and read receipt. Genesis guesses
cannot supply it. The new separately authorized `scope-observe` operation returns plain data from
the actual retained scope worker. It neither constructs `CurrentState` nor consumes its read.
Protected restaging retains its independent live read, durable fresh probe, root recheck, scope
grant verification and read consumption. The fixture aligns trust before a final observation,
then freezes that trust while preparing its exact credential and single protected delivery.

The consumer now selects the actually observed replica and derives later operation ticks from
that observation. Its post-shutdown checks reopen real native/application journals, validate the
materialized committed result and native completion-bound readiness, and require identical durable
work completion after a new source-admitted status operation. These checks are implementation,
not evidence of successful execution yet. Fresh compilation, source gates, measured admission,
the real three-process rail and negative/process-loss coverage remain required.

Strict Cairn structural validation passed earlier with the sibling provider policy, no findings,
85 specifications and 7 changes; receipt
`8c11e4f52f9bfb340d8047e962c17fc8ef625da458c9b0640a5566df675ccc7e`.
This is not runtime acceptance, change closure, release approval or permission to publish.
The first integrated currentness implementation passed these focused checks before the
negative-case harness extraction:

- actual `molten node serve --help` exposes the opt-in local flag;
- actual CLI rejects that flag without `--live-iroh` (exit 2, before opening the state root);
- `cargo test --offline -p molten --bin molten control::tests`: 6 passed;
- `cargo test --offline -p molten --test collaborationnative`: 10 passed, signed rail ignored;
- `cargo test --offline -p molten --lib collaboration_scope::`: 31 passed;
- `cargo clippy --offline -p molten --all-targets -- -D warnings`: passed.

Compilation first found a borrowed `Cow<str>` where the typed control action needs an owned
service name; `into_owned` fixes that without copying an already-owned value. Formatting was
applied after its reported diff. The scope observation path decodes its operation value once.
No integrated signed consumer success follows from these checks; the new source-binding build
and genuine baseline/negative executions remain outstanding.

### Integrated source-policy denials and repairs

The first integrated retained-owner/fault-case candidate was denied: its actual primary
summary contained 72 findings and its library summary contained 9. The denied gate receipt was
`blake3:5f7dcc14785b3eec42fd6a5ceb73254881ea10223e8610a3401f43ec18d5dfbf`;
producer artifacts remain at
`/nix/store/xcc31byi8fy7q6rsmyygfz5js2qvxqiv-molten-retained-startup-octet-cohort`.
Repairs use qualified concrete owners, explicit result handling, bounded collections,
non-underscore module filenames and environment reads at the actual test boundary.
No denied source output was signed.

The next candidate reduced both actual summaries to one `drop_must_use` finding at
`src/collaborationscope/native/runtime.rs:132`. Both summaries remain at
`/nix/store/rxp7yldlsjlzlb679d9dys212iilqp5p-molten-retained-startup-octet-cohort`.
The source gate correctly denied receipt
`blake3:5360e089c429bfa6cc2435878a12b2546b98b31996dfb5ff640d76d9f7054c64`.
The worker now handles the optional sender explicitly before dropping the owned sender.
A new build is required; the denied candidate remains unsigned.

The worker stop notification is independent of observer senders and queue capacity; shutdown
joins the worker after its accepted operation finishes. Retained-control full-queue cancellation
and parser/privacy selection passed all 7 tests. Integrated Clippy with warnings denied passed
before the final explicit-sender repair. The scope library selection passed all 31 tests after
that repair. These checks do not establish signed native execution.

Actual replacement readback now includes semantic Scope replay admissions from the durable
ordered store: exact reservation keys and request digests, checked against canonical value
identity. This compares replay state rather than incidental journal layout. The genuine signed
replacement execution and its retained observations are still required.


The integrated tree then passed the nonignored collaboration fixture selection (10 passed,
1 signed rail ignored), retained lifecycle selection (7 passed), system-extension library
selection (13 passed), formatting check, and all-targets Clippy with warnings denied.
Strict Cairn validation again reported no findings, 85 specifications and 7 changes, with
the structural receipt recorded above. Two independent read-only reviews of retained control/
fencing and Scope currentness/admission reported no actionable patch-introduced defects;
neither reviewer ran checks. These are source/structural observations, not runtime acceptance.

The next actual producer, at
`/nix/store/lxj7rsf8gpld4vg167x8rgrbdq58drgv-molten-retained-startup-octet-cohort`,
reported zero library findings but 8 primary fixture findings: 5 `function_length`,
2 `bool_naming`, and 1 `non_trait_imports`. The gate denied
`blake3:4ef44bdbb06784e6968341661a4fa749b5d10137a2c073800a63083b3857bca5`.
This candidate also remains unsigned. Repairs extract cohesive fixture phases, remove
the concrete-type re-export, and name predicates explicitly; all genuine authority,
Unknown, actual store, proposal, completion and replacement assertions remain required.

### Fresh zero-finding source admission

Both actual summaries in
`/nix/store/az4dx33bnn94g2d1i6nhc9painva54yn-molten-retained-startup-octet-cohort`
are clean: zero findings, warnings and errors. The source-binding build passed gate
`blake3:5cb4a2471f0ee4202b2e2f122177c7cc63fbacd863cf3cf8c22687b43844306f`
and remediation plan
`blake3:ad5e08d794a11f4d5405ac81496e98cfd737e1e744d7344f597a0e4609f9804f`.
Output:
`/nix/store/5xh9khp7v24wbb4q5firpz4jskvfjmwx-molten-node-source-binding-v1`.
Only this passing output received the experiment-only signature (one added).
Measured raw-byte selection:

- binding: `blake3:fa6158c8e2aff35ec040680a9eb2c02fae4fa3c03c1bbef71d0a802aea0e286e`;
- policy: `blake3:e045c1f58ded52d1d1454096b8fc7618cebab06fd4b7b250c1be93a88bce394c`;
- profile: `blake3:011767705af95b9a470f3ac8844d89ead9e21deb336471fb703ed878e720f66b`.

Trust selection is `/tmp/molten-effect-routing-trust-20261105-zero.json`.
The genuine ignored rail ran against it with explicit evidence directory
`/tmp/molten-effect-routing-evidence-20261105-zero` and failed before protected delivery:
`no stable real quorum observation within bound: collaboration-scope/uncertain`
(0 passed, 1 failed; 4780.52 seconds). No commit or completion is established.
The driver had prepared and started each owner before preparing the next one;
it now completes all three exact startup preparations before spawning the owners.
This removes a fixture-created peer-absence window without changing product quorum,
timeouts or uncertainty. A new actual run uses evidence directory
`/tmp/molten-effect-routing-evidence-20261105-coherent` and the same admitted product
output. It failed after a real quorum observation, at protected delivery:
`protected delivery did not complete: collaboration-scope/uncertain`
(0 passed, 1 failed; 5645.16 seconds). Protected delivery was not retried.
No successful commit/completion or negative-case execution follows from this failure.

The authored fixture still used a 20 ms heartbeat and only 4096 accepted ingress
deliveries. Its finite delivery budget is inappropriate for the observed long-lived
signed-source run: even without client traffic, two replies per heartbeat consume
4096 leader ingress deliveries in 40.96 seconds at the nominal schedule. This
calculation does not identify the actual terminal error in the failed process.
The fixture now explicitly admits a 2000 ms heartbeat, 6000–12000 ms election interval,
and 65536 ingress deliveries; at the nominal reply rate, that budget exceeds the
12-hour operator window. Product quorum, timeout, delivery-limit exhaustion, and
Unknown behavior remain unchanged. The fresh independent cohort at evidence root
`/tmp/molten-effect-routing-evidence-20261106-lifetime` also failed before stable
quorum observation with `collaboration-scope/uncertain` (0 passed, 1 failed;
4563.94 seconds). The fixture-only source build passed gate
`blake3:9865e3000e1f6f74a102f6aa0496708ff7dc4f7422edfcdcc3fc527b5be6623e`.
After the fixture extraction repairs, the nonignored fixture selection again passed
10 tests (signed rail ignored), formatting check and all-targets Clippy passed.
A throwaway smoke invoking the actual canonical frame-sizing implementation returned
506 bytes with its separating-bound assertion satisfied; this is resource-policy
construction evidence, not an actual queued proposal or consensus observation.

### Retained ingress idle lifecycle repair

Source diagnosis identified a concrete independent failure mode: the admitted
5000 ms pre-session accept deadline terminated `ReplicaIngressPump`, and the
scope worker permanently marked closed ingress uncertain. An idle listener is not
a failed accepted session; the direct listener's existing timeout test already
requires zero accepted/active sessions and readiness afterward. The new bounded
`poll_one_derived_frame` returns no frame only for that pre-session idle deadline.
The retained pump keeps listening and counts only actual deliveries. Existing finite
one-shot receive APIs retain timeout errors; handshake/frame/protocol failure and
successful-delivery-budget exhaustion still terminate ingress. No accepted message
or protected operation is retried.

The new real-Iroh regression waits beyond multiple accept deadlines, then exchanges
an actual canonical Raft envelope and requires its exact transport acknowledgement,
event, one accepted session, zero active sessions and successful owned shutdown.
It passed. The full `fabric_consistency::raft::iroh_tests::` selection passed 6 tests,
including malformed-payload denial before acknowledgement and canonical delivery.
An earlier mis-scoped filter selected zero tests and supplies no verification.
The failed signed runs do not identify their actual ingress terminal cause; fresh
signed execution of this lifecycle repair is still required.

The idle-fix candidate passed both actual Octet summaries (zero findings, warnings,
errors) at
`/nix/store/555fiv92b8s3j9yjf0093prl5njr8hl0-molten-retained-startup-octet-cohort`
and source gate
`blake3:5a505a5fe35d809359e7388174a138474a1d9c2fb497f94f9a5108dc162b4b4e`.
Its output is
`/nix/store/hfhqchmd5022l0nn77i4cp37glh1gqas-molten-node-source-binding-v1`,
experimentally signed only after those checks. Raw-byte binding selection is
`blake3:afe0dea93ed63c32e1ddadf04a7ebc2d7f52a924e2f6dbe01ccde8074d3c9d0a`;
policy/profile selections are unchanged from the earlier zero-finding output.
Trust is `/tmp/molten-effect-routing-trust-20261106-idle.json`; the genuine run
at `/tmp/molten-effect-routing-evidence-20261106-idle` failed after a quorum read
with `protected delivery did not complete: collaboration-scope/uncertain`
(0 passed, 1 failed; 3050.72 seconds). Idle-safe acceptance alone did not complete
the protected path.
All-targets Clippy, formatting check and strict Cairn validation passed after the
repair; strict validation retained its earlier structural receipt and no findings.

Temporary fixed-vocabulary route-phase diagnostics were then compiled and source-admitted.
The initial tracing-only diagnostic candidate passed gate
`blake3:e12029c908e550659cac80b67b8438f3e61d8138c2f0525cd1f3207f55900b51`
with both actual summaries zero, was experimentally signed, and failed delivery in
3196.06 seconds; no phases were observed on any node. No tracing initialization was
found in the CLI entrypoint files inspected; absent tracing lines supply no failure-location evidence.

The visible fixed-stderr candidate passed gate
`blake3:1a0edae6813482ed9b1626c03b4a89900f640639064605131a57330b8cd6b356`,
again with both actual summaries zero, and was experimentally signed. Its actual
delivery failed in 2964.17 seconds, with `replica-effect` and `commit-observation`
observed only on node 1. No restaging or commit-deadline phase was observed. This
locates a failed replica effect during commit observation; it does not establish
which effect failed or whether an application mutation became durable. A further
temporary diagnostic records only the typed effect kind and exact closed Scope
denial vocabulary. Raw backend text, payloads, tokens and credentials are not emitted.
All temporary diagnostics must be removed after the actual defect is located.

### Buffered append admission mismatch

The typed-effect diagnostic cohort passed both actual Octet summaries with zero findings
and source gate
`blake3:6ff3bcd97fbf382e4d1b1dbcc9d31900e7dc196c60611fba0c61fa448808aea6`
before experimental signing. Its real delivery failed in 3685.14 seconds with
`persist-entries` observed as the failed effect on node 1. Restaging completed;
no application-commit or completion success is claimed.

`RedbReplicaPort::persist_entries` requests `Buffered`, then `flush_log` separately
requests `MachineLoss`. The producer used the shared experimental profile, which
admits only `machine-loss` in both profile and namespace atomicity domain. A throwaway
smoke through the actual `append_log` decision reproduced
`Err([UnsupportedDurability(Buffered)])`. With both levels explicitly admitted,
the same exact value became Buffered and then Durable after the machine-loss flush;
the durable value ref matched the submitted bytes. This is deterministic mechanism
proof, not evidence that the signed native consumer completed.

The producer now explicitly authors `buffered` plus `machine-loss`, recomputes the
profile identity, updates its namespace profile reference and atomicity domain,
and admits the resulting profile/namespace before runtime construction. It does
not reinterpret buffered acceptance as durable completion. All temporary route-phase,
effect-kind and closed-code diagnostics and their collector were removed after this
reproduction. A fresh source build and genuine integrated run remain required.

### Durability-corrected execution and exact completion identities

The extracted fixture durability helper passed a fresh zero-finding primary and
library source gate, `blake3:28d9fe330ec5c86e9b7035f33ce9dafd052625b461a3f39cd6cbc830c5f493e8`.
The experimentally signed admitted output is
`/nix/store/wh7dg1i5n0mib48gzdryzhl7crqncbvk-molten-node-source-binding-v1`.
Its raw binding selection is
`blake3:ba60c58a8636517e6a5cacd32bd8b7e22b6495701c0d72c10a38bb3eab71cb35`;
the policy/profile selections remain those of the earlier admitted cohort.
Trust is `evidence/artifacts/molten-effect-routing-trust-20261106-durable.json`.

Two fresh, independent real three-node cohorts reached successful protected delivery,
exact live work completion, and post-shutdown native completion-bound readiness,
quiescence and terminal-effect readback. They did not finish the positive acceptance
proof or execute the six negative cases:

- `/tmp/molten-effect-routing-evidence-20261106-durable`: 0 passed, 1 failed,
  2809.72 seconds. Readback reported `actual committed value unavailable` with the
  opaque callback-value access failure while trying to materialize the provider
  terminal output ref as a separate cache entry.
- `/tmp/molten-effect-routing-evidence-20261106-inline`: 0 passed, 1 failed,
  3814.24 seconds. Readback reported `actual cached value unavailable` with the same
  opaque failure while treating the returned Message callback receipt as the
  effect-completion wrapper's cache address.

These are distinct identity domains. The provider output is inline in the actual
cached `system-extension-effect-completion-v2`; the Message callback receipt is the
public completion/readiness boundary. Neither is its wrapper's cache address. The
proof now performs bounded capability-rooted enumeration of actual immutable cache
values, verifies canonical bytes and exact address identity, and selects a unique
completion matching the actual native Request ingress receipt, effect input and
terminal provider output. Positive proof also requires the native effect operation
in `completed_operation_refs`; it retains the actual wrapper, decoded provider
result and freshly authorized proposal. No completion or output is reconstructed.

An intermediate source-only readback candidate passed both actual summaries with
zero findings and gate
`blake3:20895dfa91d8ebe8e0081ba331a95be2b5bc172f81f03975580133e0d735cee0`.
It was not signed or selected as the executing product and predates the final
cache-enumeration correction. Fresh execution and source checks remain required.

The first bounded-cache readback candidate passed both summaries with zero findings
and gate `blake3:a36e0d346806a24dc04b2c9ee427b2b48f772a3139f566a42adbc60b6d9fd36d`.
The integrated queued/post-effect proof candidate also passed both summaries with
zero findings and gate
`blake3:bf4a54fcfa45c8d80d7d927f58e8eb08532d831aad78b8c8ad3ffb7d25157a39`.
Neither intermediate candidate was signed or selected as an executing product.

The final shared observer checks the canonical completion schema and derives the
exact native effect operation identity from its request, binding and generation;
the actual terminal native journal entry must match. It searches only regular
canonical cache values, never following blocked directories or other non-regular
entries. This is selected-value readback, not a whole-cache integrity claim; the
post-effect failure fixture intentionally obstructs a future state leaf with a
directory. Queued proof validates and retains the actual fresh proposal, and denial
proof rejects any append for the exact request regardless of proposal identity.
Post-effect proof retains the actual cached completion, inline provider result and
fresh proposal while still requiring a rejected future-state publication and an
unconsumed native effect.

Formatting check and fresh primary/library source summaries passed with zero findings,
warnings and errors for this observer. Source gate:
`blake3:1448314d4fe2aa7a17f1b620dfddc1ac41f3e6c017e0d5afd639004c60297303`.
Admitted output:
`/nix/store/lk7kbw7lygb3930mljqrbw930m5wqjda-molten-node-source-binding-v1`.
One experimental signature was added only after these actual checks. Its measured
raw binding selection is
`blake3:876419018c61a98bfad12eb3e8fa5a216a8bc6b5ea4142e8f02579e242b829d6`;
policy/profile selections remain unchanged. Exact trust is
`evidence/artifacts/molten-effect-routing-trust-20261106-observer.json`. These are source-admission
facts, not proof that this candidate has completed the actual consumer scenarios.
Strict Cairn structural validation passed again: 85 specs, 7 changes, zero findings,
receipt `8c11e4f52f9bfb340d8047e962c17fc8ef625da458c9b0640a5566df675ccc7e`.

The bounded-cache cohort using the durability-corrected signed product failed after
6387.35 seconds (0 passed, 1 failed) with
`Preserves("position 0: Unexpected(Eof)")`; its evidence root
`/tmp/molten-effect-routing-evidence-20261106-linked` contained no retained positive
artifact when inspected. No source-admitted completion reopen or negative acceptance
is inferred from that run.

Source inspection found a format-contract error in the observer: callback cache
values are bounded, content-addressed opaque bytes, not necessarily Preserves.
A throwaway smoke through the actual `CapabilityCallbackValuePort` published and
materialized correctly addressed empty bytes, then reproduced exactly
`position 0: Unexpected(Eof)` on canonical parsing. Canonical candidate classification
excluded those opaque bytes and retained the exact supplied canonical record. This
establishes the observer's invalid assumption; it does not identify the failed
cohort's particular cache leaf, which was not retained for inspection.

The observer now checks the immutable address before classifying bytes, skips opaque
non-Preserves values, and still requires a unique actual canonical completion with
the exact schema/native Request/terminal output/binding/generation links. A missing
or malformed selected completion cannot become success. No special empty-value
fallback, synthesized record or protected-operation retry was added.

The opaque-aware observer passed actual primary and library summaries with zero
findings, warnings and errors, formatting check, and source gate
`blake3:d5327245a8acfb79a81b4be2d71ebe2b982855c26097adc15023d1454c25441c`.
Its source-only output is
`/nix/store/zsnk029z89hwzl11z6d6c3bbb9x7q7xs-molten-node-source-binding-v1`.
It was not signed or selected for execution. The fresh fixture executes the already
admitted, signed `lk7kbw7lygb3930mljqrbw930m5wqjda` product using observer trust above.
Measured raw product bytes match across both outputs:

- `molten`: `blake3:72b70ff15af84ebfddf9b299b628274261c12e6d24f93d7805fb104f1ea9cee7`.
- `molten-collaboration-scope`: `blake3:3262462b52e5683aea722aa76534d34592b418fc5b03e5163490cb7494e65aa2`.

Byte equality does not transfer source/installation/route/scope authority; actual
startup continues to verify its exact signed selected output and grants independently.
Fresh focused checks passed: Scope 31, system-extension/native host 13, real Iroh 6,
retained-control lifecycle 7, nonignored native fixture 10 (actual rail remains
separately ignored). All-targets Clippy passed with `-D warnings`.
The current opaque-aware actual run uses its freshly compiled test executable
directly under the pinned Nix shell, with evidence root
`evidence/artifacts`. Direct execution avoids a
second Cargo job against the shared target while focused checks run; it is the same
compiled ignored test, not a fixture substitute. Its positive baseline has now passed;
the six negative cases are still running. Retained artifacts from different attempts share
`evidence/artifacts`; each file name is its own BLAKE3, so the hashes below distinguish the
attempts.

### Retained actual positive baseline

The real signed run emitted baseline observation
`blake3:e178877d583cb9857eb22c4e5de0641a53b3aa1c953d05dfd6baf682d203c46d`.
The retained `.preserves` artifact is 22488 bytes. Independent inspection parsed its
canonical bytes, recomputed its BLAKE3 filename, and observed three actual ready
owners, the actual cached completion/provider/proposal/state and source-admitted
reopened work status. Selected identities are:

- Native instance: `blake3:44afb5c0aa8c12e4cfccdc1d1ec83a8d26ca01df6af52af816081f7d5a93672a`.
- Cached effect completion: `blake3:e8f76239eb838a6b668189d59cf1a1e6f9b9b8580c2f5035f66a8a4ad2fe91da`.
- Provider output: `blake3:b40cc6dc25e5e970573be48ea7b281de70ea11e8064ec7927bd9c26e7c441ca4`.
- Actual authorized proposal: `blake3:402454d66b323243ac7315a94347cb3e85690a685a043ccbe30149248e90d60e`.
- Request: `blake3:4fb29a16131d9a1beeb3e304fc7ace7ddc586df08a9a78cab875b10dff48e361`.
- Application receipt: `blake3:4d933c536fed3deaf804f7d79204085077db0d0f86b8ba073e812e4aa3ada21d`.
- Scope state: `blake3:a6a1e19752742dd6ad176bdbff860a0e9d0ba7a87f486aba8eb39617c3fbe02d`.

The actual retained Raft position names group binding
`blake3:f79f81def8717796c97c26e2d172ea39359c2a554b9633c7d9162bf7f7a2554b`,
generation 1, entry index 1, commit index 1 and last-applied 1. Independent artifact
inspection verified that retained proposal and state hashes equal the provider's
references and that the provider's hash equals the cached completion's output ref.
The actual Message callback receipt
`blake3:b8982736c959c572c47c76b250c9bbfb731850fbf4778452a76a4b9ada3936ea`
is the healthy running readiness boundary and the public completed delivery outcome.
After owned shutdown and actual source-admitted reopen, exact work
`blake3:3a9bda0cc0df164c59040978425e33b072e43c8e5e4403050456c2d4c0423595`
remained `completed` with that same one receipt. This is bounded native-process
positive acceptance, not a VMM, production, global exactly-once or negative-case claim.


### Unconfigured Request receipt is not completion

The opaque-aware actual run ended after 12744.54 seconds: 0 passed, 1 failed. Its
positive baseline above was retained before the first negative case failed with
`collaboration-native-unconfigured: uncertain work retained a completion receipt`.
The later negative/process-replacement cases were not reached.

`Host::deliver` persists `WorkState::Unknown` before dispatch and retains the actual
Request callback receipt after successful native dispatch even when effects remain
Unrouted. That receipt records Request execution, not effect completion. Only the
separate disposition checks promote work to Completed. The fault fixture's generic
Unknown assertion incorrectly required no receipt at all. It now requires exact,
unique Unknown work without imposing that invalid receipt absence. The unconfigured
proof additionally requires its retained work receipt to equal the actual terminal
Ingress receipt for the exact delivered payload. It still requires Unrouted,
absent Scope/Raft namespaces, no terminal native Effect, retry forbidden, unchanged
bridge/journal after source-admitted reopen, and exactly advanced Request sequence.
No product transition or completion authority was weakened.

The corrected fixture compiled and passed 10 nonignored tests (1 ignored). A fresh
independent full rail is running at
`evidence/artifacts`, using the same exactly
admitted signed product/trust and a newly compiled fixture. No possibly performed
protected delivery from the previous cohort is retried.

The uncertainty-corrected fixture passed fresh primary and library Octet summaries
with zero findings, warnings and errors and source gate
`blake3:a9ad05776054c72d70941d44ac96224b6426abfa6483c055b3585502af7c593f`.
All-targets Clippy passed with `-D warnings`. Strict Cairn validation after the
positive-baseline task/evidence update passed with no findings, receipt
`9827f466e6a512d4e0bafd369cd8513e8fa3e5d04bcf4f329fe129a9a2c77bc9`.

### Actual unconfigured pass and missing-credential denial defect

The corrected uncertainty fixture (source gate
`blake3:a9ad05776054c72d70941d44ac96224b6426abfa6483c055b3585502af7c593f`)
produced a fresh independent run using the same admitted signed
`lk7kbw7lygb3930mljqrbw930m5wqjda` product:

- Positive baseline retained as
  `blake3:dca138b957b700152adf1324cac6ebba5f52407b8dfb81b1f5a2f59175dfc2dc`.
- Unconfigured case passed and retained
  `blake3:930808d05eaa22987035a6412e90ed60861a42f999efe7b79c6a998251807c0e`.
  Both artifacts are 20040/22488 canonical bytes with BLAKE3 names matching their
  content. The unconfigured artifact retains exact Unrouted delivery, historical
  peer-scope input, live and reopened Unknown work, unchanged native bridge/journal,
  and absent Scope/Raft namespaces.
- Missing-credential then failed after 24923.68 seconds (0 passed, 1 failed):
  `scope missing did not return its fail-closed denial: operation failed; outcome
  may be uncertain; inspect durable state before retry`. No later negative case ran.

Source diagnosis: `FabricPortError` renders as `malformed observation: <worker
reply>`, and the CLI projects only exact `collaboration-scope/<code>` worker replies;
anything else becomes the generic fallback. Removing the credential made
`authority::config::read` fail with `Failure::Io`, and malformed canonical authority
material would fail with `Failure::Preserves`, so neither produced a canonical Scope
denial even though both are definite pre-effect failures. The worker's redaction
then hid the class.

The authority reader is being changed to classify a missing value as
`Denial::Missing`, a non-regular leaf as `Denial::Domain`, a read failure after a
successful entry observation as `Denial::Uncertain`, and a canonical parse/schema
failure as `Denial::Domain`, while preserving every canonical denial already
produced by identity or policy checks. This changes only failure classification; it
cannot turn any failure into success. No protected delivery from the failed cohort
is retried; the next run uses a new product output and a fresh cohort.

### Canonical scope denial classification and replay-inventory scope

`authority::config` now reads current trust and credentials through one classified
reader: absent value -> `collaboration-scope/missing`, non-regular leaf -> `domain`,
read failure after a successful entry observation -> `uncertain`, canonical
parse/schema failure -> `domain`. The trust/credential decode bodies are wrapped in
one classifier that passes existing `collaboration-scope/<code>` failures through and
maps every other decode failure to `domain`; it cannot map any failure to success.
Eight tests cover absent, empty, directory, valid, and wrong-schema authority values
(the last four added by this change). The full `collaboration_scope::` library
selection passed 36 tests, the native fixture passed its 10 nonignored tests, and
all-targets Clippy passed with `-D warnings` before the integrated build.

A read-only audit compared every remaining negative-case assertion with the actual
product and found three corrections, applied before the next execution:

- Raft reopen comparison used the whole durable log, which also contains election
  hard state, flush markers and snapshots. Observation and comparison now retain only
  actual `raft-log-mutation-v1` and `raft-commit-boundary-v1` records, matching the
  claim and the existing replacement filter.
- Completion-cache scanning aborted on a regular leaf whose name was not
  `<digest>.preserves`. Such leaves are not candidates; the scan now skips them.
- Replacement proof demanded byte-identical callback sequence and state across the
  replacement's own activation. It now requires the unchanged unresolved predecessor
  effect, unchanged consumed-completion refs, unchanged generation/incarnation, and
  no completed Effect for the predecessor request, while allowing the replacement to
  re-observe its own instance.

The audit found no remaining receipt-as-completion, historical-command-ref,
terminal-ref-as-cache-address, or store-reopen defects. `is_retry_permitted` is
hardcoded false throughout the product, so those conjuncts are consistent but
cannot fail; they were left in place rather than replaced with weaker text. The
audit's two latency risks that cannot be settled without execution are the
post-effect fixture's predicted blocked-leaf callback sequence and the exact shape
of replacement activation; both are now asserted more narrowly where semantics
allow.

The first integrated build of these corrections was denied: four
`path_segment_repetition` warnings for test names repeating the ancestor
`authority` segment. Renaming them removed all findings. The rebuilt candidate then
passed both actual summaries with zero findings, warnings and errors:
primary `blake3:c0394c8bc2de65194689bd543729c17712aead456ec994bcf054417c51d6462b`.
Admitted output is
`/nix/store/mayahs93r24rmvx9nbb4fw14f18v5gs6-molten-node-source-binding-v1`; one
experimental signature was added only after those checks. Its measured raw binding
selection is
`blake3:0835866a0192ff8a67e1d855e8d4346255d0a8b2959a57abd14fad9c83acf11d`;
policy/profile selections are unchanged. Exact trust is
`evidence/artifacts/molten-effect-routing-trust-20261108-denial.json`. The full rail is executing
at `evidence/artifacts` on a fresh cohort.

That cohort passed its positive baseline and retained
`blake3:e292fb2c7877bcf870439ddb2a46a763038bbd227539bc47fef384a7d020e29c`
(22488 canonical bytes, three ready owners, actual cached completion and reopened
completion-bound status). It then failed after 12259.50 seconds in the unconfigured
case before any delivery:

```text
collaboration-native-unconfigured: real configured peers 1/2 could not supply scope
metadata; ... no stable real quorum observation within bound: collaboration-scope/partition
```

No protected delivery occurred in that case, so nothing was retried. The diagnostic
metadata selector used a fixed 20-round cap while each failed linearizable read waits
out the worker's 30-second deadline; that cap can expire before the remaining live
voters settle. Only diagnostic reads are retried, and they are data rather than
mutation authority. The selector now retries within an explicit 30-minute elapsed
bound measured by the admitted fixture clock and reports its round count, instead of
a fixed round count. The delivery, authority, and currentness paths are unchanged.
This failure is recorded as an observation, not as a passing unconfigured case.

The first attempt at that change used `std::time::Instant`, which the source gate
denied for ambient clock use; it was replaced with the existing admitted wall-clock
helper. The rebuilt candidate then passed both actual summaries with zero findings,
warnings and errors and source gate
`blake3:66b4cdbd3d0e1f225fb6e8c5bf30e15ad97950ce2d88bd0dcecadb40e70befe1`.
Admitted output is
`/nix/store/iiwg9dh7xxjybl3ll2ij3nxjgxhwv40g-molten-node-source-binding-v1`;
one experimental signature was added only after those checks. Measured raw binding
selection is
`blake3:8b456f409a43af023b946357dd1d1c7db5c36e95fc1b9cdce54a5e9a497f7b38`,
with unchanged policy/profile selections. Trust is
`evidence/artifacts/molten-effect-routing-trust-20261108-bounded.json`; the rail is executing at
`evidence/artifacts` on a fresh cohort.

That cohort's positive baseline is
`blake3:9827a34440f1b5ee3980542d7e49b9e7aa6c94e071ec17235327de0e9c808d23`
(22488 canonical bytes, record `collaboration-native-consumer-observations-v1`).
Independent inspection confirmed three ready owners, the actual cached
`system-extension-effect-completion-v2`, provider output, fresh authorized proposal,
request, application receipt, scope state, Raft position (generation 1, entry/commit
1), the Message callback receipt as both readiness and completed delivery outcome,
and work `blake3:72a4614962f9bda77fe147f420b4ccbc5cdca5ee9d839fa771dc607614f7cf6f`
still `completed` with that same receipt after source-admitted reopen.

The bounded metadata selector then let the unconfigured case pass:
`blake3:b4e80696be394ab75b740fae280bd83e64373a55a57ef11c3d4677a2b2180797`,
record `collaboration-native-unconfigured-observation-v1`. Independent inspection
found the exact `unrouted` effect outcome, Unknown live and reopened work, five
`absent` capability-namespace observations, and the unchanged native bridge.

The same cohort then reached the missing-credential case and passed its fail-closed
denial assertion: the actual delivery returned `collaboration-scope/missing`, kept
Unknown work, retained exactly one non-retryable Unknown effect, left the application
state and proposal inventory unchanged, and survived source-admitted reopen. The case
then aborted in its process-replacement proof on an invalid fixture expectation, so no
observation artifact was retained for it.

The invalid expectation compared the persisted
`node-service-host-replay-admission-v1` digest to the producer request reference.
Source inspection shows the host persists basalt's canonical holder-invocation digest
(over audience, resource, ability, operation binding and nonce), which is a different
domain from `request_ref(bundle, request)`. The corrected proof requires the exact
credential file at that request digest path with the exact nonce, the replay admission
keyed by the actual holder DID and nonce, and a retained 32-byte invocation digest.
It no longer claims the replay value equals the producer request reference.

The corrected fixture passed formatting, its 10 nonignored tests, and all-targets
Clippy with `-D warnings` under the pinned direct toolchain. The source binding
passed both actual summaries with zero findings, warnings and errors and gate
`blake3:146c9e0063954e0d2299da6f1736772fd984f286e96c45fddac2ff81bb442d1b`.
Admitted output is
`/nix/store/pm6jq09vp99474jjvx2fd1wr64qrxvam-molten-node-source-binding-v1`;
one experimental signature was added only after those checks. Measured raw binding
selection is
`blake3:c4415d2facf637e93e8b2c4308d75b11b97bbb71e0207a2815c3b07728d5502d`,
with unchanged policy/profile selections. Trust is
`evidence/artifacts/molten-effect-routing-trust-20261109-admission.json`; the rail executes at
`evidence/artifacts` on a fresh cohort.

Two read-only source reviews pre-checked the still-unexecuted cases:

- Post-effect obstruction: the completion `Message` dispatch is exactly the second
  dispatcher invocation after the status base (Request is the first; `Deferred` does
  not increment and is unreachable under the admitted one-callback/4 MiB resource
  profile), so the predicted `sequence + 2` state and retained `last_intent` are the
  value the executor will publish. A wrong prediction would silently leave the
  obstruction unused.
- Queued without commit: the separating frame budget equals the largest synthetic
  control frame while a one-entry AppendEntries is strictly larger; the frame is
  rejected before dialing. The owning leader persists the entry (buffered append then
  machine-loss flush) before any send, no commit boundary can form without a quorum,
  and delivery returns a normal control error after the commit deadline. Owner
  shutdown/reap precedes reopening the durable store, and the filtered inventory
  excludes hard state and flush records.

These are source-level consistency findings, not execution evidence.

The corrected cohort's baseline is
`blake3:9a03bcc5fd22dfbc9ee55fc3b3077d50541dc01babe6ae099feb1eca20ba8c2e`
and its unconfigured observation is
`blake3:b0226568d7dc659dc1beeef6910648a29650d40c4ec7ee1e5e0f3a40eca8c74b`;
both were independently parsed and matched their content addresses and required
facts. The missing-credential case then passed, including its process replacement,
retaining
`blake3:2d0caf44cb2b676a2f3c7cd02139e1c717b80f0693956d9c70b4babe9187173a`
(36398 canonical bytes). Independent inspection found the embedded
`collaboration-native-scope-authority-failure-observation-v1` with label `missing`
and denial detail `collaboration-scope/missing`, Unknown live/reopened work, no
appended protected proposal, the unchanged native/journal baseline, and the
`collaboration-native-process-replacement-observation-v1` carrying a real ready
replacement (`collaboration-native-replacement-ready-v1`), its denied fenced
scope preview, the actual `node-service-host-replay-admission-v1`, and four durable
per-node inventory records. This is executed evidence for the missing-credential
and replacement requirements, not for the remaining cases.

The stale-roots case then passed and retained
`blake3:c308215569f1317cd63ae4592be73b652dedf7233e59a5fb283268e257d9e2d4`
(36394 canonical bytes), independently inspected to contain the
`collaboration-native-scope-authority-failure-observation-v1` with label `stale`
and denial detail `collaboration-scope/stale`, plus the same replacement record,
ready replacement and actual replay admission. This exercises historical scoped
intent under a rotated current trust root as a canonical `stale` denial rather than
success or generic uncertainty.

The host session then reset and terminated the remaining sequence before the
malformed case, so the last three cases had no execution evidence. The fixture now
supports an explicit `MOLTEN_TEST_NATIVE_CASES` selection (validated labels from
`scenarios::Case::label`) so an interrupted acceptance sequence can be re-executed
case by case against the same admitted product; the default remains the baseline and
all six cases. Only two fixture files changed after the last admitted output, so the
executing product binaries are unchanged. The three remaining cases
(`collaboration-native-denied`, `collaboration-native-uncommitted`,
`collaboration-native-completion-failure`) are being executed as independent fresh
cohorts, which shortens wall time without reusing a cohort or retrying protected
work.

The first post-effect attempt failed after 7656 seconds with
`collaboration-native-completion-failure: invalid harness artifact:
collaboration-scope/stale`. Source diagnosis: the case SIGKILLed the failed owner,
and any unclean shutdown of an existing Redb store triggers Redb allocator repair
on the next open, which the durable adapter classifies as
`Quarantined(RepairedOnOpen)`; `Database::new` then fails closed with `Stale`. A
quarantined store serves only status and explicit reconciliation, not the journal
readback this scenario proves. The case now closes every owner cleanly (the
obstruction is a definite pre-publication rejection, not a crash), asserts the
delivery itself failed, and leaves real process replacement to the separate
replacement proof, which starts from a cleanly closed store. The record wording no
longer claims process loss for this case. This is a fixture correction to match
documented product recovery semantics; it does not relax the failed-publication,
unconsumed-effect or Unknown-work assertions.

The malformed-credential and queued cases then passed as independent fresh cohorts:

- `collaboration-native-denied` retained
  `blake3:c57f1de6a7dcecb7fc05838fc6ca49876f096a92e3c24fe3a7cc5e3efbde4bf9`.
  Independent inspection found `collaboration-native-scope-authority-failure-observation-v1`
  with denial detail `collaboration-scope/domain`, six prequeue inventory records
  (three nodes before and after reopen) with no appended protected proposal, and the
  replacement record with its ready replacement, denied fenced preview and actual
  replay admission.
- `collaboration-native-uncommitted` retained
  `blake3:96cc51451525ddf5a48590a5f1cdaf40dad1ae872ec9ae2609d0780fcf27b9dc`.
  Independent inspection found the queued-uncommitted observation, six
  per-node replica records across the reopen, four durable
  `raft-log-mutation-v1` records with **no** `raft-commit-boundary-v1`, Unknown work
  states, and the same replacement evidence. Queue acceptance is therefore recorded
  as proposal-only, never as a commit or completion.

The post-effect case then failed after 5485 seconds in its replacement proof with
`replacement requires exactly one unresolved Unknown predecessor effect`. That
predicate was correct for route-time denials but wrong for this scenario: its
predecessor effect was *produced* (terminal provider output) yet unconsumed, because
the failure occurred during completion delivery. The replacement check now accepts
exactly one predecessor effect from either the unresolved journal or the
completed-but-unconsumed journal, requires that same operation (identity, state,
terminal ref) to survive, forbids its consumption and any second effect for the same
request, and keeps unresolved/completion-refs/generation/incarnation equality. This
is a strict generalization over the four already-retained cases, whose predecessor
was an unresolved Unknown effect and which passed the narrower check.

The corrected post-effect case then passed and retained
`blake3:a0fbc80e8e5268f2d831076c889af4ff25c9b2b1f7811c2c8690c612a5656664`.
Independent inspection found `collaboration-native-completion-failure-observation-v1`
(obstructed future publication, terminal provider effect, unconsumed completion,
Unknown work before and after reopen, unchanged native and durable inventories), the
replacement observation, six durable per-node inventory records, and the actual
`node-service-host-replay-admission-v1`. Unlike the other cases, this replacement
retained `collaboration-native-replacement-exited-v1` (ordinary exit code 1, no
readiness line): with an unconsumed completion outstanding the product refuses to
bring a new serving instance up, so no implicit reactivation or retry occurred.
All six negative cases and the positive baseline now have executed, independently
inspected artifacts from the admitted product.

### Final integrated source gate

The final tree (fixture selector, clean post-effect closure, generalized predecessor
fence) passed formatting, its 10 nonignored tests, all-targets Clippy with
`-D warnings`, and both actual Octet summaries with zero findings, warnings and
errors. Gate receipt:
`blake3:0223e23557314d2ee642e687637765535574b01293b284e03894953076e6e49c`.
Admitted output:
`/nix/store/6bfbbsznps5f4j8kqsr52p1agg5h6www-molten-node-source-binding-v1`
(raw binding selection
`blake3:1d2c9150a5660a27b1b65c41264f420ecbed20b89f4ecd0bfe5939707b4d7875`,
policy/profile selections unchanged; one experimental signature added only after
admission). Its product binaries are byte-identical to the executed signed
`/nix/store/pm6jq09vp99474jjvx2fd1wr64qrxvam-molten-node-source-binding-v1`:

- `molten`: `blake3:e628695d0fe997bfc3145e5392e63a2cdf89fc0691ad3934923be9fd95f8b35c`.
- `molten-collaboration-scope`:
  `blake3:8a6ae6491067e5f507e00df22986850f5384b136de63ae3d95a9dfd764052b37`.

So every retained artifact was produced by exactly the final tree's product bytes.
The first final-tree build was denied for three fixture lints (ambient env access in
a helper, unbounded selection growth, non-predicate boolean name); all three were
corrected before this passing gate. Strict Cairn validation passed with zero
findings (85 specs, 7 changes, receipt
`9827f466e6a512d4e0bafd369cd8513e8fa3e5d04bcf4f329fe129a9a2c77bc9`).


## Open items

- Preserve the Running-supervisor requirement and current install/route authorization checks.
- If further implementation changes land, re-run the affected case and refresh the source gate.
- The delta spec's "mismatched installation" arm (wrong service/port/group/generation/
  policy/resource/installation or altered runtime artifact bytes) has no dedicated
  fixture; the absent-configuration arm, exact-active-installation arm, and all route
  denials are executed. A worker-startup/ownership-failure observation is likewise
  covered only by the replacement startup branches and the non-ignored exited-child test.

## Not claimed

All six negative cases, the positive baseline and process replacement now have
executed canonical artifacts from a product byte-identical to the final admitted
tree. Not claimed: a killed-store journal readback (an uncleanly closed durable
store is quarantined and serves only explicit reconciliation), global exactly-once
semantics, production or cross-machine behaviour, and release eligibility.

### Executed install-boundary observations

Two additional ignored rail tests close the remaining delta-spec scenarios:

- `installed_native_effect_rejects_a_mismatched_installation` rewrote the published
  selector to a canonically encoded configuration naming a different installation
  identity, then started one signed node. The node published no readiness, exited
  with code 1, and created neither the scope-application nor the raft namespace.
  Retained observation
  `blake3:e67aa6d2bd239526bcefcd717c73ea063e940820485f0aed4dd968df6ea7c23c`
  (533 bytes) records the signed executable, the original configuration reference
  `blake3:1220d0d99db9e808ee7ee275f6b9136800f04d5de84fa53ac9196c6138661cd7`,
  the mismatched reference
  `blake3:4d145f71807ff366959ecb2aaa0ffbc3b82680e84b47d8ade961d001307bc1f5`,
  exit 1 and both absence observations.
- `installed_native_effect_startup_failure_publishes_no_readiness` removed the
  operator scope-authority trust before startup. The node published no readiness,
  exited with code 1, created no scope-application namespace, and left the native
  journal at `namespace-present` / `journal-present` / `completed-effects-absent`
  with no completed effect and no retry-permitted operation. Retained observation
  `blake3:64b8cc1c5188aaabd15ecb7ba157d872ee2aded68e4eb76797524810a9b5f79d`
  (389 bytes).

Both were produced by the signed admitted product
`/nix/store/pm6jq09vp99474jjvx2fd1wr64qrxvam-molten-node-source-binding-v1`, whose
binaries are byte-identical to the final tree's admitted output. They are
installation-boundary observations: the mismatched case proves refusal before scope
resource creation, not a callback route; neither case claims a delivery or
completion.

### Evidence provenance

Every accepted artifact and trust record referenced by this file is copied under
`evidence/artifacts/` with its BLAKE3 file name unchanged; the `.preserves` names
were re-verified against their bytes after copying. Remaining `/tmp/...` references
describe earlier failed attempts whose transient scratch roots were removed after
the session: those runs retained no canonical artifact, so only their recorded
outcomes and hashes survive. No accepted observation depends on a transient path.

### Lifecycle closure

- The six delta requirements were synchronized into
  `.cairn/specs/system-extension/spec.md` (each `r[molten.system_extension.native_effect_*]`
  identifier appears exactly once).
- `review.md` records eight read-only source reviews with findings, resolutions and
  residual risk; no human reviewer signed it, and it is not runtime evidence.
- `tests/evidence-matrix.ncl` now carries positive and negative coverage entries for
  the six requirements, and the non-ignored `exited_child_cannot_supply_readiness`
  test carries `r[verify molten.system_extension.native_effect_lifetime]`.
- The change was archived at
  `.cairn/archive/2026-11-10-route-installed-native-effects/`; strict Cairn
  validation then passed with 6 active changes and zero findings.
- Final tree source gate: both Octet summaries zero findings/warnings/errors, gate
  `blake3:fbab34928b04b8ba3f3802bb58244cb9317b45af808730232198518b7adfc7dc`,
  output `/nix/store/v18y81cm9x1265ssmkjwnq79jaya8a2s-molten-node-source-binding-v1`
  (binding `blake3:6b86bb2507380e214e305f30126e33d24b8c0d3569799f1bbd9915b9be9b983b`),
  signed only after admission. Its binaries are byte-identical to the executed
  product (`molten` `blake3:e628695d…`, `molten-collaboration-scope`
  `blake3:8a6ae649…`).
- `cairn readiness route-installed-native-effects` remains
  `classification: unavailable` because its completion stage needs an externally
  supplied observation contract/results; the change workflow itself is valid with no
  next actions. No completion observation was fabricated, and readiness is not
  claimed.
- Repository rails: the hermetic `checks.x86_64-linux.nextest` passed with receipt
  `blake3:262af0210c5d306a44f70974c04d1511aea49aa0b7e273743b3296d136e87fe9`
  (`decision=pass`). `checks.x86_64-linux.nixos-vm-multinode` failed because
  `molten-node.service` exited with
  `invalid harness artifact: node state file source-gate-trust.json does not exist`.
  That requirement was introduced by `ec1f3d7f4` (an ancestor of this change's base)
  and the VM service module has never provisioned the trust file, so the failure is
  pre-existing and outside this change; it is recorded, not fixed. A full
  `nix flake check` additionally stalls while evaluating
  `devShells.x86_64-linux.default` in this environment (Nix input fetch/eval lock),
  so the aggregate rail is not claimed.
