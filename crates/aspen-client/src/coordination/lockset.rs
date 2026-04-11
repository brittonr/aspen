//! Distributed lock-set client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::LockSetMemberTokenWire;
use aspen_client_api::LockSetResultResponse;

use super::CoordinationRpc;

/// Actionable conflict details for a blocked lock-set acquisition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockSetBlocked {
    blocked_member: Option<String>,
    blocked_holder: Option<String>,
    error: Option<String>,
}

impl LockSetBlocked {
    fn from_result(result: &LockSetResultResponse) -> Self {
        Self {
            blocked_member: result.blocked_member.clone(),
            blocked_holder: result.blocked_holder.clone(),
            error: result.error.clone(),
        }
    }

    /// Canonical member that blocked the acquire.
    pub fn blocked_member(&self) -> Option<&str> {
        self.blocked_member.as_deref()
    }

    /// Holder currently blocking the acquire.
    pub fn blocked_holder(&self) -> Option<&str> {
        self.blocked_holder.as_deref()
    }

    /// Server-provided error message.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Non-blocking lock-set acquire result.
pub enum LockSetTryAcquireResult<C: CoordinationRpc> {
    /// Full set acquired.
    Acquired(RemoteLockSetGuard<C>),
    /// Acquire failed because the set is currently blocked.
    Blocked(LockSetBlocked),
}

impl<C: CoordinationRpc> LockSetTryAcquireResult<C> {
    /// Returns true when the full set was acquired.
    pub fn is_acquired(&self) -> bool {
        matches!(self, Self::Acquired(_))
    }

    /// Returns true when another live holder blocked the set.
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked(_))
    }
}

/// Client for distributed lock-set operations.
pub struct LockSetClient<C: CoordinationRpc> {
    client: Arc<C>,
}

impl<C: CoordinationRpc> LockSetClient<C> {
    /// Create a new lock-set client.
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }

    /// Acquire a canonicalized lock set with retries.
    pub async fn acquire(
        &self,
        members: Vec<String>,
        holder_id: impl Into<String>,
        ttl: Duration,
        timeout: Duration,
    ) -> Result<RemoteLockSetGuard<C>> {
        let holder_id = holder_id.into();
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockSetAcquire {
                members,
                holder_id: holder_id.clone(),
                ttl_ms: ttl.as_millis() as u64,
                timeout_ms: timeout.as_millis() as u64,
            })
            .await?;
        Self::guard_from_response(self.client.clone(), holder_id, response, "LockSetAcquire")
    }

    /// Try to acquire a canonicalized lock set without blocking.
    pub async fn try_acquire(
        &self,
        members: Vec<String>,
        holder_id: impl Into<String>,
        ttl: Duration,
    ) -> Result<LockSetTryAcquireResult<C>> {
        let holder_id = holder_id.into();
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockSetTryAcquire {
                members,
                holder_id: holder_id.clone(),
                ttl_ms: ttl.as_millis() as u64,
            })
            .await?;
        match response {
            ClientRpcResponse::LockSetResult(result) => {
                if result.is_success {
                    Ok(LockSetTryAcquireResult::Acquired(Self::guard_from_result(
                        self.client.clone(),
                        holder_id,
                        result,
                    )?))
                } else if is_blocked_result(&result) {
                    Ok(LockSetTryAcquireResult::Blocked(LockSetBlocked::from_result(&result)))
                } else {
                    bail!("lock-set try_acquire failed: {}", format_lockset_error(&result));
                }
            }
            _ => bail!("unexpected response type for LockSetTryAcquire"),
        }
    }

    fn guard_from_response(
        client: Arc<C>,
        holder_id: String,
        response: ClientRpcResponse,
        operation: &str,
    ) -> Result<RemoteLockSetGuard<C>> {
        match response {
            ClientRpcResponse::LockSetResult(result) => Self::guard_from_result(client, holder_id, result),
            _ => bail!("unexpected response type for {operation}"),
        }
    }

    fn guard_from_result(
        client: Arc<C>,
        holder_id: String,
        result: LockSetResultResponse,
    ) -> Result<RemoteLockSetGuard<C>> {
        if !result.is_success {
            bail!("lock-set operation failed: {}", format_lockset_error(&result));
        }
        Ok(RemoteLockSetGuard {
            client,
            holder_id,
            member_tokens: result.member_tokens,
            deadline_ms: result.deadline_ms.unwrap_or(0),
        })
    }
}

fn is_blocked_result(result: &LockSetResultResponse) -> bool {
    result.blocked_member.is_some() && result.blocked_holder.is_some()
}

fn format_lockset_error(result: &LockSetResultResponse) -> String {
    let base_error = result.error.clone().unwrap_or_else(|| "unknown error".to_string());
    match (&result.blocked_member, &result.blocked_holder) {
        (Some(member), Some(holder)) => format!("{base_error}: blocked on '{member}' held by '{holder}'"),
        (Some(member), None) => format!("{base_error}: blocked on '{member}'"),
        _ => base_error,
    }
}

/// Guard for a remotely held distributed lock set.
pub struct RemoteLockSetGuard<C: CoordinationRpc> {
    client: Arc<C>,
    holder_id: String,
    member_tokens: Vec<LockSetMemberTokenWire>,
    deadline_ms: u64,
}

impl<C: CoordinationRpc> RemoteLockSetGuard<C> {
    /// Canonical member-token list.
    pub fn member_tokens(&self) -> &[LockSetMemberTokenWire] {
        &self.member_tokens
    }

    /// Canonical member names.
    pub fn members(&self) -> Vec<&str> {
        self.member_tokens.iter().map(|token| token.member.as_str()).collect()
    }

    /// Lookup one member fencing token.
    pub fn fencing_token_for(&self, member: &str) -> Option<u64> {
        self.member_tokens.iter().find(|token| token.member == member).map(|token| token.fencing_token)
    }

    /// Shared deadline for the full set.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Release the full set.
    pub async fn release(self) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockSetRelease {
                holder_id: self.holder_id.clone(),
                member_tokens: self.member_tokens.clone(),
            })
            .await?;
        match response {
            ClientRpcResponse::LockSetResult(result) if result.is_success => Ok(()),
            ClientRpcResponse::LockSetResult(result) => {
                bail!("lock-set release failed: {}", format_lockset_error(&result))
            }
            _ => bail!("unexpected response type for LockSetRelease"),
        }
    }

    /// Renew the full set with a new TTL.
    pub async fn renew(&mut self, ttl: Duration) -> Result<()> {
        let response = self
            .client
            .send_coordination_request(ClientRpcRequest::LockSetRenew {
                holder_id: self.holder_id.clone(),
                member_tokens: self.member_tokens.clone(),
                ttl_ms: ttl.as_millis() as u64,
            })
            .await?;
        match response {
            ClientRpcResponse::LockSetResult(result) if result.is_success => {
                self.member_tokens = result.member_tokens;
                if let Some(deadline_ms) = result.deadline_ms {
                    self.deadline_ms = deadline_ms;
                }
                Ok(())
            }
            ClientRpcResponse::LockSetResult(result) => {
                bail!("lock-set renew failed: {}", format_lockset_error(&result))
            }
            _ => bail!("unexpected response type for LockSetRenew"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use anyhow::bail;

    use super::*;

    struct MockRpcClient {
        responses: Mutex<Vec<ClientRpcResponse>>,
    }

    impl MockRpcClient {
        fn new(responses: Vec<ClientRpcResponse>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    impl CoordinationRpc for MockRpcClient {
        async fn send_coordination_request(&self, _request: ClientRpcRequest) -> Result<ClientRpcResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                bail!("no more mock responses");
            }
            Ok(responses.remove(0))
        }
    }

    #[tokio::test]
    async fn test_lockset_client_acquire() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockSetResult(LockSetResultResponse {
            is_success: true,
            holder_id: Some("holder-a".to_string()),
            member_tokens: vec![
                LockSetMemberTokenWire {
                    member: "pipeline:42".to_string(),
                    fencing_token: 3,
                },
                LockSetMemberTokenWire {
                    member: "repo:a".to_string(),
                    fencing_token: 7,
                },
            ],
            deadline_ms: Some(1234),
            blocked_member: None,
            blocked_holder: None,
            error: None,
        })]));

        let lockset = LockSetClient::new(client);
        let guard = lockset
            .acquire(
                vec!["repo:a".to_string(), "pipeline:42".to_string()],
                "holder-a",
                Duration::from_secs(30),
                Duration::from_secs(5),
            )
            .await
            .unwrap();
        assert_eq!(guard.fencing_token_for("repo:a"), Some(7));
        assert_eq!(guard.deadline_ms(), 1234);
    }

    #[tokio::test]
    async fn test_lockset_client_try_acquire_exposes_blocked_details() {
        let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockSetResult(LockSetResultResponse {
            is_success: false,
            holder_id: None,
            member_tokens: vec![],
            deadline_ms: None,
            blocked_member: Some("repo:a".to_string()),
            blocked_holder: Some("holder-b".to_string()),
            error: Some("lock set blocked".to_string()),
        })]));

        let lockset = LockSetClient::new(client);
        let result = lockset
            .try_acquire(vec!["repo:a".to_string(), "pipeline:42".to_string()], "holder-a", Duration::from_secs(30))
            .await
            .unwrap();
        match result {
            LockSetTryAcquireResult::Blocked(blocked) => {
                assert_eq!(blocked.blocked_member(), Some("repo:a"));
                assert_eq!(blocked.blocked_holder(), Some("holder-b"));
                assert_eq!(blocked.error(), Some("lock set blocked"));
            }
            LockSetTryAcquireResult::Acquired(_) => panic!("expected blocked result"),
        }
    }

    #[tokio::test]
    async fn test_lockset_client_try_acquire_rejects_non_contention_errors() {
        let error_cases = [
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: None,
                blocked_holder: None,
                error: Some("duplicate lock-set member 'repo:a'".to_string()),
            },
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: None,
                blocked_holder: None,
                error: Some("lock set has 9 members, max 8".to_string()),
            },
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: None,
                blocked_holder: None,
                error: Some("storage error: backend unavailable".to_string()),
            },
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: Some("repo:a".to_string()),
                blocked_holder: None,
                error: Some("partial contention metadata".to_string()),
            },
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: None,
                blocked_holder: Some("holder-b".to_string()),
                error: Some("partial contention metadata".to_string()),
            },
        ];

        for error_case in error_cases {
            let error_message = error_case.error.clone().unwrap();
            let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockSetResult(error_case)]));
            let lockset = LockSetClient::new(client);

            let result = lockset.try_acquire(vec!["repo:a".to_string()], "holder-a", Duration::from_secs(30)).await;
            match result {
                Ok(LockSetTryAcquireResult::Blocked(_)) => panic!("expected hard error for non-contention failure"),
                Ok(LockSetTryAcquireResult::Acquired(_)) => panic!("expected failure result"),
                Err(error) => assert!(error.to_string().contains(&error_message)),
            }
        }
    }

    #[tokio::test]
    async fn test_lockset_client_try_acquire_rejects_partial_contention_metadata() {
        let partial_cases = [
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: Some("repo:a".to_string()),
                blocked_holder: None,
                error: Some("partial contention metadata".to_string()),
            },
            LockSetResultResponse {
                is_success: false,
                holder_id: None,
                member_tokens: vec![],
                deadline_ms: None,
                blocked_member: None,
                blocked_holder: Some("holder-b".to_string()),
                error: Some("partial contention metadata".to_string()),
            },
        ];

        for partial_case in partial_cases {
            let client = Arc::new(MockRpcClient::new(vec![ClientRpcResponse::LockSetResult(partial_case)]));
            let lockset = LockSetClient::new(client);

            let result = lockset
                .try_acquire(vec!["repo:a".to_string(), "pipeline:42".to_string()], "holder-a", Duration::from_secs(30))
                .await;
            match result {
                Ok(LockSetTryAcquireResult::Blocked(_)) => panic!("partial metadata must not produce blocked result"),
                Ok(LockSetTryAcquireResult::Acquired(_)) => panic!("expected failure result"),
                Err(error) => assert!(error.to_string().contains("partial contention metadata")),
            }
        }
    }

    #[tokio::test]
    async fn test_remote_lockset_guard_release_and_renew() {
        let client = Arc::new(MockRpcClient::new(vec![
            ClientRpcResponse::LockSetResult(LockSetResultResponse {
                is_success: true,
                holder_id: Some("holder-a".to_string()),
                member_tokens: vec![LockSetMemberTokenWire {
                    member: "repo:a".to_string(),
                    fencing_token: 4,
                }],
                deadline_ms: Some(500),
                blocked_member: None,
                blocked_holder: None,
                error: None,
            }),
            ClientRpcResponse::LockSetResult(LockSetResultResponse {
                is_success: true,
                holder_id: Some("holder-a".to_string()),
                member_tokens: vec![LockSetMemberTokenWire {
                    member: "repo:a".to_string(),
                    fencing_token: 4,
                }],
                deadline_ms: Some(900),
                blocked_member: None,
                blocked_holder: None,
                error: None,
            }),
            ClientRpcResponse::LockSetResult(LockSetResultResponse {
                is_success: true,
                holder_id: Some("holder-a".to_string()),
                member_tokens: vec![],
                deadline_ms: Some(0),
                blocked_member: None,
                blocked_holder: None,
                error: None,
            }),
        ]));

        let lockset = LockSetClient::new(client);
        let mut guard = lockset
            .acquire(vec!["repo:a".to_string()], "holder-a", Duration::from_secs(30), Duration::from_secs(5))
            .await
            .unwrap();
        guard.renew(Duration::from_secs(60)).await.unwrap();
        assert_eq!(guard.deadline_ms(), 900);
        guard.release().await.unwrap();
    }
}
