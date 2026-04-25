//! Standalone JJ-native Aspen remote helper.
//!
//! This binary owns the client-side request planning for JJ-native Forge sync.
//! Network execution is intentionally thin: callers can inspect the JSON plan or
//! hand the request envelope to the Aspen client transport layer.

use anyhow::Result;
use aspen_client_api::forge::JJ_TRANSPORT_VERSION_CURRENT;
use aspen_client_api::forge::JjBookmarkMutation;
use aspen_client_api::forge::JjNativeOperation;
use aspen_client_api::forge::JjNativeRequest;
use clap::Parser;
use clap::Subcommand;
use serde::Serialize;

const REMOTE_HELPER_NAME: &str = "jj-remote-aspen";
const DEFAULT_TRANSPORT_VERSION: u16 = JJ_TRANSPORT_VERSION_CURRENT;

#[derive(Debug, Parser)]
#[command(name = REMOTE_HELPER_NAME)]
#[command(about = "Plan JJ-native Forge sessions over Aspen transport")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build a capability/session-admission request.
    Capabilities(SessionArgs),
    /// Build a clone/fetch request with probe-first object lists.
    Fetch(FetchArgs),
    /// Build a push request with staged objects and bookmark mutations.
    Push(PushArgs),
    /// Build a change-id resolution request.
    ResolveChangeId(ChangeIdArgs),
}

#[derive(Debug, Clone, Parser)]
struct SessionArgs {
    /// Repository ID.
    #[arg(long)]
    repo_id: String,
    /// Client transport version.
    #[arg(long, default_value_t = DEFAULT_TRANSPORT_VERSION)]
    transport_version: u16,
}

#[derive(Debug, Clone, Parser)]
struct FetchArgs {
    /// Repository ID.
    #[arg(long)]
    repo_id: String,
    /// Wanted JJ object IDs.
    #[arg(long = "want")]
    want_objects: Vec<String>,
    /// JJ object IDs already present locally.
    #[arg(long = "have")]
    have_objects: Vec<String>,
    /// Client transport version.
    #[arg(long, default_value_t = DEFAULT_TRANSPORT_VERSION)]
    transport_version: u16,
}

#[derive(Debug, Clone, Parser)]
struct PushArgs {
    /// Repository ID.
    #[arg(long)]
    repo_id: String,
    /// Staged JJ object IDs included in the push.
    #[arg(long = "object")]
    object_ids: Vec<String>,
    /// Bookmark mutation in name[:expected][:new] form. Empty new deletes.
    #[arg(long = "bookmark")]
    bookmarks: Vec<String>,
    /// Change IDs whose heads are updated by this push.
    #[arg(long = "change")]
    change_ids: Vec<String>,
    /// Client transport version.
    #[arg(long, default_value_t = DEFAULT_TRANSPORT_VERSION)]
    transport_version: u16,
}

#[derive(Debug, Clone, Parser)]
struct ChangeIdArgs {
    /// Repository ID.
    #[arg(long)]
    repo_id: String,
    /// Change IDs to resolve.
    #[arg(long = "change")]
    change_ids: Vec<String>,
    /// Client transport version.
    #[arg(long, default_value_t = DEFAULT_TRANSPORT_VERSION)]
    transport_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PlannedSession {
    helper: &'static str,
    request: JjNativeRequest,
}

fn session_request(repo_id: String, transport_version: u16) -> JjNativeRequest {
    JjNativeRequest {
        repo_id,
        operation: JjNativeOperation::Fetch,
        transport_version,
        want_objects: Vec::new(),
        have_objects: Vec::new(),
        change_ids: Vec::new(),
        bookmark_mutations: Vec::new(),
    }
}

fn fetch_request(args: FetchArgs) -> JjNativeRequest {
    JjNativeRequest {
        repo_id: args.repo_id,
        operation: JjNativeOperation::Fetch,
        transport_version: args.transport_version,
        want_objects: args.want_objects,
        have_objects: args.have_objects,
        change_ids: Vec::new(),
        bookmark_mutations: Vec::new(),
    }
}

fn push_request(args: PushArgs) -> JjNativeRequest {
    JjNativeRequest {
        repo_id: args.repo_id,
        operation: JjNativeOperation::Push,
        transport_version: args.transport_version,
        want_objects: args.object_ids,
        have_objects: Vec::new(),
        change_ids: args.change_ids,
        bookmark_mutations: parse_bookmark_mutations(&args.bookmarks),
    }
}

fn change_id_request(args: ChangeIdArgs) -> JjNativeRequest {
    JjNativeRequest {
        repo_id: args.repo_id,
        operation: JjNativeOperation::ResolveChangeId,
        transport_version: args.transport_version,
        want_objects: Vec::new(),
        have_objects: Vec::new(),
        change_ids: args.change_ids,
        bookmark_mutations: Vec::new(),
    }
}

fn parse_bookmark_mutations(values: &[String]) -> Vec<JjBookmarkMutation> {
    values.iter().map(|value| parse_bookmark_mutation(value)).collect()
}

fn parse_bookmark_mutation(value: &str) -> JjBookmarkMutation {
    let parts: Vec<&str> = value.splitn(3, ':').collect();
    let name = parts.first().copied().unwrap_or_default().to_string();
    let expected_head = parts.get(1).and_then(|part| optional_field(part));
    let new_head = parts.get(2).and_then(|part| optional_field(part));
    JjBookmarkMutation {
        name,
        expected_head,
        new_head,
    }
}

fn optional_field(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn planned_session(request: JjNativeRequest) -> PlannedSession {
    PlannedSession {
        helper: REMOTE_HELPER_NAME,
        request,
    }
}

fn command_request(command: Command) -> JjNativeRequest {
    match command {
        Command::Capabilities(args) => session_request(args.repo_id, args.transport_version),
        Command::Fetch(args) => fetch_request(args),
        Command::Push(args) => push_request(args),
        Command::ResolveChangeId(args) => change_id_request(args),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let plan = planned_session(command_request(cli.command));
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_session_uses_current_transport_version() {
        let request = session_request("repo".to_string(), JJ_TRANSPORT_VERSION_CURRENT);
        assert_eq!(request.operation, JjNativeOperation::Fetch);
        assert_eq!(request.transport_version, JJ_TRANSPORT_VERSION_CURRENT);
        assert!(request.want_objects.is_empty());
    }

    #[test]
    fn fetch_request_preserves_probe_first_sets() {
        let request = fetch_request(FetchArgs {
            repo_id: "repo".to_string(),
            want_objects: vec!["want".to_string()],
            have_objects: vec!["have".to_string()],
            transport_version: JJ_TRANSPORT_VERSION_CURRENT,
        });
        assert_eq!(request.operation, JjNativeOperation::Fetch);
        assert_eq!(request.want_objects, vec!["want".to_string()]);
        assert_eq!(request.have_objects, vec!["have".to_string()]);
    }

    #[test]
    fn push_request_carries_bookmark_and_change_updates() {
        let request = push_request(PushArgs {
            repo_id: "repo".to_string(),
            object_ids: vec!["object".to_string()],
            bookmarks: vec!["main:old:new".to_string(), "delete:old:".to_string()],
            change_ids: vec!["change".to_string()],
            transport_version: JJ_TRANSPORT_VERSION_CURRENT,
        });
        assert_eq!(request.operation, JjNativeOperation::Push);
        assert_eq!(request.want_objects, vec!["object".to_string()]);
        assert_eq!(request.change_ids, vec!["change".to_string()]);
        assert_eq!(request.bookmark_mutations.len(), 2);
        assert_eq!(request.bookmark_mutations[0].new_head, Some("new".to_string()));
        assert_eq!(request.bookmark_mutations[1].new_head, None);
    }

    #[test]
    fn change_id_request_carries_requested_change_ids() {
        let request = change_id_request(ChangeIdArgs {
            repo_id: "repo".to_string(),
            change_ids: vec!["change".to_string()],
            transport_version: JJ_TRANSPORT_VERSION_CURRENT,
        });
        assert_eq!(request.operation, JjNativeOperation::ResolveChangeId);
        assert_eq!(request.change_ids, vec!["change".to_string()]);
    }
}
