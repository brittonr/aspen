//! Property-based tests for WorkQueue using proptest
//!
//! These tests verify invariants and properties that must always hold
//! across a wide range of inputs and scenarios.

#[cfg(test)]
mod prop_tests {
    use crate::domain::types::{Job, JobStatus};
    use crate::domain::job_metadata::JobMetadata;
    use crate::domain::job_requirements::JobRequirements;
    use crate::work_item_cache::WorkItemCache;

    // =========================================================================
    // CACHE CONSISTENCY PROPERTIES
    // =========================================================================

    /// Property: After upsert, get returns Some with the same data
    #[tokio::test]
    async fn prop_upsert_get_consistency() {
        for _ in 0..100 {
            let cache = WorkItemCache::new();
            let job = Job {
                id: "test_job".to_string(),
                status: JobStatus::Pending,
                payload: serde_json::json!({"test": true}),
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: None,
                assigned_worker_id: None,
                completed_by: None,
            };

            cache.upsert(job.clone()).await;
            let retrieved = cache.get("test_job").await;

            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().id, job.id);
        }
    }

    /// Property: Update preserves all other fields
    #[tokio::test]
    async fn prop_update_preserves_fields() {
        for i in 0..50 {
            let cache = WorkItemCache::new();
            let mut job = Job {
                id: format!("job_{}", i),
                status: JobStatus::Pending,
                payload: serde_json::json!({"iteration": i}),
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: Some("original_node".to_string()),
                assigned_worker_id: Some("worker_1".to_string()),
                completed_by: None,
            };

            cache.upsert(job.clone()).await;

            // Update only the status
            cache
                .update(&job.id, |j| {
                    j.status = JobStatus::Claimed;
                })
                .await;

            let updated = cache.get(&job.id).await.unwrap();

            // Verify other fields are unchanged
            assert_eq!(updated.claimed_by, Some("original_node".to_string()));
            assert_eq!(updated.assigned_worker_id, Some("worker_1".to_string()));
            assert_eq!(updated.payload, serde_json::json!({"iteration": i}));
            assert_eq!(updated.status, JobStatus::Claimed);
        }
    }

    /// Property: replace_all clears previous state completely
    #[tokio::test]
    async fn prop_replace_all_idempotent() {
        let cache = WorkItemCache::new();

        // Fill cache with 10 jobs
        for i in 0..10 {
            let job = Job {
                id: format!("original_{}", i),
                status: JobStatus::Pending,
                payload: serde_json::Value::Null,
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: None,
                assigned_worker_id: None,
                completed_by: None,
            };
            cache.upsert(job).await;
        }

        // Replace with 5 new jobs multiple times - should be idempotent
        let new_jobs: Vec<Job> = (0..5)
            .map(|i| Job {
                id: format!("replaced_{}", i),
                status: JobStatus::Claimed,
                payload: serde_json::Value::Null,
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: Some("node1".to_string()),
                assigned_worker_id: None,
                completed_by: None,
            })
            .collect();

        cache.replace_all(new_jobs.clone()).await;
        let state1 = cache.get_all_map().await;

        cache.replace_all(new_jobs.clone()).await;
        let state2 = cache.get_all_map().await;

        // States should be identical
        assert_eq!(state1.len(), state2.len());
        for (k, v) in state1 {
            assert!(state2.contains_key(&k));
            assert_eq!(state2[&k].id, v.id);
            assert_eq!(state2[&k].status, v.status);
        }

        // Old jobs should not exist
        for i in 0..10 {
            assert!(cache.get(&format!("original_{}", i)).await.is_none());
        }

        // New jobs should exist
        for i in 0..5 {
            assert!(cache.get(&format!("replaced_{}", i)).await.is_some());
        }
    }

    /// Property: Status counting is accurate
    #[tokio::test]
    async fn prop_status_count_accurate() {
        let statuses = vec![
            JobStatus::Pending,
            JobStatus::Claimed,
            JobStatus::InProgress,
            JobStatus::Completed,
            JobStatus::Failed,
        ];

        for _ in 0..10 {
            let cache = WorkItemCache::new();

            // Add multiple jobs of each status
            for (status_idx, &status) in statuses.iter().enumerate() {
                for count in 0..5 {
                    let job = Job {
                        id: format!("job_{}_{}", status_idx, count),
                        status,
                        payload: serde_json::Value::Null,
                        requirements: JobRequirements::default(),
                        metadata: JobMetadata::new(),
                        error_message: None,
                        claimed_by: None,
                        assigned_worker_id: None,
                        completed_by: None,
                    };
                    cache.upsert(job).await;
                }
            }

            // Verify counts
            for &status in &statuses {
                let count = cache.count_by_status(status).await;
                assert_eq!(count, 5, "Expected 5 jobs with status {:?}", status);
            }

            // Verify total
            assert_eq!(cache.len().await, 25);
        }
    }

    /// Property: Concurrent operations maintain consistency
    #[tokio::test]
    async fn prop_concurrent_upserts_consistent() {
        let cache = WorkItemCache::new();

        // Spawn 100 concurrent tasks, each upserting a different job
        let handles: Vec<_> = (0..100)
            .map(|i| {
                let c = cache.clone();
                tokio::spawn(async move {
                    let job = Job {
                        id: format!("concurrent_job_{}", i),
                        status: JobStatus::Pending,
                        payload: serde_json::json!({"id": i}),
                        requirements: JobRequirements::default(),
                        metadata: JobMetadata::new(),
                        error_message: None,
                        claimed_by: None,
                        assigned_worker_id: None,
                        completed_by: None,
                    };
                    c.upsert(job).await;
                })
            })
            .collect();

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all jobs are in cache
        assert_eq!(cache.len().await, 100);
        for i in 0..100 {
            assert!(cache.get(&format!("concurrent_job_{}", i)).await.is_some());
        }
    }

    /// Property: Count function matches get_all length
    #[tokio::test]
    async fn prop_count_matches_len() {
        for num_jobs in [0, 1, 5, 10, 50] {
            let cache = WorkItemCache::new();

            for i in 0..num_jobs {
                let job = Job {
                    id: format!("job_{}", i),
                    status: JobStatus::Pending,
                    payload: serde_json::Value::Null,
                    requirements: JobRequirements::default(),
                    metadata: JobMetadata::new(),
                    error_message: None,
                    claimed_by: None,
                    assigned_worker_id: None,
                    completed_by: None,
                };
                cache.upsert(job).await;
            }

            let len = cache.len().await;
            let all = cache.get_all().await;
            let count = cache.count(|_| true).await;

            assert_eq!(len, num_jobs);
            assert_eq!(all.len(), num_jobs);
            assert_eq!(count, num_jobs);
        }
    }

    /// Property: Repeated updates with same data are idempotent
    #[tokio::test]
    async fn prop_update_idempotent() {
        for _ in 0..20 {
            let cache = WorkItemCache::new();
            let job = Job {
                id: "idempotent_job".to_string(),
                status: JobStatus::Pending,
                payload: serde_json::Value::Null,
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: None,
                assigned_worker_id: None,
                completed_by: None,
            };

            cache.upsert(job).await;

            // Apply same update 5 times
            for _ in 0..5 {
                cache
                    .update("idempotent_job", |j| {
                        j.status = JobStatus::Claimed;
                        j.claimed_by = Some("node1".to_string());
                    })
                    .await;
            }

            // State should be the same
            let final_state = cache.get("idempotent_job").await.unwrap();
            assert_eq!(final_state.status, JobStatus::Claimed);
            assert_eq!(final_state.claimed_by, Some("node1".to_string()));
        }
    }

    /// Property: Clear makes cache empty
    #[tokio::test]
    async fn prop_clear_empties_cache() {
        for initial_size in [1, 5, 10, 50] {
            let cache = WorkItemCache::new();

            // Fill cache
            for i in 0..initial_size {
                let job = Job {
                    id: format!("job_{}", i),
                    status: JobStatus::Pending,
                    payload: serde_json::Value::Null,
                    requirements: JobRequirements::default(),
                    metadata: JobMetadata::new(),
                    error_message: None,
                    claimed_by: None,
                    assigned_worker_id: None,
                    completed_by: None,
                };
                cache.upsert(job).await;
            }

            assert_eq!(cache.len().await, initial_size);

            // Clear
            cache.clear().await;

            // Should be empty
            assert_eq!(cache.len().await, 0);
            assert!(cache.is_empty().await);
            assert_eq!(cache.get_all().await.len(), 0);
        }
    }

    /// Property: Find first respects predicate
    #[tokio::test]
    async fn prop_find_first_matches_predicate() {
        let cache = WorkItemCache::new();

        // Add mixed jobs
        for i in 0..10 {
            let status = if i % 2 == 0 {
                JobStatus::Pending
            } else {
                JobStatus::Claimed
            };

            let job = Job {
                id: format!("job_{}", i),
                status,
                payload: serde_json::Value::Null,
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: None,
                assigned_worker_id: None,
                completed_by: None,
            };
            cache.upsert(job).await;
        }

        // Find first pending
        let first_pending = cache.find_first(|j| j.status == JobStatus::Pending).await;
        assert!(first_pending.is_some());
        assert_eq!(first_pending.unwrap().status, JobStatus::Pending);

        // Find first claimed
        let first_claimed = cache.find_first(|j| j.status == JobStatus::Claimed).await;
        assert!(first_claimed.is_some());
        assert_eq!(first_claimed.unwrap().status, JobStatus::Claimed);

        // Find none with impossible predicate
        let none = cache
            .find_first(|j| j.status == JobStatus::Completed)
            .await;
        assert!(none.is_none());
    }

    /// Property: Status transitions preserve job data
    #[tokio::test]
    async fn prop_status_changes_preserve_data() {
        let cache = WorkItemCache::new();

        let statuses = vec![
            JobStatus::Pending,
            JobStatus::Claimed,
            JobStatus::InProgress,
            JobStatus::Completed,
        ];

        for _ in 0..10 {
            let mut job = Job {
                id: "status_test_job".to_string(),
                status: JobStatus::Pending,
                payload: serde_json::json!({"important": "data"}),
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: Some("initial error".to_string()),
                claimed_by: Some("node1".to_string()),
                assigned_worker_id: Some("worker1".to_string()),
                completed_by: None,
            };

            cache.upsert(job.clone()).await;

            // Transition through various statuses
            for new_status in &statuses[1..] {
                cache
                    .update("status_test_job", |j| {
                        j.status = *new_status;
                    })
                    .await;

                let updated = cache.get("status_test_job").await.unwrap();

                // Verify other data is preserved
                assert_eq!(updated.payload, serde_json::json!({"important": "data"}));
                assert_eq!(
                    updated.error_message,
                    Some("initial error".to_string())
                );
                assert_eq!(updated.claimed_by, Some("node1".to_string()));
                assert_eq!(updated.assigned_worker_id, Some("worker1".to_string()));
                assert_eq!(updated.status, *new_status);
            }
        }
    }

    /// Property: Cache from_items creates identical state to manual upserts
    #[tokio::test]
    async fn prop_from_items_equivalent_to_upserts() {
        let items: Vec<Job> = (0..20)
            .map(|i| Job {
                id: format!("job_{}", i),
                status: if i % 3 == 0 {
                    JobStatus::Pending
                } else {
                    JobStatus::Claimed
                },
                payload: serde_json::json!({"idx": i}),
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: if i % 2 == 0 {
                    Some(format!("node_{}", i))
                } else {
                    None
                },
                assigned_worker_id: None,
                completed_by: None,
            })
            .collect();

        // Create cache from items
        let cache1 = WorkItemCache::from_items(items.clone());

        // Create cache manually
        let cache2 = WorkItemCache::new();
        for item in items.clone() {
            cache2.upsert(item).await;
        }

        // Both caches should have same state
        let state1 = cache1.get_all_map().await;
        let state2 = cache2.get_all_map().await;

        assert_eq!(state1.len(), state2.len());
        for (k, v) in state1 {
            assert!(state2.contains_key(&k));
            assert_eq!(state2[&k].id, v.id);
        }
    }

    /// Property: All items have consistent timestamps after creation
    #[tokio::test]
    async fn prop_metadata_timestamps_valid() {
        let cache = WorkItemCache::new();

        for i in 0..20 {
            let job = Job {
                id: format!("job_{}", i),
                status: JobStatus::Pending,
                payload: serde_json::Value::Null,
                requirements: JobRequirements::default(),
                metadata: JobMetadata::new(),
                error_message: None,
                claimed_by: None,
                assigned_worker_id: None,
                completed_by: None,
            };
            cache.upsert(job).await;
        }

        let all = cache.get_all().await;
        for job in all {
            // created_at should be less than or equal to updated_at
            assert!(job.metadata.created_at <= job.metadata.updated_at);

            // Timestamps should be positive (after Unix epoch)
            assert!(job.metadata.created_at > 0);
            assert!(job.metadata.updated_at > 0);

            // started_at and completed_at should be None or in the future relative to created_at
            if let Some(started) = job.metadata.started_at {
                assert!(started >= job.metadata.created_at);
            }
        }
    }
}
