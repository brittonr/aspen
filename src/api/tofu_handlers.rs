//! HTTP handlers for OpenTofu/Terraform state backend protocol
//!
//! Implements the HTTP backend protocol that OpenTofu/Terraform uses for remote state storage.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    state::AppState,
    tofu::types::*,
};

/// Query parameters for state operations
#[derive(Debug, Deserialize)]
pub struct StateQuery {
    #[serde(rename = "ID")]
    lock_id: Option<String>,
}

/// Get the current state for a workspace
///
/// GET /api/tofu/state/{workspace}
pub async fn get_state(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.get_state(&workspace).await {
        Ok(Some(tofu_state)) => {
            // Return the state as JSON
            Ok(Json(tofu_state).into_response())
        }
        Ok(None) => {
            // Return empty state (OpenTofu expects empty body for no state)
            Ok((StatusCode::OK, "").into_response())
        }
        Err(e) => {
            tracing::error!("Failed to get state for workspace {}: {}", workspace, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Update the state for a workspace
///
/// POST /api/tofu/state/{workspace}
pub async fn update_state(
    Path(workspace): Path<String>,
    Query(query): Query<StateQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(tofu_state): Json<TofuState>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    // Check if workspace is locked and verify lock ID if provided
    if let Some(lock_id) = query.lock_id {
        match tofu_service.get_lock_info(&workspace).await {
            Ok(Some(lock)) if lock.id != lock_id => {
                return Err(StatusCode::LOCKED);
            }
            Ok(None) => {
                // Not locked, which is fine
            }
            Ok(_) => {
                // Lock ID matches, proceed
            }
            Err(e) => {
                tracing::error!("Failed to check lock for workspace {}: {}", workspace, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // Get expected version from headers if provided
    let expected_version = headers
        .get("X-Terraform-Version")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());

    match tofu_service.put_state(&workspace, tofu_state, expected_version).await {
        Ok(_) => Ok((StatusCode::OK, "").into_response()),
        Err(e) => {
            if e.to_string().contains("StateVersionMismatch") {
                Err(StatusCode::CONFLICT)
            } else {
                tracing::error!("Failed to update state for workspace {}: {}", workspace, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Lock a workspace
///
/// LOCK /api/tofu/lock/{workspace}
pub async fn lock_workspace(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
    Json(lock_request): Json<LockRequest>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.lock_workspace(&workspace, lock_request).await {
        Ok(_) => Ok((StatusCode::OK, "").into_response()),
        Err(e) => {
            if e.to_string().contains("WorkspaceLocked") {
                // Return the current lock info
                match tofu_service.get_lock_info(&workspace).await {
                    Ok(Some(lock)) => {
                        Ok((StatusCode::LOCKED, Json(lock)).into_response())
                    }
                    _ => Err(StatusCode::LOCKED),
                }
            } else {
                tracing::error!("Failed to lock workspace {}: {}", workspace, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Unlock a workspace
///
/// UNLOCK /api/tofu/unlock/{workspace}
pub async fn unlock_workspace(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
    Json(lock_request): Json<LockRequest>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.unlock_workspace(&workspace, &lock_request.id).await {
        Ok(_) => Ok((StatusCode::OK, "").into_response()),
        Err(e) => {
            if e.to_string().contains("LockNotFound") {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to unlock workspace {}: {}", workspace, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Force unlock a workspace (admin operation)
///
/// DELETE /api/tofu/lock/{workspace}
pub async fn force_unlock_workspace(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.force_unlock_workspace(&workspace).await {
        Ok(_) => Ok((StatusCode::OK, "").into_response()),
        Err(e) => {
            tracing::error!("Failed to force unlock workspace {}: {}", workspace, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List workspaces
///
/// GET /api/tofu/workspaces
pub async fn list_workspaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.list_workspaces().await {
        Ok(workspaces) => Ok(Json(workspaces)),
        Err(e) => {
            tracing::error!("Failed to list workspaces: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete a workspace
///
/// DELETE /api/tofu/workspaces/{workspace}
pub async fn delete_workspace(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.delete_workspace(&workspace).await {
        Ok(_) => Ok((StatusCode::NO_CONTENT, "").into_response()),
        Err(e) => {
            tracing::error!("Failed to delete workspace {}: {}", workspace, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get state history for a workspace
///
/// GET /api/tofu/history/{workspace}
pub async fn get_state_history(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<StateHistoryEntry>>, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.get_state_history(&workspace, Some(50)).await {
        Ok(history) => {
            let entries: Vec<StateHistoryEntry> = history
                .into_iter()
                .map(|(version, _, created_at)| StateHistoryEntry {
                    version,
                    created_at,
                })
                .collect();
            Ok(Json(entries))
        }
        Err(e) => {
            tracing::error!("Failed to get history for workspace {}: {}", workspace, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Serialize)]
pub struct StateHistoryEntry {
    version: i64,
    created_at: i64,
}

/// Rollback to a previous state version
///
/// POST /api/tofu/rollback/{workspace}/{version}
pub async fn rollback_state(
    Path((workspace, version)): Path<(String, i64)>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.rollback_state(&workspace, version).await {
        Ok(_) => Ok((StatusCode::OK, "").into_response()),
        Err(e) => {
            tracing::error!("Failed to rollback workspace {} to version {}: {}", workspace, version, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Plan execution endpoints

/// Create and execute a plan
///
/// POST /api/tofu/plan
#[derive(Deserialize)]
pub struct CreatePlanRequest {
    workspace: String,
    config_dir: String,
    auto_approve: bool,
}

pub async fn create_plan(
    State(state): State<AppState>,
    Json(request): Json<CreatePlanRequest>,
) -> Result<Json<PlanExecutionResult>, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.execute_plan(
        &request.workspace,
        std::path::Path::new(&request.config_dir),
        request.auto_approve,
    ).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("Failed to create plan: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Apply a stored plan
///
/// POST /api/tofu/apply/{plan_id}
#[derive(Deserialize)]
pub struct ApplyPlanRequest {
    approver: String,
}

pub async fn apply_plan(
    Path(plan_id): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<ApplyPlanRequest>,
) -> Result<Json<PlanExecutionResult>, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.apply_stored_plan(&plan_id, &request.approver).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("Failed to apply plan {}: {}", plan_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List plans for a workspace
///
/// GET /api/tofu/plans/{workspace}
pub async fn list_plans(
    Path(workspace): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<PlanSummary>>, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.list_plans(&workspace).await {
        Ok(plans) => {
            let summaries: Vec<PlanSummary> = plans
                .into_iter()
                .map(|p| PlanSummary {
                    id: p.id,
                    workspace: p.workspace,
                    created_at: p.created_at,
                    status: p.status,
                    approved_by: p.approved_by,
                    executed_at: p.executed_at,
                })
                .collect();
            Ok(Json(summaries))
        }
        Err(e) => {
            tracing::error!("Failed to list plans for workspace {}: {}", workspace, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Serialize)]
pub struct PlanSummary {
    id: String,
    workspace: String,
    created_at: i64,
    status: PlanStatus,
    approved_by: Option<String>,
    executed_at: Option<i64>,
}

/// Destroy infrastructure
///
/// POST /api/tofu/destroy
#[derive(Deserialize)]
pub struct DestroyRequest {
    workspace: String,
    config_dir: String,
    auto_approve: bool,
}

pub async fn destroy_infrastructure(
    State(state): State<AppState>,
    Json(request): Json<DestroyRequest>,
) -> Result<Json<PlanExecutionResult>, StatusCode> {
    let tofu_service = state.tofu_service();

    match tofu_service.destroy(
        &request.workspace,
        std::path::Path::new(&request.config_dir),
        request.auto_approve,
    ).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("Failed to destroy infrastructure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}