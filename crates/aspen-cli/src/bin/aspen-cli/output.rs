//! Output formatting for CLI responses.
//!
//! Supports both human-readable and JSON output formats for
//! integration with scripts and other tools.

/// Trait for types that can be output in multiple formats.
pub trait Outputable {
    /// Convert to JSON value for structured output.
    fn to_json(&self) -> serde_json::Value;

    /// Convert to human-readable string.
    fn to_human(&self) -> String;
}

/// Print a value in the appropriate format.
pub fn print_output<T: Outputable>(value: &T, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value.to_json())
                .unwrap_or_else(|e| { format!("{{\"error\": \"failed to serialize: {}\"}}", e) })
        );
    } else {
        println!("{}", value.to_human());
    }
}

/// Print a success message.
pub fn print_success(message: &str, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "success",
                "message": message
            })
        );
    } else {
        println!("{}", message);
    }
}

/// Print an error message.
pub fn print_error(message: &str, json: bool) {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({
                "status": "error",
                "message": message
            })
        );
    } else {
        eprintln!("Error: {}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Test Category 1: to_json() Round-Trip Tests
    // =============================================================================

    #[test]
    fn test_health_output_to_json_with_raft_node_id() {
        let output = HealthOutput {
            status: "healthy".to_string(),
            node_id: 1,
            raft_node_id: Some(42),
            uptime_seconds: 3600,
        };

        let json = output.to_json();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["node_id"], 1);
        assert_eq!(json["raft_node_id"], 42);
        assert_eq!(json["uptime_seconds"], 3600);
    }

    #[test]
    fn test_health_output_to_json_without_raft_node_id() {
        let output = HealthOutput {
            status: "initializing".to_string(),
            node_id: 2,
            raft_node_id: None,
            uptime_seconds: 0,
        };

        let json = output.to_json();
        assert_eq!(json["status"], "initializing");
        assert_eq!(json["node_id"], 2);
        assert!(json["raft_node_id"].is_null());
        assert_eq!(json["uptime_seconds"], 0);
    }

    #[test]
    fn test_raft_metrics_output_to_json_with_leader() {
        let output = RaftMetricsOutput {
            state: "Follower".to_string(),
            current_leader: Some(1),
            current_term: 5,
            last_log_index: 100,
            last_applied: 99,
            snapshot_index: 50,
        };

        let json = output.to_json();
        assert_eq!(json["state"], "Follower");
        assert_eq!(json["current_leader"], 1);
        assert_eq!(json["current_term"], 5);
        assert_eq!(json["last_log_index"], 100);
        assert_eq!(json["last_applied"], 99);
        assert_eq!(json["snapshot_index"], 50);
    }

    #[test]
    fn test_raft_metrics_output_to_json_without_leader() {
        let output = RaftMetricsOutput {
            state: "Leader".to_string(),
            current_leader: None,
            current_term: 0,
            last_log_index: 0,
            last_applied: 0,
            snapshot_index: 0,
        };

        let json = output.to_json();
        assert_eq!(json["state"], "Leader");
        assert!(json["current_leader"].is_null());
        assert_eq!(json["current_term"], 0);
    }

    #[test]
    fn test_cluster_state_output_to_json_with_nodes() {
        let output = ClusterStateOutput {
            nodes: vec![
                NodeInfo {
                    node_id: 1,
                    endpoint_id: "abc123".to_string(),
                    is_leader: true,
                    is_voter: true,
                },
                NodeInfo {
                    node_id: 2,
                    endpoint_id: "def456".to_string(),
                    is_leader: false,
                    is_voter: false,
                },
            ],
        };

        let json = output.to_json();
        let nodes = json["nodes"].as_array().expect("nodes should be array");
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["node_id"], 1);
        assert_eq!(nodes[0]["endpoint_id"], "abc123");
        assert_eq!(nodes[0]["is_leader"], true);
        assert_eq!(nodes[0]["is_voter"], true);
        assert_eq!(nodes[1]["node_id"], 2);
        assert_eq!(nodes[1]["is_leader"], false);
    }

    #[test]
    fn test_cluster_state_output_to_json_empty_nodes() {
        let output = ClusterStateOutput { nodes: vec![] };

        let json = output.to_json();
        let nodes = json["nodes"].as_array().expect("nodes should be array");
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_kv_read_output_to_json_with_value() {
        let output = KvReadOutput {
            key: "mykey".to_string(),
            value: Some(b"myvalue".to_vec()),
            does_exist: true,
        };

        let json = output.to_json();
        assert_eq!(json["key"], "mykey");
        assert_eq!(json["does_exist"], true);
        assert_eq!(json["value"], "myvalue");
    }

    #[test]
    fn test_kv_read_output_to_json_with_binary_value() {
        let binary_data = vec![0xFF, 0xFE, 0xFD, 0xFC];
        let output = KvReadOutput {
            key: "binkey".to_string(),
            value: Some(binary_data.clone()),
            does_exist: true,
        };

        let json = output.to_json();
        assert_eq!(json["key"], "binkey");
        assert_eq!(json["does_exist"], true);

        // Binary should be base64 encoded
        let value_obj = json["value"].as_object().expect("binary value should be object");
        assert!(value_obj.contains_key("base64"));
        assert_eq!(value_obj["base64"], base64_encode(&binary_data));
    }

    #[test]
    fn test_kv_read_output_to_json_key_not_found() {
        let output = KvReadOutput {
            key: "missing".to_string(),
            value: None,
            does_exist: false,
        };

        let json = output.to_json();
        assert_eq!(json["key"], "missing");
        assert_eq!(json["does_exist"], false);
        assert!(json["value"].is_null());
    }

    #[test]
    fn test_kv_scan_output_to_json_with_entries() {
        let output = KvScanOutput {
            entries: vec![
                ("key1".to_string(), b"value1".to_vec()),
                ("key2".to_string(), b"value2".to_vec()),
            ],
            continuation_token: Some("token123".to_string()),
        };

        let json = output.to_json();
        let entries = json["entries"].as_array().expect("entries should be array");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["key"], "key1");
        assert_eq!(entries[0]["value"], "value1");
        assert_eq!(json["count"], 2);
        assert_eq!(json["continuation_token"], "token123");
    }

    #[test]
    fn test_kv_scan_output_to_json_empty() {
        let output = KvScanOutput {
            entries: vec![],
            continuation_token: None,
        };

        let json = output.to_json();
        let entries = json["entries"].as_array().expect("entries should be array");
        assert_eq!(entries.len(), 0);
        assert_eq!(json["count"], 0);
        assert!(json["continuation_token"].is_null());
    }

    #[test]
    fn test_kv_batch_read_output_to_json() {
        let output = KvBatchReadOutput {
            keys: vec!["k1".to_string(), "k2".to_string(), "k3".to_string()],
            values: vec![Some(b"v1".to_vec()), None, Some(b"v3".to_vec())],
        };

        let json = output.to_json();
        assert_eq!(json["count"], 3);
        let results = json["results"].as_array().expect("results should be array");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0]["key"], "k1");
        assert_eq!(results[0]["value"], "v1");
        assert_eq!(results[0]["does_exist"], true);
        assert_eq!(results[1]["key"], "k2");
        assert!(results[1]["value"].is_null());
        assert_eq!(results[1]["does_exist"], false);
    }

    #[test]
    fn test_kv_batch_read_output_to_json_empty() {
        let output = KvBatchReadOutput {
            keys: vec![],
            values: vec![],
        };

        let json = output.to_json();
        assert_eq!(json["count"], 0);
        let results = json["results"].as_array().expect("results should be array");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_kv_batch_write_output_to_json_success() {
        let output = KvBatchWriteOutput {
            is_success: true,
            operations_applied: 42,
        };

        let json = output.to_json();
        assert_eq!(json["is_success"], true);
        assert_eq!(json["operations_applied"], 42);
    }

    #[test]
    fn test_kv_batch_write_output_to_json_failure() {
        let output = KvBatchWriteOutput {
            is_success: false,
            operations_applied: 0,
        };

        let json = output.to_json();
        assert_eq!(json["is_success"], false);
        assert_eq!(json["operations_applied"], 0);
    }

    #[test]
    #[cfg(feature = "sql")]
    fn test_sql_query_output_to_json() {
        use aspen_client_api::SqlCellValue;

        use crate::commands::sql::OutputFormat;

        let output = SqlQueryOutput {
            columns: vec!["id".to_string(), "name".to_string(), "score".to_string()],
            rows: vec![
                vec![
                    SqlCellValue::Integer(1),
                    SqlCellValue::Text("Alice".to_string()),
                    SqlCellValue::Real(95.5),
                ],
                vec![SqlCellValue::Integer(2), SqlCellValue::Null, SqlCellValue::Real(88.0)],
            ],
            row_count: 2,
            is_truncated: false,
            execution_time_ms: 42,
            show_headers: true,
            format: OutputFormat::Table,
        };

        let json = output.to_json();
        assert_eq!(json["row_count"], 2);
        assert_eq!(json["is_truncated"], false);
        assert_eq!(json["execution_time_ms"], 42);

        let columns = json["columns"].as_array().expect("columns should be array");
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0], "id");

        let rows = json["rows"].as_array().expect("rows should be array");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["id"], 1);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[0]["score"], 95.5);
        assert!(rows[1]["name"].is_null());
    }

    #[test]
    fn test_repo_list_output_to_json() {
        let output = RepoListOutput {
            repos: vec![
                RepoListItem {
                    id: "repo123".to_string(),
                    name: "my-project".to_string(),
                    description: Some("A cool project".to_string()),
                    default_branch: "main".to_string(),
                },
                RepoListItem {
                    id: "repo456".to_string(),
                    name: "another-project".to_string(),
                    description: None,
                    default_branch: "master".to_string(),
                },
            ],
            count: 2,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 2);
        let repos = json["repos"].as_array().expect("repos should be array");
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0]["id"], "repo123");
        assert_eq!(repos[0]["name"], "my-project");
        assert_eq!(repos[0]["description"], "A cool project");
        assert_eq!(repos[0]["default_branch"], "main");
        assert!(repos[1]["description"].is_null());
    }

    #[test]
    fn test_repo_list_output_to_json_empty() {
        let output = RepoListOutput {
            repos: vec![],
            count: 0,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 0);
        let repos = json["repos"].as_array().expect("repos should be array");
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn test_repo_output_to_json() {
        let output = RepoOutput {
            id: "repo789".to_string(),
            name: "test-repo".to_string(),
            description: Some("Test repository".to_string()),
            default_branch: "develop".to_string(),
            delegates: vec!["delegate1".to_string(), "delegate2".to_string()],
            threshold: 2,
            created_at_ms: 1234567890,
        };

        let json = output.to_json();
        assert_eq!(json["id"], "repo789");
        assert_eq!(json["name"], "test-repo");
        assert_eq!(json["description"], "Test repository");
        assert_eq!(json["default_branch"], "develop");
        assert_eq!(json["threshold"], 2);
        assert_eq!(json["created_at_ms"], 1234567890);

        let delegates = json["delegates"].as_array().expect("delegates should be array");
        assert_eq!(delegates.len(), 2);
    }

    #[test]
    fn test_commit_output_to_json() {
        let output = CommitOutput {
            hash: "abc123def456".to_string(),
            tree: "tree789".to_string(),
            parents: vec!["parent1".to_string(), "parent2".to_string()],
            author_name: "Alice".to_string(),
            author_email: Some("alice@example.com".to_string()),
            message: "Initial commit".to_string(),
            timestamp_ms: 9876543210,
        };

        let json = output.to_json();
        assert_eq!(json["hash"], "abc123def456");
        assert_eq!(json["tree"], "tree789");
        assert_eq!(json["author_name"], "Alice");
        assert_eq!(json["author_email"], "alice@example.com");
        assert_eq!(json["message"], "Initial commit");
        assert_eq!(json["timestamp_ms"], 9876543210_u64);

        let parents = json["parents"].as_array().expect("parents should be array");
        assert_eq!(parents.len(), 2);
    }

    #[test]
    fn test_commit_output_to_json_no_email_no_parents() {
        let output = CommitOutput {
            hash: "hash123".to_string(),
            tree: "tree456".to_string(),
            parents: vec![],
            author_name: "Bob".to_string(),
            author_email: None,
            message: "Root commit".to_string(),
            timestamp_ms: 1111111111,
        };

        let json = output.to_json();
        assert_eq!(json["author_name"], "Bob");
        assert!(json["author_email"].is_null());
        let parents = json["parents"].as_array().expect("parents should be array");
        assert_eq!(parents.len(), 0);
    }

    #[test]
    fn test_log_output_to_json() {
        let output = LogOutput {
            commits: vec![
                CommitOutput {
                    hash: "commit1".to_string(),
                    tree: "tree1".to_string(),
                    parents: vec![],
                    author_name: "Alice".to_string(),
                    author_email: None,
                    message: "First".to_string(),
                    timestamp_ms: 1000,
                },
                CommitOutput {
                    hash: "commit2".to_string(),
                    tree: "tree2".to_string(),
                    parents: vec!["commit1".to_string()],
                    author_name: "Bob".to_string(),
                    author_email: None,
                    message: "Second".to_string(),
                    timestamp_ms: 2000,
                },
            ],
            count: 2,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 2);
        let commits_json = json["commits"].as_array().expect("commits should be array");
        assert_eq!(commits_json.len(), 2);
        assert_eq!(commits_json[0]["hash"], "commit1");
        assert_eq!(commits_json[1]["hash"], "commit2");
    }

    #[test]
    fn test_log_output_to_json_empty() {
        let output = LogOutput {
            commits: vec![],
            count: 0,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 0);
        let commits = json["commits"].as_array().expect("commits should be array");
        assert_eq!(commits.len(), 0);
    }

    #[test]
    fn test_ref_output_to_json() {
        let output = RefOutput {
            name: "refs/heads/main".to_string(),
            hash: "abc123".to_string(),
        };

        let json = output.to_json();
        assert_eq!(json["name"], "refs/heads/main");
        assert_eq!(json["hash"], "abc123");
    }

    #[test]
    fn test_ref_list_output_to_json() {
        let output = RefListOutput {
            refs: vec![
                RefOutput {
                    name: "refs/heads/main".to_string(),
                    hash: "hash1".to_string(),
                },
                RefOutput {
                    name: "refs/tags/v1.0".to_string(),
                    hash: "hash2".to_string(),
                },
            ],
            count: 2,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 2);
        let refs = json["refs"].as_array().expect("refs should be array");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0]["name"], "refs/heads/main");
        assert_eq!(refs[1]["name"], "refs/tags/v1.0");
    }

    #[test]
    fn test_comment_output_to_json() {
        let output = CommentOutput {
            hash: "comment123abc".to_string(),
            author: "author456def".to_string(),
            body: "This is a comment".to_string(),
            timestamp_ms: 5555555555,
        };

        let json = output.to_json();
        assert_eq!(json["hash"], "comment123abc");
        assert_eq!(json["author"], "author456def");
        assert_eq!(json["body"], "This is a comment");
        assert_eq!(json["timestamp_ms"], 5555555555_u64);
    }

    #[test]
    fn test_issue_output_to_json() {
        let output = IssueOutput {
            id: "issue123".to_string(),
            title: "Bug report".to_string(),
            state: "open".to_string(),
            labels: vec!["bug".to_string(), "priority".to_string()],
            comment_count: 5,
            created_at_ms: 1000,
            updated_at_ms: 2000,
        };

        let json = output.to_json();
        assert_eq!(json["id"], "issue123");
        assert_eq!(json["title"], "Bug report");
        assert_eq!(json["state"], "open");
        assert_eq!(json["comment_count"], 5);

        let labels = json["labels"].as_array().expect("labels should be array");
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0], "bug");
    }

    #[test]
    fn test_issue_list_output_to_json() {
        let output = IssueListOutput {
            issues: vec![IssueOutput {
                id: "issue1".to_string(),
                title: "First issue".to_string(),
                state: "open".to_string(),
                labels: vec![],
                comment_count: 0,
                created_at_ms: 1000,
                updated_at_ms: 1000,
            }],
            count: 1,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 1);
        let issues = json["issues"].as_array().expect("issues should be array");
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_issue_detail_output_to_json_with_comments() {
        let output = IssueDetailOutput {
            id: "issue789".to_string(),
            title: "Detailed issue".to_string(),
            body: "Issue description".to_string(),
            state: "closed".to_string(),
            labels: vec!["feature".to_string()],
            assignees: vec!["user1".to_string(), "user2".to_string()],
            comment_count: 2,
            created_at_ms: 3000,
            updated_at_ms: 4000,
            comments: Some(vec![CommentOutput {
                hash: "c1".to_string(),
                author: "a1".to_string(),
                body: "Comment 1".to_string(),
                timestamp_ms: 3500,
            }]),
        };

        let json = output.to_json();
        assert_eq!(json["id"], "issue789");
        assert_eq!(json["title"], "Detailed issue");
        assert_eq!(json["body"], "Issue description");
        assert_eq!(json["state"], "closed");

        let labels = json["labels"].as_array().expect("labels should be array");
        assert_eq!(labels.len(), 1);

        let assignees = json["assignees"].as_array().expect("assignees should be array");
        assert_eq!(assignees.len(), 2);

        let comments = json["comments"].as_array().expect("comments should be array");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0]["body"], "Comment 1");
    }

    #[test]
    fn test_issue_detail_output_to_json_without_comments() {
        let output = IssueDetailOutput {
            id: "issue999".to_string(),
            title: "No comments".to_string(),
            body: "Body".to_string(),
            state: "open".to_string(),
            labels: vec![],
            assignees: vec![],
            comment_count: 0,
            created_at_ms: 5000,
            updated_at_ms: 5000,
            comments: None,
        };

        let json = output.to_json();
        assert!(json["comments"].is_null());
        let labels = json["labels"].as_array().expect("labels should be array");
        assert_eq!(labels.len(), 0);
    }

    #[test]
    fn test_patch_output_to_json() {
        let output = PatchOutput {
            id: "patch123".to_string(),
            title: "Feature patch".to_string(),
            state: "merged".to_string(),
            base: "basehash".to_string(),
            head: "headhash".to_string(),
            labels: vec!["enhancement".to_string()],
            revision_count: 3,
            approval_count: 2,
            created_at_ms: 6000,
            updated_at_ms: 7000,
        };

        let json = output.to_json();
        assert_eq!(json["id"], "patch123");
        assert_eq!(json["title"], "Feature patch");
        assert_eq!(json["state"], "merged");
        assert_eq!(json["base"], "basehash");
        assert_eq!(json["head"], "headhash");
        assert_eq!(json["revision_count"], 3);
        assert_eq!(json["approval_count"], 2);
    }

    #[test]
    fn test_patch_list_output_to_json() {
        let output = PatchListOutput {
            patches: vec![PatchOutput {
                id: "p1".to_string(),
                title: "Patch 1".to_string(),
                state: "open".to_string(),
                base: "b1".to_string(),
                head: "h1".to_string(),
                labels: vec![],
                revision_count: 1,
                approval_count: 0,
                created_at_ms: 1000,
                updated_at_ms: 1000,
            }],
            count: 1,
        };

        let json = output.to_json();
        assert_eq!(json["count"], 1);
        let patches = json["patches"].as_array().expect("patches should be array");
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_patch_detail_output_to_json_complete() {
        let output = PatchDetailOutput {
            id: "patch789".to_string(),
            title: "Big patch".to_string(),
            description: "Long description".to_string(),
            state: "approved".to_string(),
            base: "basecommit".to_string(),
            head: "headcommit".to_string(),
            labels: vec!["critical".to_string()],
            assignees: vec!["reviewer1".to_string()],
            revision_count: 2,
            approval_count: 1,
            created_at_ms: 8000,
            updated_at_ms: 9000,
            comments: Some(vec![CommentOutput {
                hash: "c1".to_string(),
                author: "a1".to_string(),
                body: "LGTM".to_string(),
                timestamp_ms: 8500,
            }]),
            revisions: Some(vec![RevisionOutput {
                hash: "r1".to_string(),
                head: "h1".to_string(),
                message: Some("Rev 1".to_string()),
                author: "author1".to_string(),
                timestamp_ms: 8200,
            }]),
            approvals: Some(vec![ApprovalOutput {
                author: "approver1".to_string(),
                commit: "commitabc".to_string(),
                message: Some("Approved".to_string()),
                timestamp_ms: 8800,
            }]),
        };

        let json = output.to_json();
        assert_eq!(json["id"], "patch789");

        let comments = json["comments"].as_array().expect("comments should be array");
        assert_eq!(comments.len(), 1);

        let revisions = json["revisions"].as_array().expect("revisions should be array");
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0]["hash"], "r1");
        assert_eq!(revisions[0]["message"], "Rev 1");

        let approvals = json["approvals"].as_array().expect("approvals should be array");
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals[0]["author"], "approver1");
        assert_eq!(approvals[0]["commit"], "commitabc");
    }

    #[test]
    fn test_patch_detail_output_to_json_minimal() {
        let output = PatchDetailOutput {
            id: "minimal".to_string(),
            title: "Min".to_string(),
            description: "".to_string(),
            state: "draft".to_string(),
            base: "b".to_string(),
            head: "h".to_string(),
            labels: vec![],
            assignees: vec![],
            revision_count: 0,
            approval_count: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
            comments: None,
            revisions: None,
            approvals: None,
        };

        let json = output.to_json();
        assert!(json["comments"].is_null());
        assert!(json["revisions"].is_null());
        assert!(json["approvals"].is_null());
    }

    // =============================================================================
    // Test Category 2: to_human() Formatting Tests
    // =============================================================================

    #[test]
    fn test_health_output_to_human_with_raft_id() {
        let output = HealthOutput {
            status: "healthy".to_string(),
            node_id: 1,
            raft_node_id: Some(42),
            uptime_seconds: 3600,
        };

        let human = output.to_human();
        assert!(human.contains("Health Status"));
        assert!(human.contains("healthy"));
        assert!(human.contains("Node ID:        1"));
        assert!(human.contains("Raft Node ID:   42"));
        assert!(human.contains("Uptime:         3600s"));
    }

    #[test]
    fn test_health_output_to_human_without_raft_id() {
        let output = HealthOutput {
            status: "degraded".to_string(),
            node_id: 99,
            raft_node_id: None,
            uptime_seconds: 0,
        };

        let human = output.to_human();
        assert!(human.contains("degraded"));
        assert!(human.contains("Raft Node ID:   N/A"));
        assert!(human.contains("Uptime:         0s"));
    }

    #[test]
    fn test_raft_metrics_output_to_human() {
        let output = RaftMetricsOutput {
            state: "Leader".to_string(),
            current_leader: Some(5),
            current_term: 10,
            last_log_index: 500,
            last_applied: 499,
            snapshot_index: 400,
        };

        let human = output.to_human();
        assert!(human.contains("Raft Metrics"));
        assert!(human.contains("State:          Leader"));
        assert!(human.contains("Current Leader: 5"));
        assert!(human.contains("Current Term:   10"));
        assert!(human.contains("Last Log Index: 500"));
    }

    #[test]
    fn test_cluster_state_output_to_human_with_nodes() {
        let output = ClusterStateOutput {
            nodes: vec![
                NodeInfo {
                    node_id: 1,
                    endpoint_id: "abc123".to_string(),
                    is_leader: true,
                    is_voter: true,
                },
                NodeInfo {
                    node_id: 2,
                    endpoint_id: "def456".to_string(),
                    is_leader: false,
                    is_voter: false,
                },
            ],
        };

        let human = output.to_human();
        assert!(human.contains("Cluster State"));
        assert!(human.contains("Node ID"));
        assert!(human.contains("Leader"));
        assert!(human.contains("Voter"));
        assert!(human.contains("*")); // Leader marker
        assert!(human.contains("Y")); // Voter marker
        assert!(human.contains("N")); // Non-voter marker
    }

    #[test]
    fn test_cluster_state_output_to_human_empty() {
        let output = ClusterStateOutput { nodes: vec![] };

        let human = output.to_human();
        assert_eq!(human, "No nodes in cluster");
    }

    #[test]
    fn test_kv_read_output_to_human_found() {
        let output = KvReadOutput {
            key: "testkey".to_string(),
            value: Some(b"testvalue".to_vec()),
            does_exist: true,
        };

        let human = output.to_human();
        assert_eq!(human, "testvalue");
    }

    #[test]
    fn test_kv_read_output_to_human_not_found() {
        let output = KvReadOutput {
            key: "missing".to_string(),
            value: None,
            does_exist: false,
        };

        let human = output.to_human();
        assert_eq!(human, "Key 'missing' not found");
    }

    #[test]
    fn test_kv_read_output_to_human_binary() {
        let output = KvReadOutput {
            key: "binkey".to_string(),
            value: Some(vec![0xFF, 0xFE, 0xFD]),
            does_exist: true,
        };

        let human = output.to_human();
        assert!(human.contains("<binary: 3 bytes>"));
    }

    #[test]
    fn test_kv_scan_output_to_human_with_results() {
        let output = KvScanOutput {
            entries: vec![
                ("key1".to_string(), b"short".to_vec()),
                ("key2".to_string(), b"this is a very long value that should be truncated in display".to_vec()),
            ],
            continuation_token: None,
        };

        let human = output.to_human();
        assert!(human.contains("Found 2 key(s)"));
        assert!(human.contains("key1: short"));
        assert!(human.contains("key2:"));
        assert!(human.contains("...")); // Truncation indicator
    }

    #[test]
    fn test_kv_scan_output_to_human_with_token() {
        let output = KvScanOutput {
            entries: vec![("k".to_string(), b"v".to_vec())],
            continuation_token: Some("nextpage".to_string()),
        };

        let human = output.to_human();
        assert!(human.contains("More results available"));
        assert!(human.contains("--token nextpage"));
    }

    #[test]
    fn test_kv_scan_output_to_human_empty() {
        let output = KvScanOutput {
            entries: vec![],
            continuation_token: None,
        };

        let human = output.to_human();
        assert_eq!(human, "No keys found");
    }

    #[test]
    fn test_kv_batch_write_output_to_human_success() {
        let output = KvBatchWriteOutput {
            is_success: true,
            operations_applied: 10,
        };

        let human = output.to_human();
        assert_eq!(human, "OK: 10 operation(s) applied");
    }

    #[test]
    fn test_kv_batch_write_output_to_human_failure() {
        let output = KvBatchWriteOutput {
            is_success: false,
            operations_applied: 0,
        };

        let human = output.to_human();
        assert_eq!(human, "Batch write failed");
    }

    #[test]
    #[cfg(feature = "sql")]
    fn test_sql_query_output_to_human_table_format() {
        use aspen_client_api::SqlCellValue;

        use crate::commands::sql::OutputFormat;

        let output = SqlQueryOutput {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec![SqlCellValue::Integer(1), SqlCellValue::Text("Alice".to_string())],
                vec![SqlCellValue::Integer(2), SqlCellValue::Text("Bob".to_string())],
            ],
            row_count: 2,
            is_truncated: false,
            execution_time_ms: 15,
            show_headers: true,
            format: OutputFormat::Table,
        };

        let human = output.to_human();
        assert!(human.contains("id"));
        assert!(human.contains("name"));
        assert!(human.contains("Alice"));
        assert!(human.contains("Bob"));
        assert!(human.contains("2 row(s) returned"));
        assert!(human.contains("15 ms"));
    }

    #[test]
    #[cfg(feature = "sql")]
    fn test_sql_query_output_to_human_empty() {
        use crate::commands::sql::OutputFormat;

        let output = SqlQueryOutput {
            columns: vec![],
            rows: vec![],
            row_count: 0,
            is_truncated: false,
            execution_time_ms: 5,
            show_headers: true,
            format: OutputFormat::Table,
        };

        let human = output.to_human();
        assert_eq!(human, "No rows returned (5 ms)");
    }

    #[test]
    fn test_repo_list_output_to_human() {
        let output = RepoListOutput {
            repos: vec![RepoListItem {
                id: "1234567890abcdef1234".to_string(),
                name: "my-repo".to_string(),
                description: Some("A test repository".to_string()),
                default_branch: "main".to_string(),
            }],
            count: 1,
        };

        let human = output.to_human();
        assert!(human.contains("Repositories (1)"));
        assert!(human.contains("my-repo"));
        assert!(human.contains("1234567890abcdef")); // First 16 chars of ID
        assert!(human.contains("A test repository"));
    }

    #[test]
    fn test_repo_list_output_to_human_empty() {
        let output = RepoListOutput {
            repos: vec![],
            count: 0,
        };

        let human = output.to_human();
        assert_eq!(human, "No repositories found.");
    }

    #[test]
    fn test_repo_output_to_human() {
        let output = RepoOutput {
            id: "repoabc123".to_string(),
            name: "test-repo".to_string(),
            description: Some("Description".to_string()),
            default_branch: "main".to_string(),
            delegates: vec!["d1".to_string(), "d2".to_string(), "d3".to_string()],
            threshold: 2,
            created_at_ms: 1000000,
        };

        let human = output.to_human();
        assert!(human.contains("Repository: test-repo"));
        assert!(human.contains("ID:             repoabc123"));
        assert!(human.contains("Default Branch: main"));
        assert!(human.contains("Description:    Description"));
        assert!(human.contains("Delegates:      3"));
        assert!(human.contains("Threshold:      2"));
    }

    #[test]
    fn test_commit_output_to_human_with_email() {
        let output = CommitOutput {
            hash: "commitabc123".to_string(),
            tree: "treexyz".to_string(),
            parents: vec!["parent1".to_string()],
            author_name: "Alice".to_string(),
            author_email: Some("alice@example.com".to_string()),
            message: "Fix bug\n\nDetailed description".to_string(),
            timestamp_ms: 1234567890,
        };

        let human = output.to_human();
        assert!(human.contains("commit commitabc123"));
        assert!(human.contains("Author: Alice <alice@example.com>"));
        assert!(human.contains("Parent: parent1"));
        assert!(human.contains("Fix bug"));
        assert!(human.contains("Detailed description"));
    }

    #[test]
    fn test_log_output_to_human_multiple_commits() {
        let output = LogOutput {
            commits: vec![
                CommitOutput {
                    hash: "c1".to_string(),
                    tree: "t1".to_string(),
                    parents: vec![],
                    author_name: "A".to_string(),
                    author_email: None,
                    message: "M1".to_string(),
                    timestamp_ms: 1000,
                },
                CommitOutput {
                    hash: "c2".to_string(),
                    tree: "t2".to_string(),
                    parents: vec![],
                    author_name: "B".to_string(),
                    author_email: None,
                    message: "M2".to_string(),
                    timestamp_ms: 2000,
                },
            ],
            count: 2,
        };

        let human = output.to_human();
        assert!(human.contains("commit c1"));
        assert!(human.contains("commit c2"));
        assert!(human.contains("M1"));
        assert!(human.contains("M2"));
    }

    #[test]
    fn test_log_output_to_human_empty() {
        let output = LogOutput {
            commits: vec![],
            count: 0,
        };

        let human = output.to_human();
        assert_eq!(human, "No commits found");
    }

    #[test]
    fn test_ref_output_to_human() {
        let output = RefOutput {
            name: "refs/heads/feature-branch".to_string(),
            hash: "abc123def456".to_string(),
        };

        let human = output.to_human();
        assert_eq!(human, "refs/heads/feature-branch -> abc123def456");
    }

    #[test]
    fn test_issue_output_to_human_with_labels() {
        let output = IssueOutput {
            id: "issue12345678".to_string(),
            title: "Critical bug".to_string(),
            state: "open".to_string(),
            labels: vec!["bug".to_string(), "critical".to_string()],
            comment_count: 3,
            created_at_ms: 1000,
            updated_at_ms: 2000,
        };

        let human = output.to_human();
        assert!(human.contains("#issue123")); // First 8 chars of ID
        assert!(human.contains("OPEN"));
        assert!(human.contains("Critical bug"));
        assert!(human.contains("[bug, critical]"));
        assert!(human.contains("3 comments"));
    }

    #[test]
    fn test_issue_output_to_human_no_labels() {
        let output = IssueOutput {
            id: "issuexyz".to_string(),
            title: "Simple issue".to_string(),
            state: "closed".to_string(),
            labels: vec![],
            comment_count: 0,
            created_at_ms: 1000,
            updated_at_ms: 1000,
        };

        let human = output.to_human();
        assert!(human.contains("CLOSED"));
        assert!(!human.contains("[]")); // No empty labels brackets
    }

    #[test]
    fn test_issue_detail_output_to_human_complete() {
        let output = IssueDetailOutput {
            id: "detailedissue123".to_string(),
            title: "Feature request".to_string(),
            body: "Please add this feature".to_string(),
            state: "open".to_string(),
            labels: vec!["enhancement".to_string()],
            assignees: vec!["assignee123456".to_string()],
            comment_count: 1,
            created_at_ms: 1000,
            updated_at_ms: 2000,
            comments: Some(vec![CommentOutput {
                hash: "commenthash123".to_string(),
                author: "authorhash456".to_string(),
                body: "Good idea".to_string(),
                timestamp_ms: 1500,
            }]),
        };

        let human = output.to_human();
        assert!(human.contains("Issue #detailed")); // First 8 chars
        assert!(human.contains("[OPEN]"));
        assert!(human.contains("Title: Feature request"));
        assert!(human.contains("Labels: enhancement"));
        assert!(human.contains("Assignees: assignee")); // First 8 chars
        assert!(human.contains("Please add this feature"));
        assert!(human.contains("Comments:"));
        assert!(human.contains("Good idea"));
    }

    #[test]
    fn test_patch_output_to_human() {
        let output = PatchOutput {
            id: "patchabc12345678".to_string(),
            title: "Add feature".to_string(),
            state: "merged".to_string(),
            base: "basecommit".to_string(),
            head: "headcommit".to_string(),
            labels: vec!["feature".to_string()],
            revision_count: 2,
            approval_count: 1,
            created_at_ms: 1000,
            updated_at_ms: 2000,
        };

        let human = output.to_human();
        assert!(human.contains("!patchabc")); // First 8 chars
        assert!(human.contains("MERGED"));
        assert!(human.contains("Add feature"));
        assert!(human.contains("[feature]"));
        assert!(human.contains("2 revisions"));
        assert!(human.contains("1 approvals"));
    }

    #[test]
    fn test_patch_detail_output_to_human() {
        let output = PatchDetailOutput {
            id: "patchdetail123".to_string(),
            title: "Big change".to_string(),
            description: "This changes everything".to_string(),
            state: "approved".to_string(),
            base: "basecommitabc123".to_string(),
            head: "headcommitdef456".to_string(),
            labels: vec![],
            assignees: vec![],
            revision_count: 1,
            approval_count: 1,
            created_at_ms: 1000,
            updated_at_ms: 2000,
            comments: None,
            revisions: None,
            approvals: Some(vec![ApprovalOutput {
                author: "approver12345678".to_string(),
                commit: "approvedcommit123".to_string(),
                message: Some("LGTM".to_string()),
                timestamp_ms: 1500,
            }]),
        };

        let human = output.to_human();
        assert!(human.contains("Patch !patchdet")); // First 8 chars
        assert!(human.contains("[APPROVED]"));
        assert!(human.contains("Title: Big change"));
        assert!(human.contains("Base:  basecommitab")); // First 12 chars
        assert!(human.contains("Head:  headcommitde")); // First 12 chars
        assert!(human.contains("This changes everything"));
        assert!(human.contains("Approvals (1):"));
        assert!(human.contains("approver")); // First 8 chars
        assert!(human.contains("LGTM"));
    }

    // =============================================================================
    // Test Category 3: Edge Cases
    // =============================================================================

    #[test]
    fn test_cluster_state_output_long_endpoint_id() {
        let output = ClusterStateOutput {
            nodes: vec![NodeInfo {
                node_id: 1,
                endpoint_id: "a".repeat(100), // Very long endpoint ID
                is_leader: true,
                is_voter: true,
            }],
        };

        let human = output.to_human();
        // Should truncate to 40 chars
        let lines: Vec<&str> = human.lines().collect();
        let data_line = lines.iter().find(|l| l.contains("1")).unwrap();
        assert!(data_line.len() < 150); // Not full 100 char endpoint
    }

    #[test]
    fn test_kv_scan_output_long_value_truncation() {
        let long_value = "x".repeat(100);
        let output = KvScanOutput {
            entries: vec![("key".to_string(), long_value.as_bytes().to_vec())],
            continuation_token: None,
        };

        let human = output.to_human();
        assert!(human.contains("..."));
        assert!(human.len() < long_value.len() + 50); // Truncated
    }

    #[test]
    fn test_kv_batch_read_output_long_value_truncation() {
        let long_value = "y".repeat(100);
        let output = KvBatchReadOutput {
            keys: vec!["key".to_string()],
            values: vec![Some(long_value.as_bytes().to_vec())],
        };

        let human = output.to_human();
        assert!(human.contains("..."));
    }

    #[test]
    fn test_repo_list_output_long_description() {
        let output = RepoListOutput {
            repos: vec![RepoListItem {
                id: "repoid1234567890abc".to_string(),
                name: "repo".to_string(),
                description: Some("a".repeat(100)),
                default_branch: "main".to_string(),
            }],
            count: 1,
        };

        let human = output.to_human();
        assert!(human.contains("..."));
        assert!(human.len() < 200); // Truncated
    }

    #[test]
    fn test_repo_output_no_description() {
        let output = RepoOutput {
            id: "id".to_string(),
            name: "repo".to_string(),
            description: None,
            default_branch: "main".to_string(),
            delegates: vec![],
            threshold: 1,
            created_at_ms: 0,
        };

        let human = output.to_human();
        assert!(human.contains("Description:    -"));
    }

    #[test]
    fn test_base64_encode_helper() {
        let data = vec![0x00, 0xFF, 0xAB, 0xCD];
        let encoded = base64_encode(&data);
        assert_eq!(encoded, "AP+rzQ==");
    }

    #[test]
    fn test_base64_encode_empty() {
        let data = vec![];
        let encoded = base64_encode(&data);
        assert_eq!(encoded, "");
    }

    // =============================================================================
    // Test Category 4: print_output() Tests
    // =============================================================================

    #[test]
    fn test_print_output_json_mode_no_panic() {
        let output = HealthOutput {
            status: "ok".to_string(),
            node_id: 1,
            raft_node_id: Some(2),
            uptime_seconds: 100,
        };

        // Just ensure it doesn't panic - we can't easily capture stdout in tests
        print_output(&output, true);
    }

    #[test]
    fn test_print_output_human_mode_no_panic() {
        let output = HealthOutput {
            status: "ok".to_string(),
            node_id: 1,
            raft_node_id: Some(2),
            uptime_seconds: 100,
        };

        // Just ensure it doesn't panic
        print_output(&output, false);
    }

    #[test]
    fn test_print_output_with_complex_type() {
        let output = ClusterStateOutput {
            nodes: vec![NodeInfo {
                node_id: 1,
                endpoint_id: "abc".to_string(),
                is_leader: true,
                is_voter: true,
            }],
        };

        print_output(&output, true);
        print_output(&output, false);
    }

    // =============================================================================
    // Test Category 5: print_success() and print_error() Tests
    // =============================================================================

    #[test]
    fn test_print_success_json_mode_no_panic() {
        print_success("Operation completed", true);
    }

    #[test]
    fn test_print_success_human_mode_no_panic() {
        print_success("Operation completed", false);
    }

    #[test]
    fn test_print_error_json_mode_no_panic() {
        print_error("Something went wrong", true);
    }

    #[test]
    fn test_print_error_human_mode_no_panic() {
        print_error("Something went wrong", false);
    }

    #[test]
    fn test_print_success_empty_message() {
        print_success("", true);
        print_success("", false);
    }

    #[test]
    fn test_print_error_empty_message() {
        print_error("", true);
        print_error("", false);
    }

    #[test]
    fn test_print_success_special_characters() {
        print_success("Success: 100% complete! 🎉", true);
        print_success("Success: 100% complete! 🎉", false);
    }

    #[test]
    fn test_print_error_special_characters() {
        print_error("Error: failed with 'quotes' and \"double quotes\"", true);
        print_error("Error: failed with 'quotes' and \"double quotes\"", false);
    }

    // =============================================================================
    // Additional Edge Case Tests
    // =============================================================================

    #[test]
    fn test_commit_output_multiline_message() {
        let output = CommitOutput {
            hash: "abc".to_string(),
            tree: "tree".to_string(),
            parents: vec![],
            author_name: "Author".to_string(),
            author_email: None,
            message: "Line 1\nLine 2\nLine 3".to_string(),
            timestamp_ms: 1000,
        };

        let human = output.to_human();
        assert!(human.contains("    Line 1"));
        assert!(human.contains("    Line 2"));
        assert!(human.contains("    Line 3"));
    }

    #[test]
    fn test_issue_detail_output_empty_comments() {
        let output = IssueDetailOutput {
            id: "issueabc12345".to_string(),
            title: "Title".to_string(),
            body: "Body".to_string(),
            state: "open".to_string(),
            labels: vec![],
            assignees: vec![],
            comment_count: 0,
            created_at_ms: 1000,
            updated_at_ms: 1000,
            comments: Some(vec![]),
        };

        let human = output.to_human();
        // Should not show comments section if empty
        assert!(!human.contains("Comments:"));
    }

    #[test]
    fn test_patch_detail_output_no_approvals() {
        let output = PatchDetailOutput {
            id: "patchabc123".to_string(),
            title: "Title".to_string(),
            description: "Desc".to_string(),
            state: "open".to_string(),
            base: "basecommit123456".to_string(),
            head: "headcommit123456".to_string(),
            labels: vec![],
            assignees: vec![],
            revision_count: 0,
            approval_count: 0,
            created_at_ms: 1000,
            updated_at_ms: 1000,
            comments: None,
            revisions: None,
            approvals: Some(vec![]),
        };

        let human = output.to_human();
        // Should not show approvals section if empty
        assert!(!human.contains("Approvals"));
    }

    #[test]
    fn test_kv_read_output_exists_but_no_value() {
        let output = KvReadOutput {
            key: "key".to_string(),
            value: None,
            does_exist: true,
        };

        let human = output.to_human();
        assert_eq!(human, "Key 'key' exists but has no value");
    }

    #[test]
    #[cfg(feature = "sql")]
    fn test_sql_query_output_all_formats() {
        use aspen_client_api::SqlCellValue;

        use crate::commands::sql::OutputFormat;

        // Test Table format
        let table_output = SqlQueryOutput {
            columns: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![
                vec![SqlCellValue::Integer(1), SqlCellValue::Text("a".to_string())],
                vec![SqlCellValue::Integer(2), SqlCellValue::Text("b".to_string())],
            ],
            row_count: 2,
            is_truncated: false,
            execution_time_ms: 10,
            show_headers: true,
            format: OutputFormat::Table,
        };
        let table_human = table_output.to_human();
        assert!(table_human.contains("+"));
        assert!(table_human.contains("|"));

        // Test TSV format
        let tsv_output = SqlQueryOutput {
            columns: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![
                vec![SqlCellValue::Integer(1), SqlCellValue::Text("a".to_string())],
                vec![SqlCellValue::Integer(2), SqlCellValue::Text("b".to_string())],
            ],
            row_count: 2,
            is_truncated: false,
            execution_time_ms: 10,
            show_headers: true,
            format: OutputFormat::Tsv,
        };
        let tsv_human = tsv_output.to_human();
        assert!(tsv_human.contains("\t"));

        // Test CSV format
        let csv_output = SqlQueryOutput {
            columns: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![
                vec![SqlCellValue::Integer(1), SqlCellValue::Text("a".to_string())],
                vec![SqlCellValue::Integer(2), SqlCellValue::Text("b".to_string())],
            ],
            row_count: 2,
            is_truncated: false,
            execution_time_ms: 10,
            show_headers: true,
            format: OutputFormat::Csv,
        };
        let csv_human = csv_output.to_human();
        assert!(csv_human.contains(","));

        // Test Vertical format
        let vertical_output = SqlQueryOutput {
            columns: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![
                vec![SqlCellValue::Integer(1), SqlCellValue::Text("a".to_string())],
                vec![SqlCellValue::Integer(2), SqlCellValue::Text("b".to_string())],
            ],
            row_count: 2,
            is_truncated: false,
            execution_time_ms: 10,
            show_headers: true,
            format: OutputFormat::Vertical,
        };
        let vertical_human = vertical_output.to_human();
        assert!(vertical_human.contains("***"));
        assert!(vertical_human.contains("1. row"));
    }

    #[test]
    #[cfg(feature = "sql")]
    fn test_sql_query_output_csv_escaping() {
        use aspen_client_api::SqlCellValue;

        use crate::commands::sql::OutputFormat;

        let output = SqlQueryOutput {
            columns: vec!["data".to_string()],
            rows: vec![
                vec![SqlCellValue::Text("has,comma".to_string())],
                vec![SqlCellValue::Text("has\"quote".to_string())],
                vec![SqlCellValue::Text("has\nnewline".to_string())],
            ],
            row_count: 3,
            is_truncated: false,
            execution_time_ms: 5,
            show_headers: true,
            format: OutputFormat::Csv,
        };

        let csv_human = output.to_human();
        // Values with special chars should be quoted
        assert!(csv_human.contains("\"has,comma\""));
        assert!(csv_human.contains("\"has\"\"quote\""));
        assert!(csv_human.contains("\"has\nnewline\""));
    }

    #[test]
    #[cfg(feature = "sql")]
    fn test_sql_query_output_truncated() {
        use aspen_client_api::SqlCellValue;

        use crate::commands::sql::OutputFormat;

        let output = SqlQueryOutput {
            columns: vec!["id".to_string()],
            rows: vec![vec![SqlCellValue::Integer(1)]],
            row_count: 1,
            is_truncated: true,
            execution_time_ms: 20,
            show_headers: true,
            format: OutputFormat::Table,
        };

        let human = output.to_human();
        assert!(human.contains("(truncated)"));
    }

    #[test]
    fn test_all_outputable_types_implement_trait() {
        // Compile-time check that all types implement Outputable
        fn assert_outputable<T: Outputable>(_: &T) {}

        let health = HealthOutput {
            status: "ok".to_string(),
            node_id: 1,
            raft_node_id: None,
            uptime_seconds: 0,
        };
        assert_outputable(&health);

        let raft = RaftMetricsOutput {
            state: "Leader".to_string(),
            current_leader: None,
            current_term: 0,
            last_log_index: 0,
            last_applied: 0,
            snapshot_index: 0,
        };
        assert_outputable(&raft);

        let cluster = ClusterStateOutput { nodes: vec![] };
        assert_outputable(&cluster);

        let kv_read = KvReadOutput {
            key: "k".to_string(),
            value: None,
            does_exist: false,
        };
        assert_outputable(&kv_read);

        let kv_scan = KvScanOutput {
            entries: vec![],
            continuation_token: None,
        };
        assert_outputable(&kv_scan);

        let batch_read = KvBatchReadOutput {
            keys: vec![],
            values: vec![],
        };
        assert_outputable(&batch_read);

        let batch_write = KvBatchWriteOutput {
            is_success: true,
            operations_applied: 0,
        };
        assert_outputable(&batch_write);

        let repo_list = RepoListOutput {
            repos: vec![],
            count: 0,
        };
        assert_outputable(&repo_list);

        let repo = RepoOutput {
            id: "id".to_string(),
            name: "name".to_string(),
            description: None,
            default_branch: "main".to_string(),
            delegates: vec![],
            threshold: 1,
            created_at_ms: 0,
        };
        assert_outputable(&repo);

        let commit = CommitOutput {
            hash: "h".to_string(),
            tree: "t".to_string(),
            parents: vec![],
            author_name: "a".to_string(),
            author_email: None,
            message: "m".to_string(),
            timestamp_ms: 0,
        };
        assert_outputable(&commit);

        let log = LogOutput {
            commits: vec![],
            count: 0,
        };
        assert_outputable(&log);

        let ref_out = RefOutput {
            name: "n".to_string(),
            hash: "h".to_string(),
        };
        assert_outputable(&ref_out);

        let ref_list = RefListOutput { refs: vec![], count: 0 };
        assert_outputable(&ref_list);

        let comment = CommentOutput {
            hash: "h".to_string(),
            author: "a".to_string(),
            body: "b".to_string(),
            timestamp_ms: 0,
        };
        assert_outputable(&comment);

        let issue = IssueOutput {
            id: "i".to_string(),
            title: "t".to_string(),
            state: "s".to_string(),
            labels: vec![],
            comment_count: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        assert_outputable(&issue);

        let issue_list = IssueListOutput {
            issues: vec![],
            count: 0,
        };
        assert_outputable(&issue_list);

        let issue_detail = IssueDetailOutput {
            id: "i".to_string(),
            title: "t".to_string(),
            body: "b".to_string(),
            state: "s".to_string(),
            labels: vec![],
            assignees: vec![],
            comment_count: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
            comments: None,
        };
        assert_outputable(&issue_detail);

        let patch = PatchOutput {
            id: "p".to_string(),
            title: "t".to_string(),
            state: "s".to_string(),
            base: "b".to_string(),
            head: "h".to_string(),
            labels: vec![],
            revision_count: 0,
            approval_count: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        assert_outputable(&patch);

        let patch_list = PatchListOutput {
            patches: vec![],
            count: 0,
        };
        assert_outputable(&patch_list);

        let patch_detail = PatchDetailOutput {
            id: "p".to_string(),
            title: "t".to_string(),
            description: "d".to_string(),
            state: "s".to_string(),
            base: "b".to_string(),
            head: "h".to_string(),
            labels: vec![],
            assignees: vec![],
            revision_count: 0,
            approval_count: 0,
            created_at_ms: 0,
            updated_at_ms: 0,
            comments: None,
            revisions: None,
            approvals: None,
        };
        assert_outputable(&patch_detail);
    }
}

/// Health status output.
pub struct HealthOutput {
    pub status: String,
    pub node_id: u64,
    pub raft_node_id: Option<u64>,
    pub uptime_seconds: u64,
}

impl Outputable for HealthOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "status": self.status,
            "node_id": self.node_id,
            "raft_node_id": self.raft_node_id,
            "uptime_seconds": self.uptime_seconds
        })
    }

    fn to_human(&self) -> String {
        let raft_id = self.raft_node_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string());

        format!(
            "Health Status\n\
             =============\n\
             Status:         {}\n\
             Node ID:        {}\n\
             Raft Node ID:   {}\n\
             Uptime:         {}s",
            self.status, self.node_id, raft_id, self.uptime_seconds
        )
    }
}

/// Raft metrics output.
pub struct RaftMetricsOutput {
    pub state: String,
    pub current_leader: Option<u64>,
    pub current_term: u64,
    pub last_log_index: u64,
    pub last_applied: u64,
    pub snapshot_index: u64,
}

impl Outputable for RaftMetricsOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "state": self.state,
            "current_leader": self.current_leader,
            "current_term": self.current_term,
            "last_log_index": self.last_log_index,
            "last_applied": self.last_applied,
            "snapshot_index": self.snapshot_index
        })
    }

    fn to_human(&self) -> String {
        let leader = self.current_leader.map(|id| id.to_string()).unwrap_or_else(|| "none".to_string());

        format!(
            "Raft Metrics\n\
             ============\n\
             State:          {}\n\
             Current Leader: {}\n\
             Current Term:   {}\n\
             Last Log Index: {}\n\
             Last Applied:   {}\n\
             Snapshot Index: {}",
            self.state, leader, self.current_term, self.last_log_index, self.last_applied, self.snapshot_index
        )
    }
}

/// Cluster state output.
pub struct ClusterStateOutput {
    pub nodes: Vec<NodeInfo>,
}

pub struct NodeInfo {
    pub node_id: u64,
    pub endpoint_id: String,
    pub is_leader: bool,
    pub is_voter: bool,
}

impl Outputable for ClusterStateOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.iter().map(|n| {
                serde_json::json!({
                    "node_id": n.node_id,
                    "endpoint_id": n.endpoint_id,
                    "is_leader": n.is_leader,
                    "is_voter": n.is_voter
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.nodes.is_empty() {
            return "No nodes in cluster".to_string();
        }

        let mut output = String::from("Cluster State\n=============\n\n");
        output.push_str("Node ID  | Leader | Voter | Endpoint ID\n");
        output.push_str("---------+--------+-------+------------------------------------------\n");

        for node in &self.nodes {
            let leader_marker = if node.is_leader { "*" } else { " " };
            let voter_marker = if node.is_voter { "Y" } else { "N" };
            output.push_str(&format!(
                "{:8} | {:6} | {:5} | {}\n",
                node.node_id,
                leader_marker,
                voter_marker,
                &node.endpoint_id[..std::cmp::min(40, node.endpoint_id.len())]
            ));
        }

        output
    }
}

/// Key-value read result output.
pub struct KvReadOutput {
    pub key: String,
    pub value: Option<Vec<u8>>,
    pub does_exist: bool,
}

impl Outputable for KvReadOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "key": self.key,
            "does_exist": self.does_exist,
            "value": self.value.as_ref().map(|v| {
                // Try to decode as UTF-8, fall back to base64
                String::from_utf8(v.clone())
                    .map(serde_json::Value::String)
                    .unwrap_or_else(|_| serde_json::json!({"base64": base64_encode(v)}))
            })
        })
    }

    fn to_human(&self) -> String {
        if !self.does_exist {
            return format!("Key '{}' not found", self.key);
        }

        match &self.value {
            Some(v) => {
                // Try to display as UTF-8 string
                match String::from_utf8(v.clone()) {
                    Ok(s) => s,
                    Err(_) => format!("<binary: {} bytes>", v.len()),
                }
            }
            None => format!("Key '{}' exists but has no value", self.key),
        }
    }
}

/// Key-value scan result output.
pub struct KvScanOutput {
    pub entries: Vec<(String, Vec<u8>)>,
    pub continuation_token: Option<String>,
}

impl Outputable for KvScanOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "entries": self.entries.iter().map(|(k, v)| {
                serde_json::json!({
                    "key": k,
                    "value": String::from_utf8(v.clone())
                        .map(serde_json::Value::String)
                        .unwrap_or_else(|_| serde_json::json!({"base64": base64_encode(v)}))
                })
            }).collect::<Vec<_>>(),
            "count": self.entries.len(),
            "continuation_token": self.continuation_token
        })
    }

    fn to_human(&self) -> String {
        if self.entries.is_empty() {
            return "No keys found".to_string();
        }

        let mut output = format!("Found {} key(s)\n\n", self.entries.len());

        for (key, value) in &self.entries {
            let value_str =
                String::from_utf8(value.clone()).unwrap_or_else(|_| format!("<binary: {} bytes>", value.len()));

            // Truncate long values for display
            let display_value = if value_str.len() > 60 {
                format!("{}...", &value_str[..57])
            } else {
                value_str
            };

            output.push_str(&format!("{}: {}\n", key, display_value));
        }

        if let Some(ref token) = self.continuation_token {
            output.push_str(&format!("\nMore results available. Use --token {}", token));
        }

        output
    }
}

/// Simple base64 encoding helper.
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Batch read result output.
pub struct KvBatchReadOutput {
    pub keys: Vec<String>,
    pub values: Vec<Option<Vec<u8>>>,
}

impl Outputable for KvBatchReadOutput {
    fn to_json(&self) -> serde_json::Value {
        let results: Vec<_> = self
            .keys
            .iter()
            .zip(self.values.iter())
            .map(|(key, value)| {
                serde_json::json!({
                    "key": key,
                    "does_exist": value.is_some(),
                    "value": value.as_ref().map(|v| {
                        String::from_utf8(v.clone())
                            .map(serde_json::Value::String)
                            .unwrap_or_else(|_| serde_json::json!({"base64": base64_encode(v)}))
                    })
                })
            })
            .collect();

        serde_json::json!({
            "count": self.keys.len(),
            "results": results
        })
    }

    fn to_human(&self) -> String {
        if self.keys.is_empty() {
            return "No keys requested".to_string();
        }

        let mut output = format!("Batch read {} key(s)\n\n", self.keys.len());

        for (key, value) in self.keys.iter().zip(self.values.iter()) {
            match value {
                Some(v) => {
                    let value_str =
                        String::from_utf8(v.clone()).unwrap_or_else(|_| format!("<binary: {} bytes>", v.len()));
                    let display_value = if value_str.len() > 50 {
                        format!("{}...", &value_str[..47])
                    } else {
                        value_str
                    };
                    output.push_str(&format!("{}: {}\n", key, display_value));
                }
                None => {
                    output.push_str(&format!("{}: <not found>\n", key));
                }
            }
        }

        output
    }
}

/// Batch write result output.
pub struct KvBatchWriteOutput {
    pub is_success: bool,
    pub operations_applied: u32,
}

impl Outputable for KvBatchWriteOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_success": self.is_success,
            "operations_applied": self.operations_applied
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("OK: {} operation(s) applied", self.operations_applied)
        } else {
            "Batch write failed".to_string()
        }
    }
}

/// SQL query result output.
#[cfg(feature = "sql")]
pub struct SqlQueryOutput {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<aspen_client_api::SqlCellValue>>,
    pub row_count: u32,
    pub is_truncated: bool,
    pub execution_time_ms: u64,
    pub show_headers: bool,
    pub format: crate::commands::sql::OutputFormat,
}

#[cfg(feature = "sql")]
impl Outputable for SqlQueryOutput {
    fn to_json(&self) -> serde_json::Value {
        use aspen_client_api::SqlCellValue;

        let rows: Vec<serde_json::Value> = self
            .rows
            .iter()
            .map(|row| {
                let obj: serde_json::Map<String, serde_json::Value> = self
                    .columns
                    .iter()
                    .zip(row.iter())
                    .map(|(col, val)| {
                        let json_val = match val {
                            SqlCellValue::Null => serde_json::Value::Null,
                            SqlCellValue::Integer(i) => serde_json::json!(i),
                            SqlCellValue::Real(f) => serde_json::json!(f),
                            SqlCellValue::Text(s) => serde_json::Value::String(s.clone()),
                            SqlCellValue::Blob(b64) => serde_json::json!({"base64": b64}),
                        };
                        (col.clone(), json_val)
                    })
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();

        serde_json::json!({
            "columns": self.columns,
            "rows": rows,
            "row_count": self.row_count,
            "is_truncated": self.is_truncated,
            "execution_time_ms": self.execution_time_ms
        })
    }

    fn to_human(&self) -> String {
        use crate::commands::sql::OutputFormat;

        if self.rows.is_empty() {
            return format!("No rows returned ({} ms)", self.execution_time_ms);
        }

        match self.format {
            OutputFormat::Table => self.format_table(),
            OutputFormat::Tsv => self.format_delimited('\t'),
            OutputFormat::Csv => self.format_delimited(','),
            OutputFormat::Vertical => self.format_vertical(),
        }
    }
}

#[cfg(feature = "sql")]
impl SqlQueryOutput {
    /// Format as ASCII table with borders.
    fn format_table(&self) -> String {
        // Calculate column widths
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.len()).collect();

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_str = cell.to_display_string();
                    widths[i] = widths[i].max(cell_str.len().min(50)); // Cap at 50 chars
                }
            }
        }

        let mut output = String::new();

        // Header separator
        let separator: String = widths.iter().map(|w| "-".repeat(*w + 2)).collect::<Vec<_>>().join("+");
        let separator = format!("+{}+", separator);

        // Header row
        if self.show_headers {
            output.push_str(&separator);
            output.push('\n');

            let header: String = self
                .columns
                .iter()
                .zip(widths.iter())
                .map(|(c, w)| format!(" {:width$} ", c, width = w))
                .collect::<Vec<_>>()
                .join("|");
            output.push_str(&format!("|{}|", header));
            output.push('\n');
            output.push_str(&separator);
            output.push('\n');
        }

        // Data rows
        for row in &self.rows {
            let row_str: String = row
                .iter()
                .zip(widths.iter())
                .map(|(cell, w)| {
                    let s = cell.to_display_string();
                    let truncated = if s.len() > 50 { format!("{}...", &s[..47]) } else { s };
                    format!(" {:width$} ", truncated, width = w)
                })
                .collect::<Vec<_>>()
                .join("|");
            output.push_str(&format!("|{}|", row_str));
            output.push('\n');
        }

        output.push_str(&separator);
        output.push('\n');

        // Footer
        output.push_str(&format!(
            "{} row(s) returned{} ({} ms)",
            self.row_count,
            if self.is_truncated { " (truncated)" } else { "" },
            self.execution_time_ms
        ));

        output
    }

    /// Format as delimiter-separated values.
    fn format_delimited(&self, delimiter: char) -> String {
        let mut output = String::new();

        // Header
        if self.show_headers {
            output.push_str(&self.columns.join(&delimiter.to_string()));
            output.push('\n');
        }

        // Data rows
        for row in &self.rows {
            let row_str: String = row
                .iter()
                .map(|cell| {
                    let s = cell.to_display_string();
                    // Escape delimiter and quotes in CSV mode
                    if delimiter == ',' && (s.contains(',') || s.contains('"') || s.contains('\n')) {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s
                    }
                })
                .collect::<Vec<_>>()
                .join(&delimiter.to_string());
            output.push_str(&row_str);
            output.push('\n');
        }

        output
    }

    /// Format in vertical mode (one column per line).
    fn format_vertical(&self) -> String {
        let mut output = String::new();
        let max_col_len = self.columns.iter().map(|c| c.len()).max().unwrap_or(0);

        for (i, row) in self.rows.iter().enumerate() {
            output.push_str(&format!("*************************** {}. row ***************************\n", i + 1));

            for (col, cell) in self.columns.iter().zip(row.iter()) {
                let value = cell.to_display_string();
                output.push_str(&format!("{:>width$}: {}\n", col, value, width = max_col_len));
            }
        }

        output.push_str(&format!(
            "{} row(s) in set{} ({} ms)",
            self.row_count,
            if self.is_truncated { " (truncated)" } else { "" },
            self.execution_time_ms
        ));

        output
    }
}

// =============================================================================
// Forge output types (decentralized git)
// =============================================================================

/// Repository list output.
pub struct RepoListOutput {
    pub repos: Vec<RepoListItem>,
    pub count: u32,
}

/// A single repository in a list.
pub struct RepoListItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
}

impl Outputable for RepoListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repos": self.repos.iter().map(|r| serde_json::json!({
                "id": r.id,
                "name": r.name,
                "description": r.description,
                "default_branch": r.default_branch
            })).collect::<Vec<_>>(),
            "count": self.count
        })
    }

    fn to_human(&self) -> String {
        if self.repos.is_empty() {
            return "No repositories found.".to_string();
        }

        let mut output = format!("Repositories ({}):\n", self.count);
        for repo in &self.repos {
            let desc = repo.description.as_deref().unwrap_or("");
            let desc_preview = if desc.len() > 40 {
                format!("{}...", &desc[..37])
            } else {
                desc.to_string()
            };
            output.push_str(&format!(
                "  {} ({}) - {}\n",
                repo.name,
                &repo.id[..16],
                if desc_preview.is_empty() { "-" } else { &desc_preview }
            ));
        }
        output
    }
}

/// Repository output.
pub struct RepoOutput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub delegates: Vec<String>,
    pub threshold: u32,
    pub created_at_ms: u64,
}

impl Outputable for RepoOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "default_branch": self.default_branch,
            "delegates": self.delegates,
            "threshold": self.threshold,
            "created_at_ms": self.created_at_ms
        })
    }

    fn to_human(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("-");
        format!(
            "Repository: {}\n\
             ID:             {}\n\
             Default Branch: {}\n\
             Description:    {}\n\
             Delegates:      {}\n\
             Threshold:      {}",
            self.name,
            self.id,
            self.default_branch,
            desc,
            self.delegates.len(),
            self.threshold
        )
    }
}

/// Commit output.
pub struct CommitOutput {
    pub hash: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub message: String,
    pub timestamp_ms: u64,
}

impl Outputable for CommitOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "hash": self.hash,
            "tree": self.tree,
            "parents": self.parents,
            "author_name": self.author_name,
            "author_email": self.author_email,
            "message": self.message,
            "timestamp_ms": self.timestamp_ms
        })
    }

    fn to_human(&self) -> String {
        let email = self.author_email.as_deref().unwrap_or("");
        let author = if email.is_empty() {
            self.author_name.clone()
        } else {
            format!("{} <{}>", self.author_name, email)
        };
        let parents = if self.parents.is_empty() {
            String::new()
        } else {
            format!("\nParent: {}", self.parents.join(", "))
        };
        format!(
            "commit {}\n\
             Author: {}\n\
             Date:   {}{}\n\n\
             {}",
            self.hash,
            author,
            self.timestamp_ms,
            parents,
            self.message.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
        )
    }
}

/// Commit log output.
pub struct LogOutput {
    pub commits: Vec<CommitOutput>,
    pub count: u32,
}

impl Outputable for LogOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "commits": self.commits.iter().map(|c| c.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.commits.is_empty() {
            return "No commits found".to_string();
        }
        self.commits.iter().map(|c| c.to_human()).collect::<Vec<_>>().join("\n\n")
    }
}

/// Ref output (branch or tag).
pub struct RefOutput {
    pub name: String,
    pub hash: String,
}

impl Outputable for RefOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "hash": self.hash
        })
    }

    fn to_human(&self) -> String {
        format!("{} -> {}", self.name, self.hash)
    }
}

/// Ref list output (branches or tags).
pub struct RefListOutput {
    pub refs: Vec<RefOutput>,
    pub count: u32,
}

impl Outputable for RefListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "refs": self.refs.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.refs.is_empty() {
            return "No refs found".to_string();
        }
        self.refs.iter().map(|r| r.to_human()).collect::<Vec<_>>().join("\n")
    }
}

/// Comment output.
pub struct CommentOutput {
    pub hash: String,
    pub author: String,
    pub body: String,
    pub timestamp_ms: u64,
}

impl Outputable for CommentOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "hash": self.hash,
            "author": self.author,
            "body": self.body,
            "timestamp_ms": self.timestamp_ms
        })
    }

    fn to_human(&self) -> String {
        format!("[{}] {}: {}", &self.hash[..8], &self.author[..8], self.body)
    }
}

/// Issue summary output (for lists).
pub struct IssueOutput {
    pub id: String,
    pub title: String,
    pub state: String,
    pub labels: Vec<String>,
    pub comment_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl Outputable for IssueOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "state": self.state,
            "labels": self.labels,
            "comment_count": self.comment_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.labels.join(", "))
        };
        format!(
            "#{} {} {}{} ({} comments)",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            labels,
            self.comment_count
        )
    }
}

/// Issue list output.
pub struct IssueListOutput {
    pub issues: Vec<IssueOutput>,
    pub count: u32,
}

impl Outputable for IssueListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "issues": self.issues.iter().map(|i| i.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.issues.is_empty() {
            return "No issues found".to_string();
        }
        self.issues.iter().map(|i| i.to_human()).collect::<Vec<_>>().join("\n")
    }
}

/// Issue detail output.
pub struct IssueDetailOutput {
    pub id: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub comment_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub comments: Option<Vec<CommentOutput>>,
}

impl Outputable for IssueDetailOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "body": self.body,
            "state": self.state,
            "labels": self.labels,
            "assignees": self.assignees,
            "comment_count": self.comment_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms,
            "comments": self.comments.as_ref().map(|cs| cs.iter().map(|c| c.to_json()).collect::<Vec<_>>())
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!("\nLabels: {}", self.labels.join(", "))
        };
        let assignees = if self.assignees.is_empty() {
            String::new()
        } else {
            format!("\nAssignees: {}", self.assignees.iter().map(|a| &a[..8]).collect::<Vec<_>>().join(", "))
        };
        let comments_str = self
            .comments
            .as_ref()
            .map(|cs| {
                if cs.is_empty() {
                    String::new()
                } else {
                    format!("\n\nComments:\n{}", cs.iter().map(|c| c.to_human()).collect::<Vec<_>>().join("\n"))
                }
            })
            .unwrap_or_default();

        format!(
            "Issue #{} [{}]\n\
             =================\n\
             Title: {}{}{}\n\n\
             {}{}",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            labels,
            assignees,
            self.body,
            comments_str
        )
    }
}

/// Patch revision output.
pub struct RevisionOutput {
    pub hash: String,
    pub head: String,
    pub message: Option<String>,
    pub author: String,
    pub timestamp_ms: u64,
}

/// Patch approval output.
pub struct ApprovalOutput {
    pub author: String,
    pub commit: String,
    pub message: Option<String>,
    pub timestamp_ms: u64,
}

/// Patch summary output (for lists).
pub struct PatchOutput {
    pub id: String,
    pub title: String,
    pub state: String,
    pub base: String,
    pub head: String,
    pub labels: Vec<String>,
    pub revision_count: u32,
    pub approval_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl Outputable for PatchOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "state": self.state,
            "base": self.base,
            "head": self.head,
            "labels": self.labels,
            "revision_count": self.revision_count,
            "approval_count": self.approval_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.labels.join(", "))
        };
        format!(
            "!{} {} {}{} ({} revisions, {} approvals)",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            labels,
            self.revision_count,
            self.approval_count
        )
    }
}

/// Patch list output.
pub struct PatchListOutput {
    pub patches: Vec<PatchOutput>,
    pub count: u32,
}

impl Outputable for PatchListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "patches": self.patches.iter().map(|p| p.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.patches.is_empty() {
            return "No patches found".to_string();
        }
        self.patches.iter().map(|p| p.to_human()).collect::<Vec<_>>().join("\n")
    }
}

/// Patch detail output.
pub struct PatchDetailOutput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: String,
    pub base: String,
    pub head: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub revision_count: u32,
    pub approval_count: u32,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub comments: Option<Vec<CommentOutput>>,
    pub revisions: Option<Vec<RevisionOutput>>,
    pub approvals: Option<Vec<ApprovalOutput>>,
}

impl Outputable for PatchDetailOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "description": self.description,
            "state": self.state,
            "base": self.base,
            "head": self.head,
            "labels": self.labels,
            "assignees": self.assignees,
            "revision_count": self.revision_count,
            "approval_count": self.approval_count,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms,
            "comments": self.comments.as_ref().map(|cs| cs.iter().map(|c| c.to_json()).collect::<Vec<_>>()),
            "revisions": self.revisions.as_ref().map(|rs| rs.iter().map(|r| {
                serde_json::json!({
                    "hash": r.hash,
                    "head": r.head,
                    "message": r.message,
                    "author": r.author,
                    "timestamp_ms": r.timestamp_ms
                })
            }).collect::<Vec<_>>()),
            "approvals": self.approvals.as_ref().map(|as_| as_.iter().map(|a| {
                serde_json::json!({
                    "author": a.author,
                    "commit": a.commit,
                    "message": a.message,
                    "timestamp_ms": a.timestamp_ms
                })
            }).collect::<Vec<_>>())
        })
    }

    fn to_human(&self) -> String {
        let labels = if self.labels.is_empty() {
            String::new()
        } else {
            format!("\nLabels: {}", self.labels.join(", "))
        };
        let assignees = if self.assignees.is_empty() {
            String::new()
        } else {
            format!("\nReviewers: {}", self.assignees.iter().map(|a| &a[..8]).collect::<Vec<_>>().join(", "))
        };
        let approvals_str = self
            .approvals
            .as_ref()
            .map(|as_| {
                if as_.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\nApprovals ({}):\n{}",
                        as_.len(),
                        as_.iter()
                            .map(|a| {
                                let msg = a.message.as_deref().unwrap_or("");
                                format!("  {} approved {}... {}", &a.author[..8], &a.commit[..8], msg)
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            })
            .unwrap_or_default();

        format!(
            "Patch !{} [{}]\n\
             =================\n\
             Title: {}\n\
             Base:  {}...\n\
             Head:  {}...{}{}\n\n\
             {}{}",
            &self.id[..8],
            self.state.to_uppercase(),
            self.title,
            &self.base[..12],
            &self.head[..12],
            labels,
            assignees,
            self.description,
            approvals_str
        )
    }
}
