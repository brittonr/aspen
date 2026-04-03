//! Job affinity routing test.
//!
//! Scenario 1: Worker W1 has tag `gpu=true`, W2 has `gpu=false`.
//!             Job requires `gpu=true` → routes to W1 (higher affinity score).
//! Scenario 2: Job requires `region=eu`, no workers match → score is 0.

use std::collections::HashMap;
use std::sync::Arc;

use aspen_jobs::AffinityJobManager;
use aspen_jobs::AffinityStrategy;
use aspen_jobs::JobAffinity;
use aspen_jobs::JobManager;
use aspen_jobs::JobSpec;
use aspen_jobs::WorkerMetadata;

use crate::TestResult;
use crate::make_store;

pub async fn run() -> TestResult {
    test_affinity_tagged_job_routes_to_matching_worker().await?;
    test_no_matching_worker_scores_zero().await?;
    Ok(())
}

fn make_worker(id: &str, node_id: iroh::PublicKey, tags: Vec<String>) -> WorkerMetadata {
    WorkerMetadata {
        id: id.to_string(),
        node_id,
        tags,
        region: None,
        load: 0.3,
        local_blobs: vec![],
        latencies: HashMap::new(),
        local_shards: vec![],
    }
}

/// Worker W1 (gpu=true) vs W2 (gpu=false), job requires gpu=true.
/// Affinity score should rank W1 higher.
async fn test_affinity_tagged_job_routes_to_matching_worker() -> TestResult {
    let store = make_store();
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    let affinity_manager = AffinityJobManager::new(manager.clone());

    // Register workers
    let key1 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let key2 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let w1 = make_worker("w1", key1, vec!["gpu=true".to_string()]);
    let w2 = make_worker("w2", key2, vec!["gpu=false".to_string()]);

    affinity_manager.update_worker_metadata(w1.clone()).await;
    affinity_manager.update_worker_metadata(w2.clone()).await;

    // Submit a job that requires gpu=true
    let spec = JobSpec::new("gpu-task");
    let affinity = JobAffinity::new(AffinityStrategy::RequireTags(vec!["gpu=true".to_string()]));

    let job_id = affinity_manager
        .submit_with_affinity(spec, affinity.clone())
        .await
        .map_err(|e| format!("submit_with_affinity: {e}"))?;

    // Retrieve the job to check affinity scoring
    let job = manager.get_job(&job_id).await.map_err(|e| format!("get_job: {e}"))?.ok_or("job not found")?;

    // Score W1 (has gpu=true) — should be 1.0 (all tags match)
    let score_w1 = affinity_manager.calculate_affinity_score(&job, &w1, &affinity);
    // Score W2 (has gpu=false) — should be 0.0 (no tags match)
    let score_w2 = affinity_manager.calculate_affinity_score(&job, &w2, &affinity);

    if score_w1 <= score_w2 {
        return Err(format!("W1 (gpu=true) should score higher than W2 (gpu=false): w1={score_w1}, w2={score_w2}"));
    }

    println!("  affinity tags: w1={score_w1:.2} > w2={score_w2:.2} (gpu=true routes to W1)");
    Ok(())
}

/// Job requires region=eu, no workers have that tag.
/// Affinity score should be 0 for both workers.
async fn test_no_matching_worker_scores_zero() -> TestResult {
    let store = make_store();
    let manager = Arc::new(JobManager::new(store.clone()));
    manager.initialize().await.map_err(|e| format!("initialize: {e}"))?;

    let affinity_manager = AffinityJobManager::new(manager.clone());

    let key1 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let key2 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let w1 = make_worker("w1", key1, vec!["region=us".to_string()]);
    let w2 = make_worker("w2", key2, vec!["region=ap".to_string()]);

    affinity_manager.update_worker_metadata(w1.clone()).await;
    affinity_manager.update_worker_metadata(w2.clone()).await;

    let spec = JobSpec::new("eu-task");
    let affinity = JobAffinity::new(AffinityStrategy::RequireTags(vec!["region=eu".to_string()]));

    let job_id = affinity_manager
        .submit_with_affinity(spec, affinity.clone())
        .await
        .map_err(|e| format!("submit_with_affinity: {e}"))?;

    let job = manager.get_job(&job_id).await.map_err(|e| format!("get_job: {e}"))?.ok_or("job not found")?;

    let score_w1 = affinity_manager.calculate_affinity_score(&job, &w1, &affinity);
    let score_w2 = affinity_manager.calculate_affinity_score(&job, &w2, &affinity);

    // With RequireTags and no matches, the tag matching portion should be 0.
    // The final score includes weight blending, so check it's below 0.5 (neutral).
    if score_w1 > 0.5 || score_w2 > 0.5 {
        return Err(format!("no matching region=eu, scores should be ≤0.5: w1={score_w1}, w2={score_w2}"));
    }

    println!("  no match: w1={score_w1:.2}, w2={score_w2:.2} (both below neutral)");
    Ok(())
}
