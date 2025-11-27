//! Cluster formation coordinator for Hiqlite distributed database
//!
//! Handles the complex logic of waiting for a Raft cluster to form,
//! including health checks, member discovery, and progress logging.

use anyhow::Result;
use hiqlite::Client;
use std::time::{Duration, Instant};

/// Configuration for cluster formation behavior
#[derive(Debug, Clone)]
pub struct ClusterFormationConfig {
    /// Timeout for cluster formation warnings
    pub formation_warning_timeout: Duration,
    /// Interval between health checks
    pub health_check_interval: Duration,
    /// Throttle interval for progress logging
    pub log_throttle_interval: Duration,
}

impl Default for ClusterFormationConfig {
    fn default() -> Self {
        Self {
            formation_warning_timeout: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(1),
            log_throttle_interval: Duration::from_secs(5),
        }
    }
}

impl ClusterFormationConfig {
    /// Create configuration from operational config
    pub fn from_operational(operational: &crate::config::OperationalConfig) -> Self {
        Self {
            formation_warning_timeout: Duration::from_secs(operational.hiqlite_check_timeout_secs),
            health_check_interval: Duration::from_secs(operational.hiqlite_retry_delay_secs),
            log_throttle_interval: Duration::from_secs(operational.hiqlite_log_throttle_secs),
        }
    }
}

/// Coordinates waiting for a Raft cluster to form and become healthy
pub struct ClusterFormationCoordinator {
    config: ClusterFormationConfig,
}

impl ClusterFormationCoordinator {
    pub fn new(config: ClusterFormationConfig) -> Self {
        Self { config }
    }

    /// Wait for cluster to form with the expected number of nodes
    ///
    /// This method blocks until:
    /// - The cluster is healthy (has a leader)
    /// - All expected nodes are voting members
    ///
    /// # Arguments
    /// - `client`: The Hiqlite client to check
    /// - `node_id`: The current node's ID for logging
    /// - `expected_nodes`: Number of nodes expected in the cluster
    pub async fn wait_for_cluster_formation(
        &self,
        client: &Client,
        node_id: u64,
        expected_nodes: usize,
    ) -> Result<()> {
        tracing::info!(
            node_id = node_id,
            expected_nodes = expected_nodes,
            "Waiting for Raft cluster to form..."
        );

        let start_time = Instant::now();
        let mut last_log_time = Instant::now();

        loop {
            let elapsed = start_time.elapsed();

            // Warn if cluster formation is taking too long
            if elapsed > self.config.formation_warning_timeout {
                tracing::warn!(
                    node_id = node_id,
                    elapsed_secs = elapsed.as_secs(),
                    "Cluster formation is taking longer than expected - check network connectivity and hiqlite configuration"
                );
            }

            // Check if cluster is healthy
            match client.is_healthy_db().await {
                Ok(_) => {
                    // Check if all nodes are members
                    if let Ok(metrics) = client.metrics_db().await {
                        let membership = metrics.membership_config.membership();
                        let online_nodes = membership.nodes().count();
                        let voter_nodes = membership.voter_ids().count();
                        let learner_nodes = membership.learner_ids().count();

                        // Collect member IDs for debugging
                        let member_ids: Vec<_> = membership.nodes().map(|(id, _)| *id).collect();
                        let voter_ids: Vec<_> = membership.voter_ids().collect();

                        if online_nodes == expected_nodes && voter_nodes == expected_nodes {
                            tracing::info!(
                                node_id = node_id,
                                online_nodes = online_nodes,
                                voter_nodes = voter_nodes,
                                member_ids = ?member_ids,
                                elapsed_secs = elapsed.as_secs(),
                                "Raft cluster fully formed - all nodes are voting members"
                            );
                            return Ok(());
                        } else {
                            // Log progress periodically to avoid spam
                            if last_log_time.elapsed() > self.config.log_throttle_interval {
                                tracing::info!(
                                    node_id = node_id,
                                    online_nodes = online_nodes,
                                    voter_nodes = voter_nodes,
                                    learner_nodes = learner_nodes,
                                    expected_nodes = expected_nodes,
                                    member_ids = ?member_ids,
                                    voter_ids = ?voter_ids,
                                    "Waiting for all nodes to join cluster..."
                                );
                                last_log_time = Instant::now();
                            }
                        }
                    }
                }
                Err(e) => {
                    // Log at INFO level during cluster formation
                    if last_log_time.elapsed() > self.config.log_throttle_interval {
                        tracing::info!(
                            node_id = node_id,
                            error = %e,
                            elapsed_secs = elapsed.as_secs(),
                            "Cluster not yet healthy, waiting..."
                        );
                        last_log_time = Instant::now();
                    }
                }
            }

            tokio::time::sleep(self.config.health_check_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_config_defaults() {
        let config = ClusterFormationConfig::default();
        assert_eq!(config.formation_warning_timeout, Duration::from_secs(60));
        assert_eq!(config.health_check_interval, Duration::from_secs(1));
        assert_eq!(config.log_throttle_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_cluster_config_from_operational() {
        let mut op_config = crate::config::OperationalConfig::default();
        op_config.hiqlite_check_timeout_secs = 120;
        op_config.hiqlite_retry_delay_secs = 2;
        op_config.hiqlite_log_throttle_secs = 10;

        let config = ClusterFormationConfig::from_operational(&op_config);
        assert_eq!(config.formation_warning_timeout, Duration::from_secs(120));
        assert_eq!(config.health_check_interval, Duration::from_secs(2));
        assert_eq!(config.log_throttle_interval, Duration::from_secs(10));
    }
}
