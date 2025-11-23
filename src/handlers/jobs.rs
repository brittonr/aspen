//! Job management UI handlers (legacy UI, not dashboard)

use std::time::Duration;
use askama::Template;
use axum::{
    Form, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::state::{AppState, JobProgress, Status, UpdateUI};
use crate::domain::{JobLifecycleService, JobSubmission};

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
pub async fn new_job(State(state): State<AppState>, Form(job): Form<NewJob>) -> Response {
    let job_service = JobLifecycleService::new(
        state.work_queue().clone(),
        state.clone(),
    );

    let submission = JobSubmission { url: job.url };

    match job_service.submit_job(submission).await {
        Ok(job_id) => {
            tracing::info!(job_id = %job_id, "Job published to distributed queue");
            list(State(state)).await.into_response()
        }
        Err(e) => {
            tracing::error!("Failed to submit job: {}", e);
            Html(format!("<p>Error: {}</p>", e)).into_response()
        }
    }
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
    let job_service = JobLifecycleService::new(
        state.work_queue().clone(),
        state.clone(),
    );

    let job_id = format!("job-{}", ui_update.id);

    if let Err(e) = job_service
        .update_job_status(&job_id, ui_update.status, ui_update.url, ui_update.id)
        .await
    {
        tracing::warn!("Failed to update job status: {}", e);
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
