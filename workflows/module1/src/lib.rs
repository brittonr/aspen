use std::time::Duration;

use flawless::{
    serde::{Deserialize, Serialize},
    serde_json,
    workflow,
    workflow::{Input, sleep},
};
use log::info;

flawless::module! { name = "crawler", version = "0.1.8" }

// ===== Iroh Helper Functions =====

/// Store data as a blob and return its hash
fn store_blob(data: &[u8]) -> String {
    let response = flawless_http::post("http://localhost:3020/iroh/blob/store")
        .body(data.to_vec())
        .send()
        .expect("Failed to store blob");

    let blob_info: serde_json::Value = response.json()
        .expect("Failed to parse blob response");

    blob_info["hash"]
        .as_str()
        .expect("Missing hash in response")
        .to_string()
}

/// Retrieve blob data by hash
fn retrieve_blob(hash: &str) -> Vec<u8> {
    let url = format!("http://localhost:3020/iroh/blob/{}", hash);
    flawless_http::get(&url)
        .send()
        .expect("Failed to retrieve blob")
        .body()
}

/// Join a gossip topic (topic_id should be 64-char hex string)
fn join_gossip_topic(topic_id: &str) {
    let body = serde_json::json!({ "topic_id": topic_id });
    flawless_http::post("http://localhost:3020/iroh/gossip/join")
        .body(body)
        .send()
        .expect("Failed to join gossip topic");
}

/// Broadcast a message to a gossip topic
fn broadcast_to_topic(topic_id: &str, message: &str) {
    let body = serde_json::json!({
        "topic_id": topic_id,
        "message": message
    });
    flawless_http::post("http://localhost:3020/iroh/gossip/broadcast")
        .body(body)
        .send()
        .expect("Failed to broadcast message");
}

/// Connect to a peer by node address
fn connect_to_peer(node_addr: &str) {
    let body = serde_json::json!({ "node_addr": node_addr });
    flawless_http::post("http://localhost:3020/iroh/connect")
        .body(body)
        .send()
        .expect("Failed to connect to peer");
}

/// Get iroh node information
fn get_node_info() -> (String, Vec<String>) {
    let response = flawless_http::get("http://localhost:3020/iroh/info")
        .send()
        .expect("Failed to get node info");

    let info: serde_json::Value = response.json()
        .expect("Failed to parse node info");

    let node_id = info["node_id"]
        .as_str()
        .expect("Missing node_id")
        .to_string();

    let addresses = info["addresses"]
        .as_array()
        .expect("Missing addresses")
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    (node_id, addresses)
}

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
