# Fabric port ownership

<!-- r[impl molten.modularity.fabric_boundary.inventory] -->
<!-- r[impl molten.modularity.fabric_boundary.ownership] -->
<!-- r[impl molten.modularity.fabric_boundary.docs] -->

## Result

Molten keeps selected fabric capability contracts outside adapter modules.
Pure decisions remain in `molten-core` or canonical core modules.
Application shells own effect order and uncertainty.
Adapters contain host mechanisms only.

## Inventory

<!-- r[impl molten.modularity.fabric_boundary.ownership.classified] -->

| Family | Capability | Owner | Inputs | Outputs | Typed failures | Effects | Composition root |
|---|---|---|---|---|---|---|---|
| Membership | Snapshot observation | `fabric_membership/ports.rs` | Source profile and provider state | `MembershipProviderSnapshot` | unavailable or malformed observation | Reads a static, managed, or scripted provider | Membership bootstrap and simulation setup |
| Membership | Assignment persistence | `fabric_membership/ports.rs` | Current assignment and pure transition | Intent and commit refs | storage failure or uncertain outcome | Persists intent before role effects, then commits | Membership shell setup |
| Membership | Role lifecycle | `fabric_membership/ports.rs` | Admitted assignment | Role-effect ref | definite or uncertain role failure | Activates, drains, replaces, releases, fails, or quarantines a role | System-extension lifecycle setup |
| Time | Clock observation and wait | `fabric_time/ports.rs` | Admitted profile and target tick | Explicit tick observation | unavailable, malformed, or timeout | Reads a live clock or advances a virtual clock | Runtime and simulation time setup |
| Time | Cryptographic entropy | `fabric_time/ports.rs` | Bounded output buffer | Filled secret buffer | unavailable capability | Reads the operating-system entropy source | Runtime entropy setup |
| Transport | Command execution | `fabric_transport/ports.rs` | Pure admitted transport command | Canonical transition | transport failure or uncertain outcome | Runs deterministic or Iroh transport mechanisms | Registered transport effect-port setup |
| Durability | Command execution | `fabric_durability/ports.rs` | Pure admitted durable command | Canonical transition | storage failure or uncertain outcome | Runs Redb or deterministic durability mechanisms | Registered durable effect-port setup |

Pure profile checks, transitions, authority checks, command admission, and failure classification do not need ports.
They remain ordinary functions over explicit values.

## Required order

The membership shell validates authority before any protected effect.
It computes the transition, persists intent, runs the admitted role effect, and commits the outcome.
A denial does not persist intent or call the role lifecycle port.
A failure after an external effect becomes an uncertain outcome.
It does not become a policy denial.

Time, transport, and durability cores receive explicit observations or commands.
They do not construct live adapters.
Runtime and system-extension setup select concrete implementations.

## Compatibility baseline

Before migration, focused tests passed for three membership cases, 16 time cases, 18 transport cases, and seven durability cases.
The migration keeps the existing canonical Preserves values, BLAKE3 transition refs, receipt meanings, and adapter-neutral behavior.
The source audit has positive repository fixtures and negative fixtures for every prohibited dependency shape.

## Enforcement

The repository-owned fabric boundary audit rejects:

- port trait definitions in maintained `adapters.rs` modules;
- raw `String` failures in maintained `ports.rs` modules;
- host effects in declared core paths;
- adapter-owned policy functions;
- concrete live adapter construction in declared core paths.

The audit is a source-ownership check.
It does not prove type correctness, external behavior, or release eligibility.

## Active-change dependencies

This change preserves the contracts used by active consistency, transport, durability, simulation, and system-extension changes.
It does not complete those changes or transfer their authority.
The product-owned live register adapter lives in `src/reliability/`; its pure profile, generator,
history projection, fault admission, and evidence import do not move service semantics into generic fabric adapters.
`molten reliability generate --profile PROFILE.json --plan PLAN.json --metadata METADATA.json` emits the bounded
product plan and its canonical profile/plan references; identities are measured, not guessed by an external JSON hasher.
`molten reliability adapter --request REQUEST.json --response RESPONSE.json --authority-root ROOT --client-key KEY.json`
performs one authenticated public operation using explicitly provisioned controller authority and client key.
The request schema is `onixos.live-client.v1`, with `phase` (`setup`, `invoke`, `recover`, `final-read`,
or `teardown`), `run_id`, numeric socket `endpoint`, optional `operation`, and bounded `deadline_ms`.
An invocation operation carries `id`, `process`, `function` (`read` or `write`), `object_key`, and a tagged
integer/null `value` for writes. The endpoint protocol is one bounded newline-delimited JSON request and response
per TCP connection. The response repeats schema/run/endpoint and logical `operation_id` (plus supplied `attempt_id`),
and carries `outcome`, `value`, and optional diagnostic. The request embeds the exact product profile.
Only an explicit pre-effect denial may be `fail`; lost acknowledgements, malformed responses, identity mismatches,
timeouts, and disconnects remain `info`. The adapter never retries writes implicitly.

The actual service must be assembled from an admitted active group, exact seven-port bindings, capability-rooted
durability, time/entropy, and provisioned peer transport sessions. A local native callback host does not supply
this network coordination service. Possessing matching reference strings cannot replace provisioning or authority.
The disposable profile remains non-production. A native-service readiness receipt must not be issued before the
real public endpoints and their admission chain exist.

The experimental coordination issuer is separate from startup source trust and the native local-host grant.
`molten reliability provision --state-root ROOT --operator-key OPERATOR.json --policy POLICY.json --grant GRANT.json
--probe-key PROBE.json` installs a real signed operator policy and grant only into an unprovisioned protected root.
The operator explicitly supplies independently selected keys as JSON arrays of 32 bytes; there is no daemon
self-trust or implicit production issuer. Policy limits nodes, groups, source cohorts, operations and lifetime.
The grant signs exact membership, seven port/profile bindings, namespace, timer bounds, peer/session identities,
configuration, generation, client public key and shared client scope, plus a distinct purpose-bound probe public key.
Protected policy/grant/currentness is revalidated before resource effects. Server probe keys are not controller client keys.
Controller authority roots contain the independently pinned operator public key and signed policy/grant;
they need not contain server probe secrets. Client invocation signatures bind the unchanged request, scope, and nonce;
the host consumes nonce replay markers durably before effects. Forwarding retains the original signed request.

For the named three-node experimental configuration recipe, run
`molten reliability experimental-config --input RECIPE.json --out COHORT.json`.
The input explicitly supplies actual group/workload/registry definitions, the installed seven-port binding corpus,
operator scope, per-node signed source/capability references, independent keys, nonzero sockets, and storage locators.
It emits three serializable service configurations, unsigned policy/grants for separate operator signing, and
independently derived `readiness_admission` expectations; generating the recipe grants no effect authority.
`molten reliability mechanism-profile --input MECHANISMS.json --out PROFILES.json` exports only the admitted
transport/protocol/time/durable namespace mechanism bundle. Neither command creates implicit trust.
After explicit policy review and provisioning, the actual service lifecycle is
`molten reliability service init|serve|status|stop --state-root ROOT`.
Serve runs actual Iroh peer transport, rooted Redb recovery, admitted timers, the scoped live replica, and the
bounded public socket. The endpoint descriptor is published as `ROOT/coordination-endpoint.json` after bind.
The native node unit remains the separate current source/startup prerequisite; process faults target the real
coordination unit, not only the local callback host.

`molten reliability verify-readiness --state-root ROOT --node-id ID --source-binding-ref REF
--startup-receipt ROOT/startup-receipt.preserves --health-receipt ROOT/onixos-health.preserves --endpoint-proof SELECTOR.json`
validates current signed source binding, canonical startup/health/current-run lock, then sends a fresh random nonce
to the actual public endpoint. The separate `probe` phase returns a dedicated-key signed current-state observation,
not a caller-marked success. The verifier checks purpose, endpoint/group/session/generation/source identities,
actual ready state, and rechecks both protected grant and current run after the response. The endpoint selector
(`molten.coordination-endpoint.v1`: node_id, endpoint, run_id, profile) supplies no trust and must match signed config.
Only experimental readiness can result; `production_admitted` remains false.
The verifier persists `ROOT/onixos-readiness-challenge.json` before sending the nonce, and returns the full
public `signed_probe` alongside its canonical `probe_ref`. Offline intake requires a readiness map for every endpoint:
the independently admitted probe key/state from the recipe, separately retained pre-send Challenge, and the complete
actual verifier output. It never builds expected nonce/key/state merely by echoing the returned proof.

ChaosControl semantic-history v2 uses tagged values (`{"type":"integer","value":7}` or `{"type":"null"}`),
one controller's event ordering and monotonic time, and the actual retained source artifact's `blake3:` identity.
The checker shell invokes `semantic-history-contract check HISTORY REPORT`, followed by
`semantic-history-contract validate HISTORY REPORT`; a zero exit does not imply a valid verdict.
Pending histories, exhausted bounds, absent post-heal endpoint reads, checker disagreement, or incomplete teardown
cannot become passing live evidence. Every endpoint must supply a final public read after readiness and stable membership.
Simulation, NixOS VM, ChaosControl KVM, and OnixOS live receipts remain distinct evidence classes.
Clock, durability, Byzantine, locks, queues, transactions, production readiness, and release eligibility remain unsupported.

`molten reliability check --checker CHECKER.json --history HISTORY.json --work-root EXISTING_DIR --report REPORT.json`
executes both checker operations. The checker configuration names its absolute executable path, measured
`executable_blake3`, immutable `checker_ref`, `timeout_ms`, and `max_bytes`. Matching supplied refs are not an
alternative to hashing and executing the binary. The dedicated work directory is opened once as a capability.
`molten reliability import --expected EXPECTED.json --evidence EVIDENCE.json --checker CHECKER.json
--work-root EXISTING_DIR --ledger LEDGER --receipt RECEIPT.preserves` recomputes the report before import;
an optional `--reference-checker CONFIG.json` must likewise execute to support a supplied reference report.
The expected input contains the original product profile and operation plan, exact identity map/endpoints,
and finite event, witness, and recovery bounds. A caller's unexecuted checker report is insufficient.
Import stores a canonical external-evidence receipt and prints bounded operator status; it grants no authority.

Each final recovery row binds `endpoint`, `attempt_id`, tagged `value`, `ready`, `ready_event`,
`stable_membership_event`, `stable_membership_observations`, `stable_membership`, and `elapsed_ms`.
The referenced `ready` and `stable-membership` lifecycle event indices must precede that endpoint's planned
final read. Elapsed time derives from controller monotonic event time rather than a supplied success marker.
Fault evidence separately binds selected/applied/observed event indices and healed/recovered lifecycle indices.
The terminal history event must be a `teardown` lifecycle observation as well as a completed teardown fact.
`molten reliability compare --profiles PROFILES.json --report REPORT.json` retains environment, adapter,
fault mechanism, time source, checker, finite verdict, and non-claims for every profile without merging them.

The OnixOS disposable runner is `nix/live-reliability.nix` in the immutable provider source. Integration supplies
the exact signed raw Molten producer (not a relocated wrapper), native helper, checker, profile, node authority modules,
coordination settings, independent source-trust files, and controller-authority module. The node authority unit is
`molten-coordination-authority.service`; controller provisioning is `molten-client-authority.service`.
Once the parent exports the integrated check, the concrete campaign command is
`nix build .#checks.x86_64-linux.molten-live-reliability -L`.
Its retained `result/live-reliability/` includes operation plans and product metadata, control/fault histories and
checker reports, BLAKE3-bound actual fault effects, startup/health/challenge/full signed readiness artifacts,
recovery observations, teardown, product intake/expectations, and the canonical imported receipt.
The provider offline verifier is `python3 "$ONIX_SOURCE/tools/live-reliability/verify.py" result/live-reliability
--checker "$CHECKER_PACKAGE/bin/semantic-history-contract" --b3sum "$B3SUM_PACKAGE/bin/b3sum"`.
These commands describe the integration contract; they are not claims that an unexecuted campaign passed.

## Non-claims

<!-- r[impl molten.modularity.fabric_boundary.final_checks] -->
<!-- r[impl molten.modularity.fabric_boundary.final_checks.claims] -->

This boundary does not prove live transport correctness, durable storage, clock accuracy, entropy quality, authority correctness, simulation parity, or release readiness.
A canonical receipt records bounded supplied facts only.
