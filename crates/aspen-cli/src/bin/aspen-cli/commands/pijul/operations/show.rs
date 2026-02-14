//! Show operation handler.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::super::ShowArgs;
use super::super::output::PijulShowOutput;
use crate::client::AspenClient;
use crate::output::print_output;

/// Show details of a specific change.
///
/// Queries the cluster for change metadata and displays it in a human-readable
/// or JSON format.
pub(in super::super) async fn pijul_show(client: &AspenClient, args: ShowArgs, json: bool) -> Result<()> {
    // Send request to cluster
    let response = client
        .send(ClientRpcRequest::PijulShow {
            repo_id: args.repo_id.clone(),
            change_hash: args.change_hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulShowResult(result) => {
            let output = PijulShowOutput {
                change_hash: result.change_hash,
                repo_id: result.repo_id,
                channel: result.channel,
                message: result.message,
                authors: result.authors.into_iter().map(|a| (a.name, a.email)).collect(),
                dependencies: result.dependencies,
                size_bytes: result.size_bytes,
                recorded_at_ms: result.recorded_at_ms,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message);
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}
