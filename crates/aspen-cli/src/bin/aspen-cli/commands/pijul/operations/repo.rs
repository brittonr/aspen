//! Repository operation handlers.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::super::RepoInfoArgs;
use super::super::RepoInitArgs;
use super::super::RepoListArgs;
use super::super::output::PijulRepoListOutput;
use super::super::output::PijulRepoOutput;
use crate::client::AspenClient;
use crate::output::print_output;

pub(in super::super) async fn repo_init(client: &AspenClient, args: RepoInitArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PijulRepoInit {
            name: args.name.clone(),
            description: args.description,
            default_channel: args.default_channel,
        })
        .await?;

    match response {
        ClientRpcResponse::PijulRepoResult(result) => {
            let output = PijulRepoOutput {
                id: result.id,
                name: result.name,
                description: result.description,
                default_channel: result.default_channel,
                channel_count: result.channel_count,
                created_at_ms: result.created_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

pub(in super::super) async fn repo_list(client: &AspenClient, args: RepoListArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulRepoList { limit: args.limit }).await?;

    match response {
        ClientRpcResponse::PijulRepoListResult(result) => {
            let output = PijulRepoListOutput {
                repos: result
                    .repos
                    .into_iter()
                    .map(|r| PijulRepoOutput {
                        id: r.id,
                        name: r.name,
                        description: r.description,
                        default_channel: r.default_channel,
                        channel_count: r.channel_count,
                        created_at_ms: r.created_at_ms,
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

pub(in super::super) async fn repo_info(client: &AspenClient, args: RepoInfoArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::PijulRepoInfo { repo_id: args.repo_id }).await?;

    match response {
        ClientRpcResponse::PijulRepoResult(result) => {
            let output = PijulRepoOutput {
                id: result.id,
                name: result.name,
                description: result.description,
                default_channel: result.default_channel,
                channel_count: result.channel_count,
                created_at_ms: result.created_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
