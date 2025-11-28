// Health metrics aggregation for VM fleet monitoring
//
// This module provides aggregated health statistics across the VM fleet.
// It enables monitoring and observability of overall fleet health.

use std::collections::HashMap;
use uuid::Uuid;

use super::types::HealthStatus;

/// Aggregated health statistics for the VM fleet
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStats {
    pub total_vms: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
}

impl HealthStats {
    /// Calculate health statistics from a map of VM health statuses
    pub fn from_status_map(status_map: &HashMap<Uuid, HealthStatus>) -> Self {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut unknown = 0;

        for status in status_map.values() {
            match status {
                HealthStatus::Healthy { .. } => healthy += 1,
                HealthStatus::Degraded { .. } => degraded += 1,
                HealthStatus::Unhealthy { .. } => unhealthy += 1,
                HealthStatus::Unknown => unknown += 1,
            }
        }

        Self {
            total_vms: status_map.len(),
            healthy,
            degraded,
            unhealthy,
            unknown,
        }
    }

    /// Get percentage of healthy VMs
    pub fn healthy_percentage(&self) -> f64 {
        if self.total_vms == 0 {
            0.0
        } else {
            (self.healthy as f64 / self.total_vms as f64) * 100.0
        }
    }

    /// Get percentage of degraded VMs
    pub fn degraded_percentage(&self) -> f64 {
        if self.total_vms == 0 {
            0.0
        } else {
            (self.degraded as f64 / self.total_vms as f64) * 100.0
        }
    }

    /// Get percentage of unhealthy VMs
    pub fn unhealthy_percentage(&self) -> f64 {
        if self.total_vms == 0 {
            0.0
        } else {
            (self.unhealthy as f64 / self.total_vms as f64) * 100.0
        }
    }

    /// Check if fleet health is critical (>50% unhealthy or degraded)
    pub fn is_critical(&self) -> bool {
        if self.total_vms == 0 {
            return false;
        }
        let unhealthy_and_degraded = self.unhealthy + self.degraded;
        (unhealthy_and_degraded as f64 / self.total_vms as f64) > 0.5
    }

    /// Check if fleet health is good (>80% healthy)
    pub fn is_good(&self) -> bool {
        if self.total_vms == 0 {
            return true; // No VMs = no problems
        }
        (self.healthy as f64 / self.total_vms as f64) > 0.8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_fleet_stats() {
        let status_map = HashMap::new();
        let stats = HealthStats::from_status_map(&status_map);

        assert_eq!(stats.total_vms, 0);
        assert_eq!(stats.healthy, 0);
        assert_eq!(stats.degraded, 0);
        assert_eq!(stats.unhealthy, 0);
        assert_eq!(stats.unknown, 0);
    }

    #[test]
    fn test_fleet_stats_calculation() {
        let mut status_map = HashMap::new();

        status_map.insert(
            Uuid::new_v4(),
            HealthStatus::Healthy {
                last_check: 0,
                response_time_ms: 10,
            },
        );
        status_map.insert(
            Uuid::new_v4(),
            HealthStatus::Healthy {
                last_check: 0,
                response_time_ms: 15,
            },
        );
        status_map.insert(
            Uuid::new_v4(),
            HealthStatus::Degraded {
                last_check: 0,
                failures: 1,
                last_error: "test".to_string(),
            },
        );
        status_map.insert(
            Uuid::new_v4(),
            HealthStatus::Unhealthy {
                since: 0,
                failures: 3,
                last_error: "test".to_string(),
            },
        );
        status_map.insert(Uuid::new_v4(), HealthStatus::Unknown);

        let stats = HealthStats::from_status_map(&status_map);

        assert_eq!(stats.total_vms, 5);
        assert_eq!(stats.healthy, 2);
        assert_eq!(stats.degraded, 1);
        assert_eq!(stats.unhealthy, 1);
        assert_eq!(stats.unknown, 1);
    }

    #[test]
    fn test_healthy_percentage() {
        let mut status_map = HashMap::new();

        for _ in 0..8 {
            status_map.insert(
                Uuid::new_v4(),
                HealthStatus::Healthy {
                    last_check: 0,
                    response_time_ms: 10,
                },
            );
        }
        for _ in 0..2 {
            status_map.insert(
                Uuid::new_v4(),
                HealthStatus::Degraded {
                    last_check: 0,
                    failures: 1,
                    last_error: "test".to_string(),
                },
            );
        }

        let stats = HealthStats::from_status_map(&status_map);

        assert_eq!(stats.healthy_percentage(), 80.0);
        assert_eq!(stats.degraded_percentage(), 20.0);
        assert_eq!(stats.unhealthy_percentage(), 0.0);
    }

    #[test]
    fn test_is_critical() {
        let mut status_map = HashMap::new();

        // 40% healthy, 30% degraded, 30% unhealthy = 60% problematic = critical
        for _ in 0..4 {
            status_map.insert(
                Uuid::new_v4(),
                HealthStatus::Healthy {
                    last_check: 0,
                    response_time_ms: 10,
                },
            );
        }
        for _ in 0..3 {
            status_map.insert(
                Uuid::new_v4(),
                HealthStatus::Degraded {
                    last_check: 0,
                    failures: 1,
                    last_error: "test".to_string(),
                },
            );
        }
        for _ in 0..3 {
            status_map.insert(
                Uuid::new_v4(),
                HealthStatus::Unhealthy {
                    since: 0,
                    failures: 3,
                    last_error: "test".to_string(),
                },
            );
        }

        let stats = HealthStats::from_status_map(&status_map);
        assert!(stats.is_critical());
    }

    #[test]
    fn test_is_good() {
        let mut status_map = HashMap::new();

        // 90% healthy = good
        for _ in 0..9 {
            status_map.insert(
                Uuid::new_v4(),
                HealthStatus::Healthy {
                    last_check: 0,
                    response_time_ms: 10,
                },
            );
        }
        status_map.insert(
            Uuid::new_v4(),
            HealthStatus::Degraded {
                last_check: 0,
                failures: 1,
                last_error: "test".to_string(),
            },
        );

        let stats = HealthStats::from_status_map(&status_map);
        assert!(stats.is_good());
    }

    #[test]
    fn test_empty_fleet_is_good() {
        let status_map = HashMap::new();
        let stats = HealthStats::from_status_map(&status_map);
        assert!(stats.is_good());
        assert!(!stats.is_critical());
    }
}
