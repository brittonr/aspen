//! Federation orchestration for dogfood build parity.

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_cluster::ticket::parse_ticket_to_addrs;
use iroh::EndpointAddr;
use iroh::TransportAddr;
use tracing::info;

use crate::RunConfig;
use crate::error::DogfoodResult;
use crate::error::FederationSnafu;
use crate::state::DogfoodState;

const SOURCE_REPO_NAME: &str = "aspen";
const FEDERATION_MODE_PUBLIC: &str = "public";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FederationPeer {
    node_id: String,
    addr_hint: String,
}

pub async fn prepare_build(config: &RunConfig, state: &DogfoodState) -> DogfoodResult<String> {
    let alice_ticket = state.primary_ticket();
    let bob_ticket = state.bob_ticket();
    let alice_repo_id = crate::forge::lookup_repo_id(alice_ticket, SOURCE_REPO_NAME).await?.ok_or_else(|| {
        crate::error::DogfoodError::Federation {
            operation: "find source repo".to_string(),
            reason: format!("repo '{SOURCE_REPO_NAME}' not found on alice; run `push` first"),
        }
    })?;
    let peer = peer_from_ticket(alice_ticket)?;
    let mirror_name = mirror_repo_name(SOURCE_REPO_NAME);
    let bob_repo_id = crate::forge::ensure_repo_exists(bob_ticket, &mirror_name).await?;
    let clone_dir = clone_dir(config);

    federate_repo(alice_ticket, &alice_repo_id).await?;
    sync_repo(bob_ticket, &peer).await?;
    crate::forge::watch_repo(bob_ticket, &bob_repo_id).await?;

    remove_clone_dir(&clone_dir).await;
    let result = async {
        clone_repo(config, &clone_dir, bob_ticket, &peer, &alice_repo_id).await?;
        push_clone(config, &clone_dir, bob_ticket, &bob_repo_id).await
    }
    .await;
    remove_clone_dir(&clone_dir).await;
    result?;

    Ok(mirror_name)
}

async fn federate_repo(ticket: &str, repo_id: &str) -> DogfoodResult<()> {
    let client = connect(ticket).await?;
    let result = async {
        let response = send(
            &client,
            ClientRpcRequest::FederateRepository {
                repo_id: repo_id.to_string(),
                mode: FEDERATION_MODE_PUBLIC.to_string(),
            },
            "FederateRepository",
            ticket,
        )
        .await?;

        match response {
            ClientRpcResponse::FederateRepositoryResult(result) if result.is_success => Ok(()),
            ClientRpcResponse::FederateRepositoryResult(result) => FederationSnafu {
                operation: "federate repo",
                reason: result.error.unwrap_or_else(|| "unknown error".to_string()),
            }
            .fail(),
            ClientRpcResponse::Error(error) => FederationSnafu {
                operation: "federate repo",
                reason: format!("{}: {}", error.code, error.message),
            }
            .fail(),
            other => FederationSnafu {
                operation: "federate repo",
                reason: format!("unexpected response: {other:?}"),
            }
            .fail(),
        }
    }
    .await;
    client.shutdown().await;
    result
}

async fn sync_repo(ticket: &str, peer: &FederationPeer) -> DogfoodResult<()> {
    let client = connect(ticket).await?;
    let result = async {
        let response = send(
            &client,
            ClientRpcRequest::FederationSyncPeer {
                peer_node_id: peer.node_id.clone(),
                peer_addr: Some(peer.addr_hint.clone()),
            },
            "FederationSyncPeer",
            ticket,
        )
        .await?;

        match response {
            ClientRpcResponse::FederationSyncPeerResult(result) if result.is_success => {
                info!(
                    "  federation sync discovered {} resources from {}",
                    result.resources.len(),
                    result.remote_cluster_name.as_deref().unwrap_or("alice")
                );
                Ok(())
            }
            ClientRpcResponse::FederationSyncPeerResult(result) => FederationSnafu {
                operation: "sync peer",
                reason: result.error.unwrap_or_else(|| "unknown error".to_string()),
            }
            .fail(),
            ClientRpcResponse::Error(error) => FederationSnafu {
                operation: "sync peer",
                reason: format!("{}: {}", error.code, error.message),
            }
            .fail(),
            other => FederationSnafu {
                operation: "sync peer",
                reason: format!("unexpected response: {other:?}"),
            }
            .fail(),
        }
    }
    .await;
    client.shutdown().await;
    result
}

fn peer_from_ticket(ticket: &str) -> DogfoodResult<FederationPeer> {
    let (_topic_id, _cluster_id, addrs) =
        parse_ticket_to_addrs(ticket).map_err(|source| crate::error::DogfoodError::Federation {
            operation: "parse alice ticket".to_string(),
            reason: source.to_string(),
        })?;
    let peer_addr = addrs.first().ok_or_else(|| crate::error::DogfoodError::Federation {
        operation: "parse alice ticket".to_string(),
        reason: "ticket contains no bootstrap peers".to_string(),
    })?;
    let addr_hint = select_addr_hint(peer_addr).ok_or_else(|| crate::error::DogfoodError::Federation {
        operation: "parse alice ticket".to_string(),
        reason: "ticket contains no direct socket addresses".to_string(),
    })?;

    Ok(FederationPeer {
        node_id: peer_addr.id.to_string(),
        addr_hint,
    })
}

fn select_addr_hint(endpoint_addr: &EndpointAddr) -> Option<String> {
    let mut ipv6_hint = None;

    for transport_addr in &endpoint_addr.addrs {
        if let TransportAddr::Ip(socket_addr) = transport_addr {
            if socket_addr.is_ipv4() {
                return Some(socket_addr.to_string());
            }
            if ipv6_hint.is_none() {
                ipv6_hint = Some(socket_addr.to_string());
            }
        }
    }

    ipv6_hint
}

fn mirror_repo_name(source_repo_name: &str) -> String {
    format!("{source_repo_name}-mirror")
}

fn federated_clone_url(local_ticket: &str, peer_node_id: &str, repo_id: &str) -> String {
    format!("aspen://{local_ticket}/fed:{peer_node_id}:{repo_id}")
}

fn clone_dir(config: &RunConfig) -> PathBuf {
    PathBuf::from(&config.cluster_dir).join("federated-clone")
}

async fn clone_repo(
    config: &RunConfig,
    clone_dir: &Path,
    bob_ticket: &str,
    peer: &FederationPeer,
    alice_repo_id: &str,
) -> DogfoodResult<()> {
    let clone_url = federated_clone_url(bob_ticket, &peer.node_id, alice_repo_id);
    let clone_dir_str = clone_dir.to_string_lossy().to_string();
    let path_env = crate::forge::augmented_path(&config.git_remote_aspen_bin);

    for attempt in 1..=2 {
        let output = tokio::process::Command::new("git")
            .args(["clone", clone_url.as_str(), clone_dir_str.as_str()])
            .current_dir(&config.project_dir)
            .env("ASPEN_ORIGIN_ADDR", &peer.addr_hint)
            .env("PATH", &path_env)
            .output()
            .await
            .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
                binary: "git clone".to_string(),
                source: e,
            })?;

        if output.status.success() {
            ensure_clone_has_head(clone_dir).await?;
            return Ok(());
        }

        if attempt == 2 {
            return FederationSnafu {
                operation: "git clone federated repo",
                reason: git_output_detail(&output),
            }
            .fail();
        }

        remove_clone_dir(clone_dir).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    FederationSnafu {
        operation: "git clone federated repo",
        reason: "clone retry loop exhausted".to_string(),
    }
    .fail()
}

async fn ensure_clone_has_head(clone_dir: &Path) -> DogfoodResult<()> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(clone_dir)
        .output()
        .await
        .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
            binary: "git rev-parse".to_string(),
            source: e,
        })?;

    if output.status.success() {
        return Ok(());
    }

    FederationSnafu {
        operation: "verify federated clone",
        reason: "clone completed but repository has no HEAD".to_string(),
    }
    .fail()
}

async fn push_clone(config: &RunConfig, clone_dir: &Path, bob_ticket: &str, bob_repo_id: &str) -> DogfoodResult<()> {
    let remote_url = format!("aspen://{bob_ticket}/{bob_repo_id}");
    let path_env = crate::forge::augmented_path(&config.git_remote_aspen_bin);

    let _ = tokio::process::Command::new("git")
        .args(["remote", "remove", "bob-forge"])
        .current_dir(clone_dir)
        .output()
        .await;

    let add_output = tokio::process::Command::new("git")
        .args(["remote", "add", "bob-forge", remote_url.as_str()])
        .current_dir(clone_dir)
        .output()
        .await
        .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
            binary: "git remote add bob-forge".to_string(),
            source: e,
        })?;
    if !add_output.status.success() {
        let _ = tokio::process::Command::new("git")
            .args(["remote", "set-url", "bob-forge", remote_url.as_str()])
            .current_dir(clone_dir)
            .output()
            .await;
    }

    let push_output = tokio::process::Command::new("git")
        .args(["push", "bob-forge", "HEAD:refs/heads/main", "--force"])
        .current_dir(clone_dir)
        .env("PATH", path_env)
        .output()
        .await
        .map_err(|e| crate::error::DogfoodError::ProcessSpawn {
            binary: "git push bob-forge".to_string(),
            source: e,
        })?;

    if push_output.status.success() {
        return Ok(());
    }

    FederationSnafu {
        operation: "push federated clone to bob",
        reason: git_output_detail(&push_output),
    }
    .fail()
}

fn git_output_detail(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout.trim(), stderr.trim()).trim().to_string();

    if combined.is_empty() {
        format!("git exited with status {}", output.status)
    } else {
        combined
    }
}

async fn connect(ticket: &str) -> DogfoodResult<AspenClient> {
    AspenClient::connect(ticket, Duration::from_secs(30), None).await.map_err(|e| {
        crate::error::DogfoodError::ClientRpc {
            operation: "connect".to_string(),
            target: crate::cluster::ticket_preview(ticket),
            source: e,
        }
    })
}

async fn send(
    client: &AspenClient,
    request: ClientRpcRequest,
    operation: &str,
    ticket: &str,
) -> DogfoodResult<ClientRpcResponse> {
    client.send(request).await.map_err(|e| crate::error::DogfoodError::ClientRpc {
        operation: operation.to_string(),
        target: crate::cluster::ticket_preview(ticket),
        source: e,
    })
}

async fn remove_clone_dir(clone_dir: &Path) {
    if let Err(error) = tokio::fs::remove_dir_all(clone_dir).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %clone_dir.display(), %error, "failed to clean up clone dir");
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::Ipv6Addr;
    use std::net::SocketAddr;

    use iroh::SecretKey;

    use super::*;

    #[test]
    fn mirror_repo_name_adds_suffix() {
        assert_eq!(mirror_repo_name("aspen"), "aspen-mirror");
    }

    #[test]
    fn federated_clone_url_uses_bob_ticket_and_origin() {
        let url = federated_clone_url("aspenv2ticket", "alice-node", "repo-123");
        assert_eq!(url, "aspen://aspenv2ticket/fed:alice-node:repo-123");
    }

    #[test]
    fn select_addr_hint_prefers_ipv4() {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let endpoint_id = secret_key.public();
        let endpoint_addr = EndpointAddr::from_parts(endpoint_id, [
            TransportAddr::Ip(SocketAddr::from((Ipv6Addr::LOCALHOST, 7000))),
            TransportAddr::Ip(SocketAddr::from((Ipv4Addr::LOCALHOST, 7001))),
        ]);

        assert_eq!(select_addr_hint(&endpoint_addr).as_deref(), Some("127.0.0.1:7001"));
    }
}
