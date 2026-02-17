//! Blame operation handler.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use super::super::BlameArgs;
use super::super::output::BlameEntry;
use super::super::output::PijulBlameOutput;
use crate::client::AspenClient;
use crate::output::print_output;

/// Show change attribution for a file.
///
/// Queries the cluster for blame information and displays it in a human-readable
/// or JSON format.
pub(in super::super) async fn pijul_blame(client: &AspenClient, args: BlameArgs, json: bool) -> Result<()> {
    // Send request to cluster
    let response = client
        .send(ClientRpcRequest::PijulBlame {
            repo_id: args.repo_id.clone(),
            channel: args.channel.clone(),
            path: args.path.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PijulBlameResult(result) => {
            let output = PijulBlameOutput {
                path: result.path,
                channel: result.channel,
                repo_id: result.repo_id,
                attributions: result
                    .attributions
                    .into_iter()
                    .map(|a| BlameEntry {
                        change_hash: a.change_hash,
                        author: a.author,
                        author_email: a.author_email,
                        message: a.message,
                        recorded_at_ms: a.recorded_at_ms,
                        change_type: a.change_type,
                    })
                    .collect(),
                file_exists: result.does_file_exist,
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
