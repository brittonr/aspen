# Deadline, retry, and lease limits

Deadlines, retry plans, and lease observations answer related but different questions: whether a local target has been reached, when another attempt should be planned, and whether supplied lease conditions permit a local decision. This [Technical companion](../README.md) assumes the domain distinctions in the [fabric-time runtime](../../fabric-time-scheduler-runtime.md). Its purpose is to make the arithmetic and authority limits explicit, not to infer application safety from timing machinery.

## A deadline is a bounded decision problem

`Deadline` carries profile, subject, generation, target `TimeValue`, and explicit uncertainty. `evaluate_deadline` validates that identity and generation, rejects a wall-clock target, validates both target and observation, and requires their domains to agree. It then classifies the observation relative to the target ([deadline and lease implementation](../../../crates/molten-core/src/fabric_time/lease.rs)).

For observed coordinate N and uncertainty U, the implementation uses saturating arithmetic to form an earliest possible observation `N − U` and latest possible observation `N + U`. If the latest value is strictly below the target, the result is pending. If the earliest value is at or above the target, it is expired. Otherwise it is indeterminate within uncertainty. Saturation at integer endpoints avoids wrapping the observation interval; it does not make the uncertainty physically justified.

For an illustrative target of 100 and uncertainty 5, observation 94 is pending, 95 is indeterminate, 104 is still indeterminate, and 105 is expired. With zero uncertainty, observation 100 is expired. These boundary inequalities matter: a caller that rounds “close enough” to expired has changed the decision law.

## Retry planning has finite arithmetic, not effect knowledge

`RetryPolicy` requires nonzero maximum attempts, base delay, and maximum delay, with base no greater than maximum. Bounded jitter requires a positive bound. Attempts are zero-based in the arithmetic and are denied when `attempt >= maximum_attempts`. Fixed backoff uses the base directly; exponential backoff computes a capped base times two to the attempt power.

The exponential helper guards attempts at or beyond `u64::BITS` before shifting. Below that width, it uses checked multiplication and saturates an overflowing product to the maximum delay. This avoids both narrowing the attempt and performing an attempt-sized loop. Jitter is different: a bounded policy requires an explicit supplied value within its inclusive maximum; a no-jitter policy rejects even a supplied zero. Checked base-plus-jitter addition happens before the maximum-delay cap, and adding the resulting duration to the current instant is also checked ([retry implementation](../../../crates/molten-core/src/fabric_time/lease.rs)).

For an illustrative policy with base 3, maximum delay 20, and enough admitted attempts, exponential attempts 0, 1, 2, and 3 produce unjittered delays 3, 6, 12, and 20. A much larger admitted attempt remains capped at 20. This saturation does not authorize an attempt beyond the finite attempt limit. Nor does it rescue jitter-addition overflow or a target beyond the coordinate range.

The planner receives jitter ticks; it does not itself draw entropy or prove the value came from an admitted entropy stream. A caller composing purpose-bound entropy with retries must preserve that separate admission path.

## Construction and evaluation both deny wall-clock deadlines

The [governing contract](../../fabric-time-scheduler-runtime.md#deadlines-retries-and-local-leases) states that wall-clock deadlines are denied. `evaluate_deadline` and lease validation enforce that domain restriction, and `plan_retry` calls the same deadline-domain validator on the current `TimeValue` before it does any delay arithmetic. A wall-clock base therefore fails retry planning with `UnsupportedDeadlineDomain(WallClock)` and no `Deadline` is constructed; a monotonic base yields a deadline in the monotonic domain.

The evaluator keeps its own wall-domain check as defence in depth: a `Deadline` assembled outside `plan_retry` is still rejected when evaluated. Neither check makes UTC movement a deadline authority oracle.

## Lease classification is not distributed exclusivity

`evaluate_lease` validates identifiers, generation, matching admitted domains, non-wall time, and uncertainty. A pending observation maps to locally active for observe and renewal allowed for renew. An expired observation denies renewal and exclusive acquisition. Indeterminate time remains indeterminate before the exclusive fencing check.

For pending exclusive acquisition, `classify_exclusive_lease` requires `LeaseConsistency::FencedExclusive` and a supplied token. If a previous token is supplied, the new token must be strictly greater. These checks operate on request values. The core does not contact remote storage, issue tokens, or authenticate that all participants reject old tokens.

The governing document additionally requires a reviewed fenced consistency profile. The inspected request represents consistency as an enum rather than carrying such a profile reference into `evaluate_lease`. Therefore `ExclusiveActionAllowed` is a local classification under supplied inputs, not independent evidence that the reviewed-profile requirement was satisfied. Even the full admitted contract explicitly declines to prove distributed lease exclusivity, partition absence, or remote clock agreement.

## Verification and historical evidence

Suggested review cases cover uncertainty boundaries, last permitted and first exhausted attempts, exponential width boundaries, jitter overflow before capping, stale generations, and equal versus increasing fencing tokens. These checks are proposed, not executed for this article. [Runtime limit profiles](../../runtime-limit-profiles.md) additionally constrain operational budgets; they cannot prove a retry is idempotent or grant an exclusive action.

The governing runtime notes that corrected exponential arithmetic retains `v1` schema shapes while changing formerly wrapped outputs. Replay consequently requires the exact implementation cohort; historical receipts must not be rewritten to resemble corrected events. A successful retry plan proves neither timer execution nor safe re-execution of an application operation. Finite attempts, local deduplication elsewhere, and a deadline do not together establish exactly-once effects or production readiness.

## Sources

- [Fabric-time deadline, retry, lease, and replay contract](../../fabric-time-scheduler-runtime.md)
- [Runtime limit profiles](../../runtime-limit-profiles.md)
- [Deadline evaluation, retry arithmetic, and lease classification](../../../crates/molten-core/src/fabric_time/lease.rs)
- [Checked domain arithmetic](../../../crates/molten-core/src/fabric_time/domain.rs)
- [Technical companion](../README.md)
