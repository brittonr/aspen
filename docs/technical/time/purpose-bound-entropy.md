# Purpose-bound entropy

Entropy in Molten is an admitted stream with an explicit purpose, capability reference, lifecycle generation, replay class, and consumption position. This [Technical companion](../README.md) examines the separation between deterministic input, production secret input, and secret-free evidence. It assumes the [fabric-time runtime](../../fabric-time-scheduler-runtime.md) contract; it does not propose a random-number API or a cryptographic security proof.

## Binding a stream is not granting a capability

`EntropyStreamRequest` names its profile, stream identifier, purpose, capability reference, generation, mode, and optional simulation seed and seed reference. `open_entropy_stream` checks profile equality, mode compatibility with the profile kind, identifier shapes, capability-reference shape, nonzero generation, and current generation. A deterministic-simulation profile selects deterministic mode; a live profile selects production-cryptographic mode ([entropy core](../../../crates/molten-core/src/fabric_time/entropy.rs)).

The word “capability” needs careful interpretation at this boundary. A well-formed reference is not proof that the caller possesses the referenced authority. `ExtensionTimeContext::open_entropy_stream` separately requires the reference to occur in the host snapshot's admitted capability list and requires an admitted entropy port profile. The shell's request admission also checks the extension byte envelope ([extension shell](../../../src/fabric_time/shell.rs)). Purpose names describe the stream's admitted use; the byte generator does not establish application-level authorization from those names alone.

A stream state retains those bindings and a `position_bytes` counter. Requests are either nonempty byte draws or bounded choices with a nonzero upper bound. Every bounded choice charges eight bytes regardless of the size of its range. The core checks the per-request limit, checked position addition, total stream limit, and generation before returning a transition. These profile bounds are distinct from the host's envelope and from the general operator budgets described in [runtime limit profiles](../../runtime-limit-profiles.md).

## Deterministic position is independent of chunking

Simulation requires both an explicit `u64` seed and a valid deterministic-input reference. The deterministic generator uses seed and absolute byte position to locate eight-byte blocks, then copies the requested portion. Stream identity and purpose are retained metadata; the inspected generator does not mix them into the seed. Giving different purposes the same seed and position therefore does not create cryptographically separated byte sequences.

An illustrative replay consumes twelve bytes in one run and consumes five followed by seven in another. Provided profile limits admit both request patterns and the second draw starts from the first transition's next state, the concatenated bytes match the twelve-byte draw. The split crosses a block boundary, but the generator derives each block from absolute position rather than advancing a hidden host RNG per request. Existing tests cover chunk invariance and exhaustion ([entropy tests](../../../crates/molten-core/src/fabric_time/tests.rs)).

This property concerns output bytes, not identical evidence histories. The two-request run has two consumption intervals and two transitions, whereas the single-request run has one. Reproducible input does not imply that differently partitioned event traces have the same canonical reference.

## Production input crosses a shell boundary

Production admission forbids both deterministic seed fields. `consume_production_entropy` accepts externally supplied secret bytes, verifies their exact requested count, and advances the same position accounting. The pure core neither opens a device nor judges the cryptographic quality of supplied bytes.

The Unix `OperatingSystemEntropySource` opens `/dev/urandom` and uses `read_exact`; failures propagate. The non-Unix branch reports that no admitted source is available. There is no deterministic fallback in this adapter ([production adapter](../../../src/fabric_time/parts/adapters/p002/body.rs)). This is a specific implementation path, not a claim about every possible `CryptographicEntropySource` implementation.

Ordering matters when assessing resource behavior: `ProductionEntropyAdapter::draw` allocates and fills the requested buffer before invoking core consumption validation. Consequently, core rejection does not itself prove that no shell allocation or entropy read happened. The extension request-admission boundary is relevant to an end-to-end review; it should not be mentally replaced by the later pure validation step.

## Evidence deliberately omits secret material

`entropy_evidence_metadata` records profile, stream, purpose, generation, mode, replay class, deterministic-input reference, and start/request/end positions. `canonical_entropy_event` rejects incompatible mode/replay combinations, requires a deterministic-input reference for simulation, and forbids one for production. It serializes consumption metadata without output bytes or raw seed ([canonical entropy event](../../../src/fabric_time/parts/canonical/p001/body.rs)). Production replay thus needs separately authorized secret input rather than a receipt containing that input.

There is a scoped source discrepancy: the [governing prose](../../fabric-time-scheduler-runtime.md#entropy) describes capability-bound stream metadata in evidence, but the inspected `EntropyEvidenceMetadata` and canonical event do not contain `capability_ref`. The request and state carry it, and the extension shell checks it; this article does not claim that the entropy event alone binds or reconstructs that capability reference. Resolving that evidence relationship requires a broader admitted context, not an invented event field.

## Verification and non-claims

Suggested checks include split draws across eight-byte boundaries, exhaustion by one byte, stale generations, production seeds, wrong supplied byte counts, and absent secret replay input. Compare metadata separately from secret output. These checks are guidance, not reported execution.

`BoundedChoice` uses the high half of a widened sample-times-bound product. The implementation supplies a bounded value; this article makes no exact-uniformity claim for arbitrary bounds. Simulation is not cryptography, purpose binding is not cryptographic domain separation, and a secret-free event is not proof of secret-source quality, application retry safety, or production readiness.

## Sources

- [Fabric-time entropy contract](../../fabric-time-scheduler-runtime.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Entropy admission, stream generation, and metadata](../../../crates/molten-core/src/fabric_time/entropy.rs)
- [System-extension capability and byte-envelope checks](../../../src/fabric_time/shell.rs)
- [Production entropy shell](../../../src/fabric_time/parts/adapters/p002/body.rs)
- [Canonical entropy event](../../../src/fabric_time/parts/canonical/p001/body.rs)
- [Entropy regression tests](../../../crates/molten-core/src/fabric_time/tests.rs)
- [Technical companion](../README.md)
