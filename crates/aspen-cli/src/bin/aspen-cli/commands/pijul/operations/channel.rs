//! Channel operation handlers.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::super::ChannelCreateArgs;
use super::super::ChannelDeleteArgs;
use super::super::ChannelForkArgs;
use super::super::ChannelInfoArgs;
use super::super::ChannelListArgs;
use super::super::output::PijulChannelListOutput;
use super::super::output::PijulChannelOutput;
use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;

pub(in super::super) async fn channel_list(client: &AspenClient, args: ChannelListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulChannelList { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::PijulChannelListResult(result) => {
            let output = PijulChannelListOutput {
                channels: result
                    .channels
                    .into_iter()
                    .map(|c| PijulChannelOutput {
                        name: c.name,
                        head: c.head,
                        updated_at_ms: c.updated_at_ms,
                    })
                    .collect(),
                count: result.count,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(in super::super) async fn channel_create(client: &AspenClient, args: ChannelCreateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelCreate {
            repo_id: args.repo_id,
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(in super::super) async fn channel_delete(client: &AspenClient, args: ChannelDeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelDelete {
            repo_id: args.repo_id,
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulSuccess => {
            print_success(&format!("Deleted channel '{}'", args.name), json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(in super::super) async fn channel_fork(client: &AspenClient, args: ChannelForkArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelFork {
            repo_id: args.repo_id,
            source: args.source,
            target: args.target.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(in super::super) async fn channel_info(client: &AspenClient, args: ChannelInfoArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulChannelInfo {
            repo_id: args.repo_id,
            name: args.name,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulChannelResult(result) => {
            let output = PijulChannelOutput {
                name: result.name,
                head: result.head,
                updated_at_ms: result.updated_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
