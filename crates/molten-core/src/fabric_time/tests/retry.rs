use super::ACTIVE_GENERATION;
use super::DEADLINE_TICKS;
use super::DeadlineLeaseError;
use super::RETRY_BASE;
use super::RETRY_JITTER;
use super::RETRY_MAX;
use super::RetryBackoff;
use super::RetryJitter;
use super::RetryPlan;
use super::RetryPolicy;
use super::STALE_GENERATION;
use super::TimeArithmeticError;
use super::TimeDomain;
use super::plan_retry;
use super::profile;
use super::virtual_time;

const AUDIT_BASE_DELAY_TICKS: u64 = 2;
const AUDIT_ATTEMPT: u64 = 63;
const AUDIT_MAXIMUM_DELAY_TICKS: u64 = 128;
const ORDINARY_ATTEMPT: u64 = 3;
const ORDINARY_DELAY_TICKS: u64 = 16;
const SUBJECT_ID: &str = "retry.audit-f12";

fn exponential_policy() -> RetryPolicy {
    RetryPolicy {
        maximum_attempts: u64::MAX,
        base_delay_ticks: AUDIT_BASE_DELAY_TICKS,
        maximum_delay_ticks: AUDIT_MAXIMUM_DELAY_TICKS,
        backoff: RetryBackoff::Exponential,
        jitter: RetryJitter::None,
    }
}

fn retry(attempt: u64, policy: RetryPolicy) -> Result<RetryPlan, DeadlineLeaseError> {
    plan_retry(
        &profile(),
        ACTIVE_GENERATION,
        SUBJECT_ID,
        ACTIVE_GENERATION,
        &virtual_time(0),
        attempt,
        policy,
        None,
    )
}

// r[verify molten.audit_f12.saturation]
#[test]
fn retry_exponential_preserves_ordinary_growth_and_caps_the_audit_case() {
    let policy = exponential_policy();
    for (attempt, expected) in [
        (0, AUDIT_BASE_DELAY_TICKS),
        (ORDINARY_ATTEMPT, ORDINARY_DELAY_TICKS),
        (AUDIT_ATTEMPT, AUDIT_MAXIMUM_DELAY_TICKS),
    ] {
        let plan = retry(attempt, policy).expect("admitted exponential retry");
        assert_eq!(plan.delay.ticks, expected, "attempt {attempt}");
        assert_eq!(plan.deadline.target.ticks(), expected);
    }
}

// r[verify molten.audit_f12.saturation]
// r[verify molten.audit_f12.bounds]
#[test]
fn retry_exponential_matches_a_wide_integer_reference() {
    for base_delay_ticks in 1..=AUDIT_MAXIMUM_DELAY_TICKS {
        let policy = RetryPolicy {
            base_delay_ticks,
            ..exponential_policy()
        };
        for attempt in 0..=u64::BITS {
            let multiplier = 1_u128.checked_shl(attempt).expect("bounded reference shift");
            let product = u128::from(base_delay_ticks).checked_mul(multiplier).expect("bounded reference product");
            let capped = product.min(u128::from(policy.maximum_delay_ticks));
            let expected = u64::try_from(capped).expect("capped reference fits the delay type");
            let plan = retry(u64::from(attempt), policy).expect("admitted boundary retry");
            assert_eq!(plan.delay.ticks, expected, "base {base_delay_ticks}, attempt {attempt}");
            assert_eq!(plan.deadline.target.ticks(), expected);
        }
    }
}

// r[verify molten.audit_f12.bounds]
#[test]
fn retry_large_admitted_attempts_saturate_without_narrowing() {
    for attempt in [u64::from(u64::BITS), u64::from(u32::MAX) + 1, u64::MAX - 1] {
        let plan = retry(attempt, exponential_policy()).expect("large admitted retry");
        assert_eq!(plan.delay.ticks, AUDIT_MAXIMUM_DELAY_TICKS);
        assert_eq!(plan.deadline.target.ticks(), AUDIT_MAXIMUM_DELAY_TICKS);
    }
}

// r[verify molten.audit_f12.bounds]
#[test]
fn retry_exhaustion_precedes_exponential_calculation() {
    assert_eq!(
        retry(u64::MAX, exponential_policy()),
        Err(DeadlineLeaseError::RetryExhausted {
            attempt: u64::MAX,
            maximum: u64::MAX,
        })
    );
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_fixed_delay_remains_exact_for_large_attempts() {
    let policy = RetryPolicy {
        base_delay_ticks: RETRY_BASE,
        maximum_delay_ticks: RETRY_MAX,
        backoff: RetryBackoff::Fixed,
        ..exponential_policy()
    };
    for attempt in [0, ORDINARY_ATTEMPT, u64::MAX - 1] {
        let now = virtual_time(DEADLINE_TICKS);
        let plan =
            plan_retry(&profile(), ACTIVE_GENERATION, SUBJECT_ID, ACTIVE_GENERATION, &now, attempt, policy, None)
                .expect("admitted fixed retry");
        assert_eq!(plan.delay.ticks, RETRY_BASE);
        assert_eq!(plan.deadline.target.ticks(), DEADLINE_TICKS + RETRY_BASE);
        assert_eq!(now, virtual_time(DEADLINE_TICKS));
    }
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_rejects_invalid_policies_before_calculation() {
    let valid = exponential_policy();
    let invalid_policies = [
        RetryPolicy {
            maximum_attempts: 0,
            ..valid
        },
        RetryPolicy {
            base_delay_ticks: 0,
            ..valid
        },
        RetryPolicy {
            maximum_delay_ticks: 0,
            ..valid
        },
        RetryPolicy {
            base_delay_ticks: AUDIT_MAXIMUM_DELAY_TICKS + 1,
            ..valid
        },
        RetryPolicy {
            jitter: RetryJitter::Bounded { maximum_ticks: 0 },
            ..valid
        },
    ];
    for policy in invalid_policies {
        assert_eq!(retry(AUDIT_ATTEMPT, policy), Err(DeadlineLeaseError::InvalidRetryPolicy));
    }
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_rejects_invalid_jitter_without_changing_inputs() {
    let profile = profile();
    let now = virtual_time(DEADLINE_TICKS);
    let policy = RetryPolicy {
        jitter: RetryJitter::Bounded {
            maximum_ticks: RETRY_JITTER,
        },
        ..exponential_policy()
    };
    for (jitter, expected) in [
        (None, DeadlineLeaseError::JitterRequired),
        (Some(RETRY_JITTER + 1), DeadlineLeaseError::JitterOutOfBounds {
            actual: RETRY_JITTER + 1,
            maximum: RETRY_JITTER,
        }),
    ] {
        assert_eq!(
            plan_retry(&profile, ACTIVE_GENERATION, SUBJECT_ID, ACTIVE_GENERATION, &now, 0, policy, jitter),
            Err(expected)
        );
    }
    assert_eq!(
        plan_retry(&profile, ACTIVE_GENERATION, SUBJECT_ID, ACTIVE_GENERATION, &now, 0, exponential_policy(), Some(0)),
        Err(DeadlineLeaseError::JitterOutOfBounds { actual: 0, maximum: 0 })
    );
    assert_eq!(now, virtual_time(DEADLINE_TICKS));
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_caps_valid_jitter_after_saturation() {
    let policy = RetryPolicy {
        jitter: RetryJitter::Bounded {
            maximum_ticks: RETRY_JITTER,
        },
        ..exponential_policy()
    };
    let plan = plan_retry(
        &profile(),
        ACTIVE_GENERATION,
        SUBJECT_ID,
        ACTIVE_GENERATION,
        &virtual_time(DEADLINE_TICKS),
        AUDIT_ATTEMPT,
        policy,
        Some(RETRY_JITTER),
    )
    .expect("admitted jitter after saturation");
    assert_eq!(plan.delay.ticks, AUDIT_MAXIMUM_DELAY_TICKS);
    assert_eq!(plan.deadline.target.ticks(), DEADLINE_TICKS + AUDIT_MAXIMUM_DELAY_TICKS);
    assert_eq!(plan.jitter_ticks, RETRY_JITTER);
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_rejects_jitter_addition_overflow_before_the_cap() {
    let policy = RetryPolicy {
        jitter: RetryJitter::Bounded {
            maximum_ticks: u64::MAX,
        },
        ..exponential_policy()
    };
    let now = virtual_time(DEADLINE_TICKS);
    assert_eq!(
        plan_retry(
            &profile(),
            ACTIVE_GENERATION,
            SUBJECT_ID,
            ACTIVE_GENERATION,
            &now,
            AUDIT_ATTEMPT,
            policy,
            Some(u64::MAX)
        ),
        Err(DeadlineLeaseError::Overflow)
    );
    assert_eq!(now, virtual_time(DEADLINE_TICKS));
}

// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_rejects_stale_generation_and_wrong_domain() {
    let profile = profile();
    let policy = exponential_policy();
    let now = virtual_time(DEADLINE_TICKS);
    assert_eq!(
        plan_retry(&profile, ACTIVE_GENERATION, SUBJECT_ID, STALE_GENERATION, &now, 0, policy, None),
        Err(DeadlineLeaseError::StaleGeneration {
            expected: ACTIVE_GENERATION,
            actual: STALE_GENERATION
        })
    );
    let mut wrong_profile = profile.clone();
    wrong_profile.supported_domains = vec![TimeDomain::Logical];
    assert_eq!(
        plan_retry(&wrong_profile, ACTIVE_GENERATION, SUBJECT_ID, ACTIVE_GENERATION, &now, 0, policy, None),
        Err(DeadlineLeaseError::Arithmetic(TimeArithmeticError::UnsupportedDomain(TimeDomain::Virtual)))
    );
    assert_eq!(now, virtual_time(DEADLINE_TICKS));
}

// r[verify molten.audit_f12.bounds]
// r[verify molten.audit_f12.compatibility]
#[test]
fn retry_saturation_does_not_authorize_deadline_overflow() {
    let now = virtual_time(u64::MAX);
    for backoff in [RetryBackoff::Fixed, RetryBackoff::Exponential] {
        let policy = RetryPolicy {
            backoff,
            ..exponential_policy()
        };
        assert_eq!(
            plan_retry(&profile(), ACTIVE_GENERATION, SUBJECT_ID, ACTIVE_GENERATION, &now, AUDIT_ATTEMPT, policy, None),
            Err(DeadlineLeaseError::Arithmetic(TimeArithmeticError::Overflow))
        );
    }
    assert_eq!(now, virtual_time(u64::MAX));
}
