# Clock domains and conversion

Molten represents time as admitted, explicitly supplied data rather than a universal environmental fact. This article explains the domain boundaries and conversion mechanics behind the [fabric-time runtime](../../fabric-time-scheduler-runtime.md). Familiarity with checked arithmetic and profile admission is assumed. It is a [Technical companion](../README.md), not a replacement for the governing runtime description.

## A coordinate includes its interpretation

The four variants of `TimeValue` are not interchangeable timestamps. `WallClockObservation` carries `unix_nanos`, uncertainty, and an observation sequence; `MonotonicInstant` carries process-local ticks; `LogicalEventTime` carries an ordering position; and `VirtualInstant` carries simulation-controlled ticks. Each includes a `profile_ref`. Although `TimeValue::ticks` exposes their numeric coordinate through one accessor, that accessor does not erase the domain distinction in the checked operations ([domain implementation](../../../crates/molten-core/src/fabric_time/domain.rs)).

This distinction prevents a common error: interpreting equality of integers as equality of events or durations. Logical position 100 and monotonic tick 100 need not describe related moments. Two monotonic observations from different profiles also fail the profile check. Even matching profile references do not establish synchronized origins across independent processes; the live adapter anchors an origin using a host `Instant` when constructed ([live adapter](../../../src/fabric_time/parts/adapters/p000/body.rs)).

`CheckedDuration` includes both profile and domain. `compare_time_values` validates both operands before comparing coordinates. Addition and subtraction validate the instant and duration, require equal domains, and use checked integer operations. A profile's duration limit applies independently of representability: a representable duration can still exceed the admitted bound. These are deterministic admission decisions, not measurements of whether a machine can wait that long.

## Conversion is explicit evidence-bearing arithmetic

`ExplicitTimeConversion` names source and target profiles and domains, a signed offset, uncertainty, a target wall-observation sequence, and a conversion-evidence reference. `convert_time_value` validates the source, checks those bindings, verifies the reference's accepted shape, and checks uncertainty against the target profile. It then adds the offset in `i128` and converts the result to `u64` before constructing the target variant ([conversion implementation](../../../crates/molten-core/src/fabric_time/domain.rs)).

The evidence reference is an input, not a calibration procedure. This function does not contact a clock service, establish physical simultaneity, or authenticate the proposition that the supplied offset is correct. Nor does it infer a rate conversion. An offset mapping between coordinates is all that this arithmetic implements.

There is also an important representational limit: conversion uncertainty is stored in a resulting wall observation, but the monotonic, logical, and virtual value structures have no uncertainty field. Passing conversion validation therefore does not mean uncertainty has become an enduring property of every target value. A subsequent deadline carries its own explicit uncertainty. Callers reviewing an end-to-end temporal argument must follow that information separately rather than assuming automatic propagation.

## Worked coordinate example

Consider an illustrative source virtual value at tick 120 under profile A. An explicit conversion targets logical time under profile B with offset −20, uncertainty 3, and a valid conversion-evidence reference. Assuming both profiles admit the relevant domains and B permits that uncertainty, the constructed logical position is 100.

Three superficially similar operations differ:

1. Direct comparison of the source virtual value and target logical value is rejected rather than treating 120 as later than 100.
2. Adding a profile-A virtual duration to the profile-B logical result is rejected, even if its numeric length is small.
3. Changing the conversion offset to −121 yields an unrepresentable negative target and fails rather than wrapping to a large unsigned value.

The third case should be reviewed as a representability failure, not as a statement about historical time. In the inspected implementation, failed `u64::try_from` conversion is mapped to `Underflow`; that diagnostic label also covers a positive result above `u64::MAX`. Consumers should not infer the sign of a failed conversion solely from that error variant.

## Observation order is not wall-clock order

`classify_wall_clock_observation` requires equal profiles and a strictly increasing observation sequence. Within that sequence, excessive uncertainty takes precedence over backward- or forward-jump classification. A later observation may contain an earlier UTC coordinate. “Later observation” thus refers to collection order, not necessarily increasing wall time.

The live shell reads wall and monotonic clocks separately. A wall read can fail if the host clock predates the Unix epoch, and a monotonic read checks against the adapter's previous tick value. These shell checks supply observations; they do not change the pure domain laws. Wall movement is not itself a lease or authority oracle, as the [governing document](../../fabric-time-scheduler-runtime.md#time-domains) emphasizes.

## Verification and limits

For review, trace the profile reference, domain, integer range, uncertainty, and evidence reference independently through every conversion. Suggested boundary exercises include mismatched profiles, unsupported target domains, negative offsets crossing zero, positive offsets exceeding the unsigned range, and non-increasing wall observation sequences. These are proposed checks, not commands executed for this article.

Runtime budget admission is a separate concern: [runtime limit profiles](../../runtime-limit-profiles.md) select effective budgets under hard caps and do not confer temporal authority. Canonical Preserves values and BLAKE3 references identify artifacts; neither a timestamp nor a successful conversion proves global time, synchronized clocks, remote deadline agreement, or production readiness.

## Sources

- [Fabric time, scheduler, and entropy runtime](../../fabric-time-scheduler-runtime.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Time domains and explicit conversion](../../../crates/molten-core/src/fabric_time/domain.rs)
- [Live clock adapter and local deadlines](../../../src/fabric_time/parts/adapters/p000/body.rs)
- [Technical companion](../README.md)
