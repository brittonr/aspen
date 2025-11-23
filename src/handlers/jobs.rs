//! Job management UI handlers (legacy UI, not dashboard)

use std::time::Duration;
use askama::Template;
use axum::{
    Form, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
};
use serde::Deserialize;

use crate::state::{AppState, JobProgress, Status, UpdateUI};
use crate::work_queue;

/// Index page template
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

/// Index page handler
pub async fn index() -> impl IntoResponse {
    Html(IndexTemplate.render().expect("index renders"))
}

/// Job submission form data
#[derive(Deserialize)]
pub struct NewJob {
    pub url: String,
}

/// Job submission handler (legacy UI)
pub async fn new_job(State(state): State<AppState>, Form(job): Form<NewJob>) -> impl IntoResponse {
    let url = job.url;
    let id = state.add_job(url.clone()).await;
    let job_id = format!("job-{}", id);

    // Publish work to hiqlite-backed queue
    let payload = serde_json::json!({
        "id": id,
        "url": url,
    });

    state
        .work_queue()
        .publish_work(job_id.clone(), payload)
        .await
        .expect("Failed to publish work to hiqlite");

    // Job is now in the queue - background workers will claim and process it
    tracing::info!(job_id = %job_id, url = %url, "Job published to distributed queue");

    list(State(state)).await
}

/// Job list template
#[derive(Template)]
#[template(path = "list.html")]
struct ListTemplate {
    progress: Vec<JobProgress>,
}

/// Job list handler
pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    let progress = state.progress().await;
    // Display max 10 last requests.
    let progress = progress
        .iter()
        .map(|job| {
            let url = job.url.clone();
            let list = job.list.iter().rev().take(10).map(|e| e.clone()).collect();
            JobProgress { url, list }
        })
        .collect();
    let template = ListTemplate { progress };
    Html(template.render().expect("index renders"))
}

/// UI update handler (called by workflows)
pub async fn ui_update(
    State(state): State<AppState>,
    Json(ui_update): Json<UpdateUI>,
) -> impl IntoResponse {
    // Update local UI state
    state
        .update_job(ui_update.id, ui_update.status.clone(), ui_update.url.clone())
        .await;

    // Sync to hiqlite as the single source of truth
    let job_id = format!("job-{}", ui_update.id);
    let work_status = match ui_update.status {
        Status::Request | Status::Parse => work_queue::WorkStatus::InProgress,
        Status::Done => work_queue::WorkStatus::Completed,
        Status::Error => work_queue::WorkStatus::Failed,
    };

    // Update hiqlite workflow state
    if let Err(e) = state.work_queue().update_status(&job_id, work_status).await {
        tracing::warn!("Failed to sync workflow status to hiqlite: {}", e);
    }

    Html("OK")
}

/// Static CSS asset
static STYLE_CSS: &str = include_str!("../../assets/styles.css");

/// Asset handler for serving static files
pub async fn handle_assets(Path(path): Path<String>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();

    if path == "styles.css" {
        headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
        (StatusCode::OK, headers, STYLE_CSS)
    } else {
        (StatusCode::NOT_FOUND, headers, "")
    }
}

/// Timeout test endpoint (sleeps for 60 seconds)
pub async fn timeout() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_secs(60)).await;
    Html("Response after 60 seconds")
}
