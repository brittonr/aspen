//! WASM guest plugin for the Aspen forge handler (Phase 1).
//!
//! Handles repositories (3 ops), git objects (7 ops), and refs (7 ops)
//! using the host-provided KV store, blob store, and crypto bindings.
//! COBs, federation, and git bridge are deferred to later phases.

mod issues;
mod kv;
mod objects;
mod patches;
mod refs;
mod repo;
mod signing;
mod types;

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

struct ForgePlugin;

impl AspenPlugin for ForgePlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "forge".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                // Repos
                "ForgeCreateRepo".to_string(),
                "ForgeGetRepo".to_string(),
                "ForgeListRepos".to_string(),
                // Git objects
                "ForgeStoreBlob".to_string(),
                "ForgeGetBlob".to_string(),
                "ForgeCreateTree".to_string(),
                "ForgeGetTree".to_string(),
                "ForgeCommit".to_string(),
                "ForgeGetCommit".to_string(),
                "ForgeLog".to_string(),
                // Refs
                "ForgeGetRef".to_string(),
                "ForgeSetRef".to_string(),
                "ForgeDeleteRef".to_string(),
                "ForgeCasRef".to_string(),
                "ForgeListBranches".to_string(),
                "ForgeListTags".to_string(),
                "ForgeGetDelegateKey".to_string(),
                // Issues
                "ForgeCreateIssue".to_string(),
                "ForgeListIssues".to_string(),
                "ForgeGetIssue".to_string(),
                "ForgeCommentIssue".to_string(),
                "ForgeCloseIssue".to_string(),
                "ForgeReopenIssue".to_string(),
                // Patches
                "ForgeCreatePatch".to_string(),
                "ForgeListPatches".to_string(),
                "ForgeGetPatch".to_string(),
                "ForgeUpdatePatch".to_string(),
                "ForgeApprovePatch".to_string(),
                "ForgeMergePatch".to_string(),
                "ForgeClosePatch".to_string(),
            ],
            priority: 950,
            app_id: Some("forge".to_string()),
            kv_prefixes: vec!["forge:".to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                blob_read: true,
                blob_write: true,
                signing: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            // Repos
            ClientRpcRequest::ForgeCreateRepo {
                name,
                description,
                default_branch,
            } => repo::handle_create_repo(name, description, default_branch),

            ClientRpcRequest::ForgeGetRepo { repo_id } => repo::handle_get_repo(repo_id),

            ClientRpcRequest::ForgeListRepos { limit, offset } => repo::handle_list_repos(limit, offset),

            // Git objects
            ClientRpcRequest::ForgeStoreBlob { repo_id, content } => objects::handle_store_blob(repo_id, content),

            ClientRpcRequest::ForgeGetBlob { hash } => objects::handle_get_blob(hash),

            ClientRpcRequest::ForgeCreateTree { repo_id, entries_json } => {
                objects::handle_create_tree(repo_id, entries_json)
            }

            ClientRpcRequest::ForgeGetTree { hash } => objects::handle_get_tree(hash),

            ClientRpcRequest::ForgeCommit {
                repo_id,
                tree,
                parents,
                message,
            } => objects::handle_commit(repo_id, tree, parents, message),

            ClientRpcRequest::ForgeGetCommit { hash } => objects::handle_get_commit(hash),

            ClientRpcRequest::ForgeLog {
                repo_id,
                ref_name,
                limit,
            } => objects::handle_log(repo_id, ref_name, limit),

            // Refs
            ClientRpcRequest::ForgeGetRef { repo_id, ref_name } => refs::handle_get_ref(repo_id, ref_name),

            ClientRpcRequest::ForgeSetRef {
                repo_id,
                ref_name,
                hash,
                signer,
                signature,
                timestamp_ms,
            } => refs::handle_set_ref(repo_id, ref_name, hash, signer, signature, timestamp_ms),

            ClientRpcRequest::ForgeDeleteRef { repo_id, ref_name } => refs::handle_delete_ref(repo_id, ref_name),

            ClientRpcRequest::ForgeCasRef {
                repo_id,
                ref_name,
                expected,
                new_hash,
                signer,
                signature,
                timestamp_ms,
            } => refs::handle_cas_ref(repo_id, ref_name, expected, new_hash, signer, signature, timestamp_ms),

            ClientRpcRequest::ForgeListBranches { repo_id } => refs::handle_list_branches(repo_id),

            ClientRpcRequest::ForgeListTags { repo_id } => refs::handle_list_tags(repo_id),

            ClientRpcRequest::ForgeGetDelegateKey { repo_id: _ } => refs::handle_get_delegate_key(),

            // Issues
            ClientRpcRequest::ForgeCreateIssue {
                repo_id,
                title,
                body,
                labels,
            } => issues::handle_create_issue(repo_id, title, body, labels),

            ClientRpcRequest::ForgeListIssues { repo_id, state, limit } => {
                issues::handle_list_issues(repo_id, state, limit)
            }

            ClientRpcRequest::ForgeGetIssue { repo_id, issue_id } => issues::handle_get_issue(repo_id, issue_id),

            ClientRpcRequest::ForgeCommentIssue {
                repo_id,
                issue_id,
                body,
            } => issues::handle_comment_issue(repo_id, issue_id, body),

            ClientRpcRequest::ForgeCloseIssue {
                repo_id,
                issue_id,
                reason,
            } => issues::handle_close_issue(repo_id, issue_id, reason),

            ClientRpcRequest::ForgeReopenIssue { repo_id, issue_id } => issues::handle_reopen_issue(repo_id, issue_id),

            // Patches
            ClientRpcRequest::ForgeCreatePatch {
                repo_id,
                title,
                description,
                base,
                head,
            } => patches::handle_create_patch(repo_id, title, description, base, head),

            ClientRpcRequest::ForgeListPatches { repo_id, state, limit } => {
                patches::handle_list_patches(repo_id, state, limit)
            }

            ClientRpcRequest::ForgeGetPatch { repo_id, patch_id } => patches::handle_get_patch(repo_id, patch_id),

            ClientRpcRequest::ForgeUpdatePatch {
                repo_id,
                patch_id,
                head,
                message,
            } => patches::handle_update_patch(repo_id, patch_id, head, message),

            ClientRpcRequest::ForgeApprovePatch {
                repo_id,
                patch_id,
                commit,
                message,
            } => patches::handle_approve_patch(repo_id, patch_id, commit, message),

            ClientRpcRequest::ForgeMergePatch {
                repo_id,
                patch_id,
                merge_commit,
            } => patches::handle_merge_patch(repo_id, patch_id, merge_commit),

            ClientRpcRequest::ForgeClosePatch {
                repo_id,
                patch_id,
                reason,
            } => patches::handle_close_patch(repo_id, patch_id, reason),

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "forge plugin does not handle this request type",
            )),
        }
    }
}

register_plugin!(ForgePlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = ForgePlugin::info();
        assert_eq!(info.name, manifest.name, "name mismatch between code and plugin.json");
        assert_eq!(info.handles, manifest.handles, "handles mismatch between code and plugin.json");
        assert_eq!(info.priority, manifest.priority, "priority mismatch between code and plugin.json");
        assert_eq!(info.version, manifest.version, "version mismatch between code and plugin.json");
        assert_eq!(info.app_id, manifest.app_id, "app_id mismatch between code and plugin.json");
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes, "kv_prefixes mismatch between code and plugin.json");
    }
}
