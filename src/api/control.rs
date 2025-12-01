//! Control-plane and key-value shim contracts.
//!
//! These traits are consumed by the HTTP handlers inside `aspen-node`. The
//! in-memory implementation keeps the existing stub behavior, but individual
//! methods are now described in terms of the external Raft/DB service so we can
//! swap the backend without touching the API layer.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

/// Node identifier to keep type signatures short.
pub type NodeId = u64;

/// Cluster node description surfaced at the control-plane boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterNode {
    /// Unique Raft node identifier.
    pub id: NodeId,
    /// Address (host:port) where the control-plane can reach the node.
    pub addr: String,
}

impl ClusterNode {
    /// Convenience constructor for tests and scripts.
    pub fn new(id: NodeId, addr: impl Into<String>) -> Self {
        Self {
            id,
            addr: addr.into(),
        }
    }
}

/// Snapshot of the current cluster state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterState {
    /// Known voting members.
    pub members: Vec<NodeId>,
    /// Known learner endpoints.
    pub learners: Vec<ClusterNode>,
    /// Bootstrapped nodes (voters + address metadata).
    pub nodes: Vec<ClusterNode>,
}

/// Contract for management operations that mutate the cluster membership.
#[async_trait]
pub trait ClusterController: Send + Sync {
    /// Initialize the cluster with the provided voters.
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError>;
    /// Add a learner that may be promoted later.
    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError>;
    /// Replace the current voting set with a new membership.
    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError>;
    /// Return the latest state snapshot (used by `/metrics` / diagnostics).
    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError>;
}

/// Request payload for `POST /init`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitRequest {
    /// Nodes that should be bootstrapped as voters.
    pub initial_members: Vec<ClusterNode>,
}

/// Request payload for `POST /add-learner`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLearnerRequest {
    /// Learner to register.
    pub learner: ClusterNode,
}

/// Request payload for `POST /change-membership`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMembershipRequest {
    /// New voting membership.
    pub members: Vec<NodeId>,
}

/// Request payload for `POST /write`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRequest {
    /// Command that should be replicated.
    pub command: WriteCommand,
}

/// Supported write commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WriteCommand {
    /// Set a key/value pair.
    Set {
        /// Key that should be updated.
        key: String,
        /// Value that should be stored.
        value: String,
    },
}

/// Response for successful write operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    /// Command that the backend accepted.
    pub command: WriteCommand,
}

/// Request payload for `POST /read`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadRequest {
    /// Key that should be fetched.
    pub key: String,
}

/// Response for successful read operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    /// Requested key.
    pub key: String,
    /// Value found in the replicated store.
    pub value: String,
}

/// Contract for the key/value data-path.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Apply a command via Raft.
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError>;
    /// Read a key (eventually via a read-index operation).
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError>;
}

/// Errors returned by the cluster controller.
#[derive(Debug, Error)]
pub enum ControlPlaneError {
    /// Caller supplied invalid parameters.
    #[error("invalid request: {reason}")]
    InvalidRequest {
        /// Explanation returned to the caller.
        reason: String,
    },
    /// Backend reported a missing resource.
    #[error("not found: {reason}")]
    NotFound {
        /// Text included in the backend response.
        reason: String,
    },
    /// Backend failed (network/storage/etc).
    #[error("control-plane failure: {reason}")]
    Backend {
        /// Description of the backend failure.
        reason: String,
    },
}

impl ControlPlaneError {
    fn invalid<S: Into<String>>(reason: S) -> Self {
        Self::InvalidRequest {
            reason: reason.into(),
        }
    }

    fn backend<S: Into<String>>(reason: S) -> Self {
        Self::Backend {
            reason: reason.into(),
        }
    }

    fn not_found<S: Into<String>>(reason: S) -> Self {
        Self::NotFound {
            reason: reason.into(),
        }
    }
}

/// Errors returned by the key/value store.
#[derive(Debug, Error)]
pub enum KeyValueStoreError {
    /// Requested key was not found.
    #[error("key not found: {key}")]
    NotFound {
        /// Key the client attempted to read.
        key: String,
    },
    /// Backend failed (network/storage/etc).
    #[error("key-value failure: {reason}")]
    Backend {
        /// Description of the backend failure.
        reason: String,
    },
}

/// Simple in-memory implementation that keeps the existing stub behavior.
#[derive(Debug, Default)]
pub struct InMemoryControlPlane {
    state: Mutex<InMemoryState>,
}

#[derive(Debug, Default)]
struct InMemoryState {
    cluster: ClusterState,
    data: HashMap<String, String>,
}

impl InMemoryControlPlane {
    /// Build a new stub controller.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(InMemoryState::default()),
        }
    }
}

/// HTTP-backed control-plane client that forwards operations to an external Raft/DB shim.
#[derive(Clone)]
pub struct ExternalControlPlane {
    client: Client,
    base_url: Url,
}

const INIT_PATH: &str = "/cluster/init";
const ADD_LEARNER_PATH: &str = "/cluster/add-learner";
const CHANGE_MEMBERSHIP_PATH: &str = "/cluster/change-membership";
const CLUSTER_STATE_PATH: &str = "/cluster/state";
const WRITE_PATH: &str = "/kv/write";
const READ_PATH: &str = "/kv/read";

impl ExternalControlPlane {
    /// Build a new HTTP client targeting the provided base URL.
    pub fn new(base_url: Url, timeout: Duration) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("build reqwest client")?;
        Ok(Self { client, base_url })
    }

    async fn post<B, T>(&self, path: &str, body: &B) -> Result<T, ControlPlaneError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(path)
            .map_err(|err| ControlPlaneError::backend(err.to_string()))?;
        let resp = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|err| ControlPlaneError::backend(err.to_string()))?;
        Self::decode_response(resp).await
    }

    async fn get<T>(&self, path: &str) -> Result<T, ControlPlaneError>
    where
        T: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(path)
            .map_err(|err| ControlPlaneError::backend(err.to_string()))?;
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| ControlPlaneError::backend(err.to_string()))?;
        Self::decode_response(resp).await
    }

    async fn decode_response<T>(resp: Response) -> Result<T, ControlPlaneError>
    where
        T: DeserializeOwned,
    {
        let status = resp.status();
        if status.is_success() {
            resp.json::<T>()
                .await
                .map_err(|err| ControlPlaneError::backend(err.to_string()))
        } else {
            let reason = resp
                .text()
                .await
                .unwrap_or_else(|err| format!("backend response body error: {err}"));
            match status {
                StatusCode::NOT_FOUND => Err(ControlPlaneError::not_found(reason)),
                _ if status.is_client_error() => Err(ControlPlaneError::InvalidRequest { reason }),
                _ => Err(ControlPlaneError::Backend { reason }),
            }
        }
    }
}

#[async_trait]
impl ClusterController for InMemoryControlPlane {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        if request.initial_members.is_empty() {
            return Err(ControlPlaneError::invalid(
                "initial_members must include at least one voter",
            ));
        }
        let mut guard = self.state.lock().await;
        guard.cluster.nodes = request.initial_members.clone();
        guard.cluster.members = request.initial_members.iter().map(|node| node.id).collect();
        Ok(guard.cluster.clone())
    }

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        let mut guard = self.state.lock().await;
        if guard
            .cluster
            .learners
            .iter()
            .any(|learner| learner.id == request.learner.id)
        {
            return Err(ControlPlaneError::invalid("learner already registered"));
        }
        guard.cluster.learners.push(request.learner.clone());
        Ok(guard.cluster.clone())
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        if request.members.is_empty() {
            return Err(ControlPlaneError::invalid(
                "members must include at least one voter",
            ));
        }
        let mut guard = self.state.lock().await;
        guard.cluster.members = request.members.clone();
        Ok(guard.cluster.clone())
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        let guard = self.state.lock().await;
        Ok(guard.cluster.clone())
    }
}

#[async_trait]
impl ClusterController for ExternalControlPlane {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        self.post(INIT_PATH, &request).await
    }

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        self.post(ADD_LEARNER_PATH, &request).await
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        self.post(CHANGE_MEMBERSHIP_PATH, &request).await
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        self.get(CLUSTER_STATE_PATH).await
    }
}

#[async_trait]
impl KeyValueStore for InMemoryControlPlane {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let mut guard = self.state.lock().await;
        match request.command.clone() {
            WriteCommand::Set { key, value } => {
                guard.data.insert(key.clone(), value.clone());
                Ok(WriteResult {
                    command: WriteCommand::Set { key, value },
                })
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let guard = self.state.lock().await;
        match guard.data.get(&request.key) {
            Some(value) => Ok(ReadResult {
                key: request.key,
                value: value.clone(),
            }),
            None => Err(KeyValueStoreError::NotFound { key: request.key }),
        }
    }
}

#[async_trait]
impl KeyValueStore for ExternalControlPlane {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        self.post(WRITE_PATH, &request)
            .await
            .map_err(|err| match err {
                ControlPlaneError::InvalidRequest { reason } => {
                    KeyValueStoreError::Backend { reason }
                }
                ControlPlaneError::NotFound { reason } => KeyValueStoreError::Backend { reason },
                ControlPlaneError::Backend { reason } => KeyValueStoreError::Backend { reason },
            })
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        self.post(READ_PATH, &request)
            .await
            .map_err(|err| match err {
                ControlPlaneError::InvalidRequest { reason } => {
                    KeyValueStoreError::Backend { reason }
                }
                ControlPlaneError::NotFound { .. } => {
                    KeyValueStoreError::NotFound { key: request.key }
                }
                ControlPlaneError::Backend { reason } => KeyValueStoreError::Backend { reason },
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    #[tokio::test]
    async fn init_requires_members() {
        let plane = InMemoryControlPlane::new();
        let err = plane
            .init(InitRequest {
                initial_members: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ControlPlaneError::InvalidRequest { .. }));
    }

    #[tokio::test]
    async fn write_and_read() {
        let plane = InMemoryControlPlane::new();
        plane
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "127.0.0.1:1")],
            })
            .await
            .unwrap();
        plane
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: "foo".into(),
                    value: "bar".into(),
                },
            })
            .await
            .unwrap();
        let read = plane.read(ReadRequest { key: "foo".into() }).await.unwrap();
        assert_eq!(read.value, "bar");
    }

    #[tokio::test]
    async fn external_control_plane_roundtrip() {
        let (base_url, handle) = spawn_external_backend_server().await;
        let client = ExternalControlPlane::new(base_url, Duration::from_secs(5)).unwrap();

        let cluster = client
            .init(InitRequest {
                initial_members: vec![
                    ClusterNode::new(1, "127.0.0.1:21001"),
                    ClusterNode::new(2, "127.0.0.1:21002"),
                ],
            })
            .await
            .unwrap();
        assert_eq!(cluster.members, vec![1, 2]);

        let cluster = client
            .add_learner(AddLearnerRequest {
                learner: ClusterNode::new(3, "127.0.0.1:21003"),
            })
            .await
            .unwrap();
        assert_eq!(cluster.learners.len(), 1);

        let cluster = client
            .change_membership(ChangeMembershipRequest { members: vec![2] })
            .await
            .unwrap();
        assert_eq!(cluster.members, vec![2]);

        let state = client.current_state().await.unwrap();
        assert_eq!(state.members, vec![2]);

        client
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: "foo".into(),
                    value: "bar".into(),
                },
            })
            .await
            .unwrap();
        let read = client
            .read(ReadRequest { key: "foo".into() })
            .await
            .unwrap();
        assert_eq!(read.value, "bar");

        let err = client
            .read(ReadRequest {
                key: "missing".into(),
            })
            .await
            .unwrap_err();
        match err {
            KeyValueStoreError::NotFound { key } => assert_eq!(key, "missing"),
            other => panic!("unexpected error {other:?}"),
        }

        handle.abort();
    }

    async fn spawn_external_backend_server() -> (Url, JoinHandle<()>) {
        #[derive(Clone)]
        struct ExternalState {
            plane: Arc<InMemoryControlPlane>,
        }

        async fn init_handler(
            State(state): State<ExternalState>,
            Json(req): Json<InitRequest>,
        ) -> Result<Json<ClusterState>, (StatusCode, String)> {
            state
                .plane
                .init(req)
                .await
                .map(Json)
                .map_err(|err| (map_control_status(&err), err.to_string()))
        }

        async fn add_learner_handler(
            State(state): State<ExternalState>,
            Json(req): Json<AddLearnerRequest>,
        ) -> Result<Json<ClusterState>, (StatusCode, String)> {
            state
                .plane
                .add_learner(req)
                .await
                .map(Json)
                .map_err(|err| (map_control_status(&err), err.to_string()))
        }

        async fn change_membership_handler(
            State(state): State<ExternalState>,
            Json(req): Json<ChangeMembershipRequest>,
        ) -> Result<Json<ClusterState>, (StatusCode, String)> {
            state
                .plane
                .change_membership(req)
                .await
                .map(Json)
                .map_err(|err| (map_control_status(&err), err.to_string()))
        }

        async fn cluster_state_handler(
            State(state): State<ExternalState>,
        ) -> Result<Json<ClusterState>, (StatusCode, String)> {
            state
                .plane
                .current_state()
                .await
                .map(Json)
                .map_err(|err| (map_control_status(&err), err.to_string()))
        }

        async fn write_handler(
            State(state): State<ExternalState>,
            Json(req): Json<WriteRequest>,
        ) -> Result<Json<WriteResult>, (StatusCode, String)> {
            state
                .plane
                .write(req)
                .await
                .map(Json)
                .map_err(|err| (map_kv_status(&err), err.to_string()))
        }

        async fn read_handler(
            State(state): State<ExternalState>,
            Json(req): Json<ReadRequest>,
        ) -> Result<Json<ReadResult>, (StatusCode, String)> {
            state
                .plane
                .read(req)
                .await
                .map(Json)
                .map_err(|err| (map_kv_status(&err), err.to_string()))
        }

        fn map_control_status(err: &ControlPlaneError) -> StatusCode {
            match err {
                ControlPlaneError::InvalidRequest { .. } => StatusCode::BAD_REQUEST,
                ControlPlaneError::NotFound { .. } => StatusCode::NOT_FOUND,
                ControlPlaneError::Backend { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }

        fn map_kv_status(err: &KeyValueStoreError) -> StatusCode {
            match err {
                KeyValueStoreError::NotFound { .. } => StatusCode::NOT_FOUND,
                KeyValueStoreError::Backend { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }

        let state = ExternalState {
            plane: Arc::new(InMemoryControlPlane::new()),
        };
        let router = Router::new()
            .route("/cluster/init", post(init_handler))
            .route("/cluster/add-learner", post(add_learner_handler))
            .route(
                "/cluster/change-membership",
                post(change_membership_handler),
            )
            .route("/cluster/state", get(cluster_state_handler))
            .route("/kv/write", post(write_handler))
            .route("/kv/read", post(read_handler))
            .with_state(state);
        let port = portpicker::pick_unused_port().expect("port");
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let url = Url::parse(&format!("http://{addr}/")).unwrap();
        (url, handle)
    }
}
