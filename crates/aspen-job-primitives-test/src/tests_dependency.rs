//! Dependency DAG execution test.
//!
//! Scenario 1: Diamond DAG (A → B, A → C, B+C → D) completes in order.
//! Scenario 2: Upstream failure (B fails) blocks downstream (D).

use aspen_jobs::DependencyFailurePolicy;
use aspen_jobs::DependencyGraph;
use aspen_jobs::JobId;

use crate::TestResult;

pub async fn run() -> TestResult {
    test_diamond_dag_completes_in_order().await?;
    test_upstream_failure_blocks_downstream().await?;
    Ok(())
}

/// Diamond DAG: A has no deps, B and C depend on A, D depends on B and C.
/// Execute in topological order, verify D runs last.
async fn test_diamond_dag_completes_in_order() -> TestResult {
    let graph = DependencyGraph::new();

    let job_a = JobId::from_string("job-a".to_string());
    let job_b = JobId::from_string("job-b".to_string());
    let job_c = JobId::from_string("job-c".to_string());
    let job_d = JobId::from_string("job-d".to_string());

    // A has no dependencies
    graph
        .add_job(job_a.clone(), vec![], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job A: {e}"))?;

    // B depends on A
    graph
        .add_job(job_b.clone(), vec![job_a.clone()], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job B: {e}"))?;

    // C depends on A
    graph
        .add_job(job_c.clone(), vec![job_a.clone()], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job C: {e}"))?;

    // D depends on B and C
    graph
        .add_job(job_d.clone(), vec![job_b.clone(), job_c.clone()], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job D: {e}"))?;

    // Initially only A should be ready
    let ready = graph.get_all_ready().await;
    if ready.len() != 1 || ready[0] != job_a {
        return Err(format!(
            "expected only job-a ready, got: {:?}",
            ready.iter().map(|j| j.as_str()).collect::<Vec<_>>()
        ));
    }

    // Complete A — B and C should become ready
    graph.mark_running(&job_a).await.map_err(|e| format!("mark_running A: {e}"))?;
    let newly_ready = graph.mark_completed(&job_a).await.map_err(|e| format!("mark_completed A: {e}"))?;

    let mut ready_names: Vec<&str> = newly_ready.iter().map(|j| j.as_str()).collect();
    ready_names.sort();
    if ready_names != vec!["job-b", "job-c"] {
        return Err(format!("after completing A, expected B and C ready, got: {ready_names:?}"));
    }

    // D should NOT be ready yet
    let d_ready = graph.is_ready(&job_d).await.map_err(|e| format!("is_ready D: {e}"))?;
    if d_ready {
        return Err("D should not be ready before B and C complete".to_string());
    }

    // Complete B
    graph.mark_running(&job_b).await.map_err(|e| format!("mark_running B: {e}"))?;
    graph.mark_completed(&job_b).await.map_err(|e| format!("mark_completed B: {e}"))?;

    // D still not ready (C pending)
    let d_ready = graph.is_ready(&job_d).await.map_err(|e| format!("is_ready D after B: {e}"))?;
    if d_ready {
        return Err("D should not be ready before C completes".to_string());
    }

    // Complete C — D should become ready
    graph.mark_running(&job_c).await.map_err(|e| format!("mark_running C: {e}"))?;
    let newly_ready = graph.mark_completed(&job_c).await.map_err(|e| format!("mark_completed C: {e}"))?;

    if newly_ready.len() != 1 || newly_ready[0] != job_d {
        return Err(format!(
            "after completing C, expected D ready, got: {:?}",
            newly_ready.iter().map(|j| j.as_str()).collect::<Vec<_>>()
        ));
    }

    // Verify the dependency chain: D is blocked by B and C
    let d_chain = graph.get_dependency_chain(&job_d).await.map_err(|e| format!("get_dependency_chain D: {e}"))?;
    let chain_names: Vec<&str> = d_chain.iter().map(|j| j.as_str()).collect();

    // The chain should include B and C as direct dependencies of D
    if !chain_names.contains(&"job-b") || !chain_names.contains(&"job-c") {
        return Err(format!("expected B and C in D's dependency chain, got: {chain_names:?}"));
    }

    println!("  diamond DAG: ordering and dependency chain verified ({chain_names:?})");
    Ok(())
}

/// B fails → D (which depends on B and C) should be blocked.
async fn test_upstream_failure_blocks_downstream() -> TestResult {
    let graph = DependencyGraph::new();

    let job_a = JobId::from_string("job-a".to_string());
    let job_b = JobId::from_string("job-b".to_string());
    let job_c = JobId::from_string("job-c".to_string());
    let job_d = JobId::from_string("job-d".to_string());

    graph
        .add_job(job_a.clone(), vec![], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job A: {e}"))?;
    graph
        .add_job(job_b.clone(), vec![job_a.clone()], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job B: {e}"))?;
    graph
        .add_job(job_c.clone(), vec![job_a.clone()], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job C: {e}"))?;
    graph
        .add_job(job_d.clone(), vec![job_b.clone(), job_c.clone()], DependencyFailurePolicy::FailCascade)
        .await
        .map_err(|e| format!("add_job D: {e}"))?;

    // Complete A
    graph.mark_running(&job_a).await.map_err(|e| format!("mark_running A: {e}"))?;
    graph.mark_completed(&job_a).await.map_err(|e| format!("mark_completed A: {e}"))?;

    // Fail B — with FailCascade, D should be blocked
    graph.mark_running(&job_b).await.map_err(|e| format!("mark_running B: {e}"))?;
    let cascaded = graph
        .mark_failed(&job_b, "intentional failure".to_string())
        .await
        .map_err(|e| format!("mark_failed B: {e}"))?;

    // D should appear in the cascaded failures
    if !cascaded.contains(&job_d) {
        return Err(format!(
            "D should be in cascaded failures after B fails, got: {:?}",
            cascaded.iter().map(|j| j.as_str()).collect::<Vec<_>>()
        ));
    }

    // Verify D is in a terminal/blocked state
    let d_info = graph.get_job_info(&job_d).await.ok_or("D not found in graph")?;

    if d_info.state.is_ready() {
        return Err("D should NOT be ready after B failed (FailCascade policy)".to_string());
    }

    println!("  upstream failure: D correctly blocked after B failed");
    Ok(())
}
