use std::time::Duration;

use flawless::{
    serde::{Deserialize, Serialize},
    serde_json,
    workflow,
    workflow::{Input, sleep},
};
use log::info;

flawless::module! { name = "crawler", version = "0.1.8" }

const MAX_ECHO_COUNT: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: usize,
    pub url: String,
}

#[workflow("start")]
pub fn start_crawler(input: Input<Job>) {
    let id = input.id;
    let message = input.url.clone();

    info!("Starting echo workflow for job {}: '{}'", id, message);

    for count in 1..=MAX_ECHO_COUNT {
        info!("Echo iteration {}/{} for job {}", count, MAX_ECHO_COUNT, id);

        sleep(Duration::from_millis(300));
        info!("Job {}: Processing message '{}'", id, message);
        update_ui(UpdateUI::new(id, Status::Request, &message, MAX_ECHO_COUNT - count));
        sleep(Duration::from_millis(300));

        info!("Job {}: Parsing (iteration {})", id, count);
        update_ui(UpdateUI::new(id, Status::Parse, "last", MAX_ECHO_COUNT - count));
        sleep(Duration::from_millis(300));

        info!("Job {}: Completed iteration {}/{}", id, count, MAX_ECHO_COUNT);
        update_ui(UpdateUI::new(id, Status::Done, "last", MAX_ECHO_COUNT - count));
    }

    info!("Finished echo workflow for job {}", id);
}

// Send update to UI server.
fn update_ui(update: UpdateUI) {
    info!("Sending UI update: status={:?}, url={}, urls_left={}",
          update.status, update.url, update.urls_left);
    flawless_http::post("http://localhost:3020/ui-update")
        .body(serde_json::to_value(update).expect("UpdateUI serialization"))
        .send()
        .expect("UI update");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUI {
    pub id: usize,
    pub status: Status,
    pub url: String,
    pub urls_left: usize,
}

impl UpdateUI {
    fn new(id: usize, status: Status, url: &str, urls_left: usize) -> Self {
        UpdateUI {
            id,
            status,
            url: url.to_string(),
            urls_left,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    // Request in progress.
    Request,
    // Parsing in progress.
    Parse,
    // Finished.
    Done,
    // Error
    Error,
}
