//! Deterministic receive budgeting before identity-reference resolution.
//!
//! A shell supplies an observed network address and completed-handshake facts.
//! Passing this gate is not membership, trust, or capability admission.

use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use core::net::IpAddr;
use core::net::Ipv4Addr;
use core::net::Ipv6Addr;

use super::EndpointParticipantRole;
use super::MAX_TRANSPORT_TEXT_BYTES;
use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

/// Observed IPv4 address or IPv6 /64, never a claimed peer identity or port.
/// IPv4-mapped IPv6 addresses share the corresponding IPv4 bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReceiveSource(SourceKey);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceKey {
    Ipv4(Ipv4Addr),
    Ipv6Prefix([u8; 8]),
}

impl From<IpAddr> for ReceiveSource {
    fn from(address: IpAddr) -> Self {
        match address {
            IpAddr::V4(address) => Self::from(address),
            IpAddr::V6(address) => Self::from(address),
        }
    }
}

impl From<Ipv4Addr> for ReceiveSource {
    fn from(address: Ipv4Addr) -> Self {
        Self(SourceKey::Ipv4(address))
    }
}

impl From<Ipv6Addr> for ReceiveSource {
    fn from(address: Ipv6Addr) -> Self {
        if let Some(address) = address.to_ipv4_mapped() {
            return Self::from(address);
        }
        let octets = address.octets();
        Self(SourceKey::Ipv6Prefix([
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5], octets[6], octets[7],
        ]))
    }
}

/// One token per attempt; one token refills per interval, up to `burst`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveBudget {
    pub burst: u64,
    pub refill_ticks: u64,
    pub max_sources: usize,
}

/// Bounded handshake facts only; no reference resolution takes place here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveObservation {
    pub peer_identity: String,
    pub peer_context_ref: String,
    pub service_id: String,
    pub alpn: String,
    pub generation: u64,
    pub role: EndpointParticipantRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveObservationField {
    PeerIdentity,
    PeerContextRef,
    ServiceId,
    Alpn,
    Generation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveDenyReason {
    InvalidBudget,
    BudgetOverflow,
    SourceSaturated,
    BackwardsTime,
    TickOverflow,
    RateLimited,
    InvalidObservation(ReceiveObservationField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveDeny {
    pub reason: ReceiveDenyReason,
    /// Relative injected ticks; None means retrying after a delay is not sufficient.
    pub retry_ticks: Option<u64>,
}

impl ReceiveDeny {
    pub const fn code(self) -> &'static str {
        match self.reason {
            ReceiveDenyReason::InvalidBudget => "receive-invalid-budget",
            ReceiveDenyReason::BudgetOverflow => "receive-budget-overflow",
            ReceiveDenyReason::SourceSaturated => "receive-source-saturated",
            ReceiveDenyReason::BackwardsTime => "receive-backwards-time",
            ReceiveDenyReason::TickOverflow => "receive-tick-overflow",
            ReceiveDenyReason::RateLimited => "receive-rate-limited",
            ReceiveDenyReason::InvalidObservation(_) => "receive-invalid-observation",
        }
    }

    const fn terminal(reason: ReceiveDenyReason) -> Self {
        Self {
            reason,
            retry_ticks: None,
        }
    }
}

/// Proof that this observation consumed a receive-budget token.
///
/// Construction is private, including from other modules in molten-core.
/// `GatedInbound` is deliberately not Clone: handlers consume the gate result.
///
/// ```compile_fail
/// use molten_core::fabric_transport::{GatedInbound, ReceiveObservation};
/// fn forge(observation: ReceiveObservation) -> GatedInbound {
///     GatedInbound { observation }
/// }
/// ```
///
/// A costly handler requiring the proof cannot receive raw observations.
///
/// ```compile_fail
/// use molten_core::fabric_transport::{GatedInbound, ReceiveObservation};
/// fn costly_authority_handler(_inbound: GatedInbound) {}
/// fn bypass(observation: ReceiveObservation) {
///     costly_authority_handler(observation);
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct GatedInbound {
    observation: ReceiveObservation,
}

impl GatedInbound {
    pub fn observation(&self) -> &ReceiveObservation {
        &self.observation
    }
}

#[derive(Debug)]
struct ReceiveBucket {
    tokens: u64,
    refill_at: u64,
}

/// Bounded source registry with no eviction or identity-churn reset path.
///
/// Keep one gate alive for the listener lifetime. Reconstructing a gate resets
/// its budget and is an explicit owner lifecycle decision, not a denial retry.
#[derive(Debug)]
pub struct ReceiveGate {
    budget: ReceiveBudget,
    buckets: BTreeMap<ReceiveSource, ReceiveBucket>,
    last_observed_tick: Option<u64>,
}

// r[impl molten.fabric_transport.receive_gate]
impl ReceiveGate {
    pub fn new(budget: ReceiveBudget) -> Result<Self, ReceiveDeny> {
        if budget.burst == 0 || budget.refill_ticks == 0 || budget.max_sources == 0 {
            return Err(ReceiveDeny::terminal(ReceiveDenyReason::InvalidBudget));
        }
        if budget.burst.checked_mul(budget.refill_ticks).is_none() {
            return Err(ReceiveDeny::terminal(ReceiveDenyReason::BudgetOverflow));
        }
        Ok(Self {
            budget,
            buckets: BTreeMap::new(),
            last_observed_tick: None,
        })
    }

    /// Charge the source before validating bounded facts or resolving any refs.
    ///
    /// Equal ticks are permitted. Backwards time is rejected globally, even for
    /// unseen sources. Monotonic denied attempts advance the high-water mark.
    /// Malformed observations consume tokens, so invalid input is not a bypass.
    pub fn admit_inbound(
        &mut self,
        source: ReceiveSource,
        observation: ReceiveObservation,
        now: u64,
    ) -> Result<GatedInbound, ReceiveDeny> {
        if self.last_observed_tick.is_some_and(|last| now < last) {
            return Err(ReceiveDeny::terminal(ReceiveDenyReason::BackwardsTime));
        }
        self.last_observed_tick = Some(now);
        let source_capacity_reached = self.buckets.len() >= self.budget.max_sources;
        let bucket = match self.buckets.entry(source) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                if source_capacity_reached {
                    return Err(ReceiveDeny::terminal(ReceiveDenyReason::SourceSaturated));
                }
                entry.insert(ReceiveBucket {
                    tokens: self.budget.burst,
                    refill_at: now,
                })
            }
        };
        let elapsed = now - bucket.refill_at;
        let refill = elapsed / self.budget.refill_ticks;
        let missing = self.budget.burst - bucket.tokens;
        if refill >= missing {
            bucket.tokens = self.budget.burst;
            bucket.refill_at = now;
        } else {
            // refill < missing guarantees this addition cannot exceed burst.
            bucket.tokens += refill;
            bucket.refill_at = now - elapsed % self.budget.refill_ticks;
        }
        if bucket.tokens == 0 {
            let retry_ticks = self.budget.refill_ticks - (now - bucket.refill_at);
            if now.checked_add(retry_ticks).is_none() {
                return Err(ReceiveDeny::terminal(ReceiveDenyReason::TickOverflow));
            }
            return Err(ReceiveDeny {
                reason: ReceiveDenyReason::RateLimited,
                retry_ticks: Some(retry_ticks),
            });
        }
        bucket.tokens -= 1;
        validate_observation(&observation)?;
        Ok(GatedInbound { observation })
    }
}

fn validate_observation(observation: &ReceiveObservation) -> Result<(), ReceiveDeny> {
    let field = if observation.peer_identity.is_empty()
        || observation.peer_identity.len() > MAX_TRANSPORT_TEXT_BYTES
        || !observation.peer_identity.bytes().all(|byte| byte.is_ascii_graphic())
    {
        Some(ReceiveObservationField::PeerIdentity)
    } else if !valid_blake3_ref(&observation.peer_context_ref) {
        Some(ReceiveObservationField::PeerContextRef)
    } else if observation.service_id.len() > MAX_TRANSPORT_TEXT_BYTES || !valid_fabric_token(&observation.service_id) {
        Some(ReceiveObservationField::ServiceId)
    } else if observation.alpn.is_empty()
        || observation.alpn.len() > MAX_TRANSPORT_TEXT_BYTES
        || !observation.alpn.bytes().all(|byte| byte.is_ascii_graphic())
    {
        Some(ReceiveObservationField::Alpn)
    } else if observation.generation == 0 {
        Some(ReceiveObservationField::Generation)
    } else {
        None
    };
    match field {
        Some(field) => Err(ReceiveDeny::terminal(ReceiveDenyReason::InvalidObservation(field))),
        None => Ok(()),
    }
}

// r[verify molten.fabric_transport.receive_gate]
#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn observation() -> ReceiveObservation {
        ReceiveObservation {
            peer_identity: "0123456789abcdef".repeat(4),
            peer_context_ref: CONTEXT_REF.to_string(),
            service_id: "direct-stream-service".to_string(),
            alpn: "molten/direct-stream/1".to_string(),
            generation: 1,
            role: EndpointParticipantRole::Client,
        }
    }

    fn source(last: u8) -> ReceiveSource {
        ReceiveSource::from(Ipv4Addr::new(192, 0, 2, last))
    }

    fn gate(burst: u64, max_sources: usize) -> ReceiveGate {
        ReceiveGate::new(ReceiveBudget {
            burst,
            refill_ticks: 10,
            max_sources,
        })
        .expect("valid receive budget")
    }

    #[test]
    fn budget_requires_positive_limits_and_representable_refill_horizon() {
        for budget in [
            ReceiveBudget {
                burst: 0,
                refill_ticks: 1,
                max_sources: 1,
            },
            ReceiveBudget {
                burst: 1,
                refill_ticks: 0,
                max_sources: 1,
            },
            ReceiveBudget {
                burst: 1,
                refill_ticks: 1,
                max_sources: 0,
            },
        ] {
            assert_eq!(ReceiveGate::new(budget).expect_err("zero limit").reason, ReceiveDenyReason::InvalidBudget,);
        }
        assert_eq!(
            ReceiveGate::new(ReceiveBudget {
                burst: u64::MAX,
                refill_ticks: 2,
                max_sources: 1
            })
            .expect_err("unrepresentable refill horizon")
            .reason,
            ReceiveDenyReason::BudgetOverflow,
        );
    }

    #[test]
    fn integer_refill_preserves_fractional_ticks_and_caps_burst() {
        let mut gate = gate(2, 1);
        assert!(gate.admit_inbound(source(1), observation(), 100).is_ok());
        assert!(gate.admit_inbound(source(1), observation(), 100).is_ok());
        let denied = gate.admit_inbound(source(1), observation(), 109).expect_err("fractional interval");
        assert_eq!(denied.reason, ReceiveDenyReason::RateLimited);
        assert_eq!(denied.code(), "receive-rate-limited");
        assert_eq!(denied.retry_ticks, Some(1));
        assert!(gate.admit_inbound(source(1), observation(), 115).is_ok());
        assert_eq!(gate.admit_inbound(source(1), observation(), 119).expect_err("next interval").retry_ticks, Some(1),);
        assert!(gate.admit_inbound(source(1), observation(), 120).is_ok());
        assert!(gate.admit_inbound(source(1), observation(), 1_000).is_ok());
        assert!(gate.admit_inbound(source(1), observation(), 1_000).is_ok());
        assert_eq!(
            gate.admit_inbound(source(1), observation(), 1_000).expect_err("capped burst").retry_ticks,
            Some(10),
        );
    }

    #[test]
    fn source_budget_cannot_be_reset_by_identity_service_generation_or_role() {
        let mut gate = gate(1, 2);
        let original = observation();
        assert_eq!(gate.admit_inbound(source(1), original.clone(), 0).expect("first").observation(), &original);
        let mut changed = observation();
        changed.peer_identity = "different-peer".to_string();
        changed.service_id = "different-service".to_string();
        changed.generation = 2;
        changed.role = EndpointParticipantRole::Listener;
        assert_eq!(
            gate.admit_inbound(source(1), changed, 0).expect_err("same source").reason,
            ReceiveDenyReason::RateLimited,
        );
        assert!(gate.admit_inbound(source(2), observation(), 0).is_ok());
    }

    #[test]
    fn ipv6_hosts_share_a_prefix_but_distinct_prefixes_are_isolated() {
        let first = ReceiveSource::from("2001:db8:1:2::1".parse::<Ipv6Addr>().expect("IPv6"));
        let rotated = ReceiveSource::from("2001:db8:1:2:ffff:ffff:ffff:ffff".parse::<Ipv6Addr>().expect("IPv6"));
        let other = ReceiveSource::from("2001:db8:1:3::1".parse::<IpAddr>().expect("IP"));
        assert_eq!(first, rotated);
        assert_ne!(first, other);
        let mut gate = gate(1, 2);
        assert!(gate.admit_inbound(first, observation(), 0).is_ok());
        assert_eq!(
            gate.admit_inbound(rotated, observation(), 0).expect_err("same /64").reason,
            ReceiveDenyReason::RateLimited,
        );
        assert!(gate.admit_inbound(other, observation(), 0).is_ok());
        assert_eq!(ReceiveSource::from("::ffff:192.0.2.1".parse::<Ipv6Addr>().expect("mapped IPv6")), source(1),);
    }

    #[test]
    fn saturation_never_evicts_a_source_or_refunds_its_tokens() {
        let mut gate = gate(1, 1);
        assert!(gate.admit_inbound(source(1), observation(), 0).is_ok());
        let denied = gate.admit_inbound(source(2), observation(), 1).expect_err("source registry full");
        assert_eq!(denied.reason, ReceiveDenyReason::SourceSaturated);
        assert_eq!(denied.retry_ticks, None);
        assert_eq!(
            gate.admit_inbound(source(1), observation(), 1).expect_err("original budget retained").retry_ticks,
            Some(9),
        );
        assert!(gate.admit_inbound(source(1), observation(), 10).is_ok());
        assert_eq!(
            gate.admit_inbound(source(2), observation(), 100).expect_err("no idle eviction").reason,
            ReceiveDenyReason::SourceSaturated,
        );
        assert_eq!(gate.buckets.len(), 1);
    }

    #[test]
    fn time_is_monotonic_across_sources_and_denials() {
        let mut gate = gate(1, 2);
        assert!(gate.admit_inbound(source(1), observation(), 10).is_ok());
        assert_eq!(
            gate.admit_inbound(source(2), observation(), 9).expect_err("new source cannot rewind").reason,
            ReceiveDenyReason::BackwardsTime,
        );
        assert_eq!(gate.buckets.len(), 1);
        assert_eq!(
            gate.admit_inbound(source(1), observation(), 19).expect_err("limited").reason,
            ReceiveDenyReason::RateLimited,
        );
        assert_eq!(
            gate.admit_inbound(source(2), observation(), 18).expect_err("denial advances clock").reason,
            ReceiveDenyReason::BackwardsTime,
        );
        assert!(gate.admit_inbound(source(2), observation(), 19).is_ok());
        assert!(gate.admit_inbound(source(1), observation(), 20).is_ok());
    }

    #[test]
    fn tick_overflow_is_a_terminal_deny_not_a_wrapped_retry() {
        let mut gate = gate(1, 1);
        assert!(gate.admit_inbound(source(1), observation(), u64::MAX - 1).is_ok());
        let denied = gate.admit_inbound(source(1), observation(), u64::MAX).expect_err("future refill unrepresentable");
        assert_eq!(denied.reason, ReceiveDenyReason::TickOverflow);
        assert_eq!(denied.retry_ticks, None);
        assert_eq!(denied.code(), "receive-tick-overflow");
    }

    #[test]
    fn huge_elapsed_time_refills_without_intermediate_overflow() {
        let mut gate = gate(2, 1);
        assert!(gate.admit_inbound(source(1), observation(), 0).is_ok());
        assert!(gate.admit_inbound(source(1), observation(), u64::MAX).is_ok());
        assert!(gate.admit_inbound(source(1), observation(), u64::MAX).is_ok());
        assert_eq!(
            gate.admit_inbound(source(1), observation(), u64::MAX).expect_err("exhausted").reason,
            ReceiveDenyReason::TickOverflow,
        );
    }

    #[test]
    fn malformed_observations_are_bounded_and_still_consume_budget() {
        for field in [
            ReceiveObservationField::PeerIdentity,
            ReceiveObservationField::PeerContextRef,
            ReceiveObservationField::ServiceId,
            ReceiveObservationField::Alpn,
            ReceiveObservationField::Generation,
        ] {
            let mut gate = gate(1, 1);
            let mut invalid = observation();
            match field {
                ReceiveObservationField::PeerIdentity => {
                    invalid.peer_identity = "x".repeat(MAX_TRANSPORT_TEXT_BYTES + 1);
                }
                ReceiveObservationField::PeerContextRef => {
                    invalid.peer_context_ref = "not-a-content-ref".to_string();
                }
                ReceiveObservationField::ServiceId => invalid.service_id = "not a token".to_string(),
                ReceiveObservationField::Alpn => invalid.alpn = "not an alpn".to_string(),
                ReceiveObservationField::Generation => invalid.generation = 0,
            }
            assert_eq!(
                gate.admit_inbound(source(1), invalid, 0).expect_err("malformed facts").reason,
                ReceiveDenyReason::InvalidObservation(field),
            );
            assert_eq!(
                gate.admit_inbound(source(1), observation(), 0).expect_err("invalid attempts charged").reason,
                ReceiveDenyReason::RateLimited,
            );
            assert!(gate.admit_inbound(source(1), observation(), 10).is_ok());
        }
    }

    #[test]
    fn identical_budget_observations_and_ticks_reproduce_denials_and_tokens() {
        let budget = ReceiveBudget {
            burst: 2,
            refill_ticks: 7,
            max_sources: 2,
        };
        let mut first = ReceiveGate::new(budget).expect("budget");
        let mut replay = ReceiveGate::new(budget).expect("budget");
        for (host, now) in [(1, 0), (1, 0), (1, 6), (2, 6), (3, 6), (2, 5), (1, 7), (2, 20)] {
            assert_eq!(
                first.admit_inbound(source(host), observation(), now),
                replay.admit_inbound(source(host), observation(), now),
            );
        }
    }
}
