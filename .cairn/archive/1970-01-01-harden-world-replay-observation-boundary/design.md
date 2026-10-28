## Context

The active replay-capsule change binds inputs and expected world commits. This change closes the remaining environment boundary.

RR records nondeterministic inputs at the user and kernel boundary, then supplies recorded results during replay. Molten adapts that principle at its product-owned semantic boundary. It does not adopt RR process internals as world semantics.

## Decisions

### Decision: Inventory every nondeterministic source

**Choice:** Each replay profile contains a closed inventory of hostcalls, effect ports, scheduler decisions, asynchronous event classes, clocks, entropy streams, external reads, and runtime observations.

Each source has one handling class: `deterministic`, `simulated`, `recorded-observation`, or `unsupported`. Unknown sources deny recording and replay.

**Rationale:** Replay completeness is impossible when a source can cross the boundary without a declared treatment.

### Decision: Use the semantic host boundary

**Choice:** Molten records observations at Wasm hostcalls and typed effect ports. Native syscalls inside an admitted runtime are outside logical replay unless the selected opaque profile captures them separately.

**Rationale:** Hostcalls carry product meaning. Syscalls expose lower-level implementation details and cannot define complete world transitions.

### Decision: Bind observations to requests and logical order

**Choice:** Every recorded observation binds source ID, operation or request ID, transition step, logical position, canonical request identity, result or error class, protection profile, adapter cohort, and BLAKE3 identity.

Schedule choices and asynchronous deliveries use explicit logical ordering. Wall-clock arrival is an observation, not hidden authority over order.

**Rationale:** Correct values in the wrong order can produce a different world while final summaries still appear similar.

### Decision: Replay observations without repeating effects

**Choice:** During replay, recorded-observation adapters return sealed results. They do not call the original external service or perform the original write, send, publish, or release effect.

Effect completion remains represented by recorded effect evidence. Current effect release still requires the existing reservation and promotion protocol.

**Rationale:** Repeating an external effect changes the environment and can duplicate irreversible work.

### Decision: Make completeness a pure admission result

**Choice:** The core validates inventory closure, expected observation counts, source classes, ordering, request binding, profile identity, and protection metadata. It emits either a replay plan or exact blockers.

The shell provides trace bytes and runtime observations. It cannot waive a missing or unsupported source.

**Rationale:** Completeness and ordering are deterministic policy decisions.

### Decision: Keep opaque native replay detached

**Choice:** A ChaosControl native-process replay descriptor can attach as an opaque diagnostic member under an exact cohort. It cannot satisfy missing semantic hostcall observations and cannot become a logical world root.

**Rationale:** Native forensic replay and semantic world replay answer different questions.

### Decision: Treat storage optimization as non-identity

**Choice:** Trace members use canonical content identities and lengths. The shell may use reflinks or deduplication only after source identity, immutability, destination readback, and cohort policy pass.

Paths, inode numbers, hard links, and reflinks never substitute for byte identity.

**Rationale:** Storage sharing is an optimization, not integrity evidence.

## Verification strategy

Positive tests cover deterministic sources, simulated clocks and entropy, recorded external reads, ordered asynchronous delivery, sealed effect observations, stable repeated replay, and detached opaque diagnostics.

Negative tests cover unknown hostcalls, omitted inventory rows, missing results, wrong request bindings, reordered and duplicate events, extra observations, unsupported sources, profile drift, adapter drift, plaintext secrets, repeated effects, tampered trace members, and opaque-as-semantic overclaims.

## Rollout

1. Add inventory and observation schemas without changing runtime behavior.
2. Gate one logical replay profile on complete source coverage.
3. Convert existing clock, entropy, scheduler, and effect-log paths.
4. Add external-read and asynchronous-event fixtures.
5. Admit optional ChaosControl opaque diagnostics only after its profile contract is published.

## Claim boundary

Passing replay proves that the selected Molten adapters reproduced the recorded semantic observations and expected world transitions for one exact profile. It does not prove arbitrary process determinism, kernel replay, external effect completion, host security, or release eligibility.
