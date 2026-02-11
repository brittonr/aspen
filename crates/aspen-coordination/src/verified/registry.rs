//! Pure service registry computation functions.
//!
//! This module contains pure functions for service registry operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

use crate::registry::HealthStatus;

/// Service registry key prefix.
pub const SERVICE_PREFIX: &str = "__service:";

// ============================================================================
// Key Generation
// ============================================================================

/// Generate the key for a service instance.
///
/// # Example
///
/// ```ignore
/// assert_eq!(instance_key("api", "inst-1"), "__service:api:inst-1");
/// ```
#[inline]
pub fn instance_key(service_name: &str, instance_id: &str) -> String {
    format!("{}{}:{}", SERVICE_PREFIX, service_name, instance_id)
}

/// Generate the prefix for scanning all instances of a service.
///
/// # Example
///
/// ```ignore
/// assert_eq!(service_instances_prefix("api"), "__service:api:");
/// ```
#[inline]
pub fn service_instances_prefix(service_name: &str) -> String {
    format!("{}{}:", SERVICE_PREFIX, service_name)
}

/// Generate the prefix for scanning services by name prefix.
///
/// # Example
///
/// ```ignore
/// assert_eq!(services_scan_prefix("api-"), "__service:api-");
/// ```
#[inline]
pub fn services_scan_prefix(prefix: &str) -> String {
    format!("{}{}", SERVICE_PREFIX, prefix)
}

// ============================================================================
// Expiry
// ============================================================================

/// Check if a service instance has expired.
///
/// An instance is expired if:
/// - It has a deadline (deadline_ms > 0), AND
/// - The current time has passed the deadline
///
/// # Arguments
///
/// * `deadline_ms` - Instance deadline in Unix milliseconds (0 = no expiration)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the instance has expired.
#[inline]
pub fn is_instance_expired(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms > 0 && now_ms > deadline_ms
}

/// Calculate remaining TTL for a service instance.
///
/// # Arguments
///
/// * `deadline_ms` - Instance deadline in Unix milliseconds (0 = no expiration)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Remaining time in milliseconds. Returns `u64::MAX` if no deadline (lease-based).
///
/// # Tiger Style
///
/// - Uses saturating_sub to prevent underflow
#[inline]
pub fn instance_remaining_ttl(deadline_ms: u64, now_ms: u64) -> u64 {
    if deadline_ms == 0 {
        return u64::MAX;
    }
    deadline_ms.saturating_sub(now_ms)
}

/// Check if a service instance matches a discovery filter.
///
/// # Arguments
///
/// * `health_status` - Instance health status
/// * `tags` - Instance tags
/// * `version` - Instance version string
/// * `healthy_only` - Filter: only match healthy instances
/// * `required_tags` - Filter: all these tags must be present
/// * `version_prefix` - Filter: version must start with this prefix
///
/// # Returns
///
/// `true` if the instance matches all filter criteria.
#[inline]
pub fn matches_discovery_filter<S: AsRef<str>>(
    health_status: HealthStatus,
    tags: &[S],
    version: &str,
    healthy_only: bool,
    required_tags: &[S],
    version_prefix: Option<&str>,
) -> bool {
    // Check health filter
    if healthy_only && health_status != HealthStatus::Healthy {
        return false;
    }

    // Check tags filter
    if !required_tags.is_empty() {
        let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_ref()).collect();
        for required in required_tags {
            if !tag_strs.contains(&required.as_ref()) {
                return false;
            }
        }
    }

    // Check version prefix filter
    if let Some(prefix) = version_prefix
        && !version.starts_with(prefix)
    {
        return false;
    }

    true
}

/// Compute the new deadline for an instance heartbeat.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `ttl_ms` - Time-to-live in milliseconds
/// * `is_lease_based` - Whether the instance uses lease-based expiration
///
/// # Returns
///
/// The new deadline in Unix milliseconds (0 for lease-based).
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_heartbeat_deadline(now_ms: u64, ttl_ms: u64, is_lease_based: bool) -> u64 {
    if is_lease_based {
        0
    } else {
        now_ms.saturating_add(ttl_ms)
    }
}

/// Compute the next fencing token for an instance.
///
/// # Arguments
///
/// * `current_token` - Current fencing token (0 if new instance)
///
/// # Returns
///
/// The next fencing token.
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_next_instance_token(current_token: u64) -> u64 {
    current_token.saturating_add(1)
}

/// Alias for compute_next_instance_token for consistency with Verus specs.
#[inline]
pub fn compute_next_registry_token(current_token: u64) -> u64 {
    current_token.saturating_add(1)
}

// ============================================================================
// Service Lifecycle
// ============================================================================

/// Calculate the deadline for a service registration.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `ttl_ms` - Time-to-live in milliseconds
///
/// # Returns
///
/// Deadline in Unix milliseconds.
#[inline]
pub fn calculate_service_deadline(now_ms: u64, ttl_ms: u64) -> u64 {
    now_ms.saturating_add(ttl_ms)
}

/// Check if a service registration has expired.
///
/// # Arguments
///
/// * `deadline_ms` - Service deadline (0 = no expiration)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the service has expired.
#[inline]
pub fn is_service_expired(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms > 0 && now_ms > deadline_ms
}

/// Check if a service is live (not expired).
///
/// # Arguments
///
/// * `deadline_ms` - Service deadline (0 = no expiration)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the service is live.
#[inline]
pub fn is_service_live(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms == 0 || now_ms <= deadline_ms
}

/// Calculate time until service lease expiration.
///
/// # Arguments
///
/// * `deadline_ms` - Service deadline (0 = no expiration)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Time until expiration in milliseconds (u64::MAX if no expiration).
#[inline]
pub fn time_until_lease_expiration(deadline_ms: u64, now_ms: u64) -> u64 {
    if deadline_ms == 0 {
        return u64::MAX;
    }
    deadline_ms.saturating_sub(now_ms)
}

// ============================================================================
// Registration Validation
// ============================================================================

/// Check if a registration is valid.
///
/// A registration is valid if:
/// - Service name is not empty
/// - Instance ID is not empty
/// - TTL is positive (or 0 for lease-based)
///
/// # Arguments
///
/// * `service_name` - Service name
/// * `instance_id` - Instance identifier
///
/// # Returns
///
/// `true` if the registration is valid.
#[inline]
pub fn is_valid_registration(service_name: &str, instance_id: &str) -> bool {
    !service_name.is_empty() && !instance_id.is_empty()
}

/// Normalize a service weight to the range [0, 100].
///
/// # Arguments
///
/// * `weight` - Raw weight value
///
/// # Returns
///
/// Normalized weight in [0, 100].
#[inline]
pub fn normalize_service_weight(weight: u32) -> u32 {
    if weight > 100 { 100 } else { weight }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Key Generation Tests
    // ========================================================================

    #[test]
    fn test_instance_key() {
        assert_eq!(instance_key("api", "inst-1"), "__service:api:inst-1");
        assert_eq!(instance_key("", ""), "__service::");
    }

    #[test]
    fn test_service_instances_prefix() {
        assert_eq!(service_instances_prefix("api"), "__service:api:");
    }

    #[test]
    fn test_services_scan_prefix() {
        assert_eq!(services_scan_prefix("api-"), "__service:api-");
    }

    // ========================================================================
    // Expiry Tests
    // ========================================================================

    #[test]
    fn test_instance_expired_no_deadline() {
        // deadline = 0 means lease-based, never expires via TTL
        assert!(!is_instance_expired(0, 1000));
        assert!(!is_instance_expired(0, u64::MAX));
    }

    #[test]
    fn test_instance_expired_past_deadline() {
        assert!(is_instance_expired(1000, 2000));
    }

    #[test]
    fn test_instance_expired_active() {
        assert!(!is_instance_expired(2000, 1000));
    }

    #[test]
    fn test_instance_expired_at_deadline() {
        assert!(!is_instance_expired(1000, 1000));
    }

    #[test]
    fn test_remaining_ttl_no_deadline() {
        assert_eq!(instance_remaining_ttl(0, 1000), u64::MAX);
    }

    #[test]
    fn test_remaining_ttl_active() {
        assert_eq!(instance_remaining_ttl(2000, 1000), 1000);
    }

    #[test]
    fn test_remaining_ttl_expired() {
        assert_eq!(instance_remaining_ttl(1000, 2000), 0);
    }

    #[test]
    fn test_matches_filter_no_filters() {
        let tags: Vec<String> = vec![];
        let required: Vec<String> = vec![];
        assert!(matches_discovery_filter(HealthStatus::Healthy, &tags, "1.0.0", false, &required, None));
    }

    #[test]
    fn test_matches_filter_healthy_only() {
        let tags: Vec<String> = vec![];
        let required: Vec<String> = vec![];
        assert!(matches_discovery_filter(HealthStatus::Healthy, &tags, "1.0.0", true, &required, None));
        assert!(!matches_discovery_filter(HealthStatus::Unhealthy, &tags, "1.0.0", true, &required, None));
        assert!(!matches_discovery_filter(HealthStatus::Unknown, &tags, "1.0.0", true, &required, None));
    }

    #[test]
    fn test_matches_filter_tags() {
        let tags = vec!["region:us-east".to_string(), "env:prod".to_string()];
        let required = vec!["region:us-east".to_string()];
        assert!(matches_discovery_filter(HealthStatus::Healthy, &tags, "1.0.0", false, &required, None));

        let missing = vec!["region:us-west".to_string()];
        assert!(!matches_discovery_filter(HealthStatus::Healthy, &tags, "1.0.0", false, &missing, None));
    }

    #[test]
    fn test_matches_filter_version_prefix() {
        let tags: Vec<String> = vec![];
        let required: Vec<String> = vec![];
        assert!(matches_discovery_filter(HealthStatus::Healthy, &tags, "1.0.0", false, &required, Some("1.")));
        assert!(!matches_discovery_filter(HealthStatus::Healthy, &tags, "2.0.0", false, &required, Some("1.")));
    }

    #[test]
    fn test_heartbeat_deadline() {
        assert_eq!(compute_heartbeat_deadline(1000, 30000, false), 31000);
        assert_eq!(compute_heartbeat_deadline(1000, 30000, true), 0);
    }

    #[test]
    fn test_heartbeat_deadline_overflow() {
        assert_eq!(compute_heartbeat_deadline(u64::MAX, 1, false), u64::MAX);
    }

    #[test]
    fn test_next_instance_token() {
        assert_eq!(compute_next_instance_token(0), 1);
        assert_eq!(compute_next_instance_token(5), 6);
        assert_eq!(compute_next_instance_token(u64::MAX), u64::MAX);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_no_deadline_never_expires() {
        check!().with_type::<u64>().for_each(|now| {
            assert!(!is_instance_expired(0, *now));
        });
    }

    #[test]
    fn prop_remaining_ttl_consistent() {
        check!().with_type::<(u64, u64)>().for_each(|(deadline, now)| {
            if *deadline == 0 {
                assert_eq!(instance_remaining_ttl(*deadline, *now), u64::MAX);
            } else {
                let remaining = instance_remaining_ttl(*deadline, *now);
                let expired = is_instance_expired(*deadline, *now);
                if expired {
                    assert_eq!(remaining, 0);
                } else {
                    assert!(remaining > 0 || *deadline == *now);
                }
            }
        });
    }

    #[test]
    fn prop_token_monotonic() {
        check!().with_type::<u64>().for_each(|current| {
            let next = compute_next_instance_token(*current);
            assert!(next >= *current);
        });
    }

    #[test]
    fn prop_no_filters_always_matches_healthy() {
        let empty_tags: Vec<String> = vec![];
        let empty_required: Vec<String> = vec![];
        check!().with_type::<String>().for_each(|version| {
            // With no filters, healthy instances always match
            assert!(matches_discovery_filter(
                HealthStatus::Healthy,
                &empty_tags,
                version,
                false,
                &empty_required,
                None
            ));
        });
    }
}
