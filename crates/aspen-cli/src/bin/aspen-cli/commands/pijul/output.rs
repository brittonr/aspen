//! Output types for Pijul CLI commands.
//!
//! All output structs implement the `Outputable` trait for both JSON
//! and human-readable output formatting.

use crate::output::Outputable;

/// Pijul repository output.
pub struct PijulRepoOutput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_channel: String,
    pub channel_count: u32,
    pub created_at_ms: u64,
}

impl Outputable for PijulRepoOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "default_channel": self.default_channel,
            "channel_count": self.channel_count,
            "created_at_ms": self.created_at_ms
        })
    }

    fn to_human(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("-");
        format!(
            "Repository: {}\n\
             ID:              {}\n\
             Default Channel: {}\n\
             Channels:        {}\n\
             Description:     {}",
            self.name, self.id, self.default_channel, self.channel_count, desc
        )
    }
}

/// Pijul repository list output.
pub struct PijulRepoListOutput {
    pub repos: Vec<PijulRepoOutput>,
    pub count: u32,
}

impl Outputable for PijulRepoListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "repos": self.repos.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.repos.is_empty() {
            return "No repositories found".to_string();
        }

        let mut output = format!("Repositories ({}):\n\n", self.count);
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
                &repo.id[..16.min(repo.id.len())],
                if desc_preview.is_empty() { "-" } else { &desc_preview }
            ));
        }
        output
    }
}

/// Pijul channel output.
pub struct PijulChannelOutput {
    pub name: String,
    pub head: Option<String>,
    pub updated_at_ms: u64,
}

impl Outputable for PijulChannelOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "head": self.head,
            "updated_at_ms": self.updated_at_ms
        })
    }

    fn to_human(&self) -> String {
        let head = self.head.as_deref().unwrap_or("(empty)");
        let head_short = if head.len() > 16 { &head[..16] } else { head };
        format!("{:<20} -> {}", self.name, head_short)
    }
}

/// Pijul channel list output.
pub struct PijulChannelListOutput {
    pub channels: Vec<PijulChannelOutput>,
    pub count: u32,
}

impl Outputable for PijulChannelListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "channels": self.channels.iter().map(|c| c.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.channels.is_empty() {
            return "No channels found".to_string();
        }

        let mut output = format!("Channels ({}):\n\n", self.count);
        for channel in &self.channels {
            output.push_str(&format!("  {}\n", channel.to_human()));
        }
        output
    }
}

/// Pijul record result output.
pub struct PijulRecordOutput {
    pub change_hash: String,
    pub message: String,
    pub hunks: u32,
    pub size_bytes: u64,
}

impl Outputable for PijulRecordOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "message": self.message,
            "hunks": self.hunks,
            "size_bytes": self.size_bytes
        })
    }

    fn to_human(&self) -> String {
        format!(
            "Recorded change {}\n\
             Message: {}\n\
             Hunks:   {}\n\
             Size:    {} bytes",
            &self.change_hash[..16.min(self.change_hash.len())],
            self.message,
            self.hunks,
            self.size_bytes
        )
    }
}

/// Pijul apply result output.
pub struct PijulApplyOutput {
    pub change_hash: String,
    pub channel: String,
    pub operations: u64,
}

impl Outputable for PijulApplyOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "channel": self.channel,
            "operations": self.operations
        })
    }

    fn to_human(&self) -> String {
        format!(
            "Applied {} to channel '{}' ({} operations)",
            &self.change_hash[..16.min(self.change_hash.len())],
            self.channel,
            self.operations
        )
    }
}

/// Pijul unrecord result output.
pub struct PijulUnrecordOutput {
    pub change_hash: String,
    pub channel: String,
    pub unrecorded: bool,
}

impl Outputable for PijulUnrecordOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "channel": self.channel,
            "unrecorded": self.unrecorded
        })
    }

    fn to_human(&self) -> String {
        if self.unrecorded {
            format!(
                "Unrecorded {} from channel '{}'",
                &self.change_hash[..16.min(self.change_hash.len())],
                self.channel
            )
        } else {
            format!(
                "Change {} was not in channel '{}'",
                &self.change_hash[..16.min(self.change_hash.len())],
                self.channel
            )
        }
    }
}

/// Pijul change log entry.
pub struct PijulLogEntry {
    pub change_hash: String,
    pub message: String,
    pub author: Option<String>,
    pub timestamp_ms: u64,
}

impl Outputable for PijulLogEntry {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "message": self.message,
            "author": self.author,
            "timestamp_ms": self.timestamp_ms
        })
    }

    fn to_human(&self) -> String {
        let author = self.author.as_deref().unwrap_or("unknown");
        format!(
            "change {}\n\
             Author: {}\n\n\
             {}",
            &self.change_hash[..16.min(self.change_hash.len())],
            author,
            self.message.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n")
        )
    }
}

/// Pijul log output.
pub struct PijulLogOutput {
    pub entries: Vec<PijulLogEntry>,
    pub count: u32,
}

impl Outputable for PijulLogOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "entries": self.entries.iter().map(|e| e.to_json()).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.entries.is_empty() {
            return "No changes found".to_string();
        }

        self.entries.iter().map(|e| e.to_human()).collect::<Vec<_>>().join("\n\n")
    }
}

/// Pijul checkout result output.
pub struct PijulCheckoutOutput {
    pub channel: String,
    pub output_dir: String,
    pub files_written: u32,
    pub conflicts: u32,
}

impl Outputable for PijulCheckoutOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "channel": self.channel,
            "output_dir": self.output_dir,
            "files_written": self.files_written,
            "conflicts": self.conflicts
        })
    }

    fn to_human(&self) -> String {
        if self.conflicts > 0 {
            format!(
                "Checked out '{}' to {}\n\
                 Files written: {}\n\
                 Conflicts:     {} (review manually)",
                self.channel, self.output_dir, self.files_written, self.conflicts
            )
        } else {
            format!(
                "Checked out '{}' to {}\n\
                 Files written: {}",
                self.channel, self.output_dir, self.files_written
            )
        }
    }
}

/// Pijul sync result output.
pub struct PijulSyncOutput {
    pub repo_id: String,
    pub channel: Option<String>,
    pub changes_fetched: u32,
    pub changes_applied: u32,
    pub already_synced: bool,
    pub conflicts: u32,
    pub cache_dir: String,
}

impl Outputable for PijulSyncOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "changes_fetched": self.changes_fetched,
            "changes_applied": self.changes_applied,
            "already_synced": self.already_synced,
            "conflicts": self.conflicts,
            "cache_dir": self.cache_dir
        })
    }

    fn to_human(&self) -> String {
        let channel_str = self.channel.as_deref().unwrap_or("all channels");
        if self.already_synced {
            format!(
                "Sync complete for {}\n\
                 Status:    Already up to date\n\
                 Cache:     {}",
                channel_str, self.cache_dir
            )
        } else if self.conflicts > 0 {
            format!(
                "Sync complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Conflicts: {} (review after checkout)\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.conflicts, self.cache_dir
            )
        } else {
            format!(
                "Sync complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.cache_dir
            )
        }
    }
}

/// A single hunk in a diff showing changes to a file.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Path of the file being changed.
    pub path: String,
    /// Type of change: "add", "delete", "modify", "rename", "permission".
    pub change_type: String,
    /// Number of lines added.
    pub additions: u32,
    /// Number of lines deleted.
    pub deletions: u32,
}

impl Outputable for DiffHunk {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "change_type": self.change_type,
            "additions": self.additions,
            "deletions": self.deletions
        })
    }

    fn to_human(&self) -> String {
        let symbol = match self.change_type.as_str() {
            "add" => "+",
            "delete" => "-",
            "modify" => "M",
            "rename" => "R",
            "permission" => "P",
            _ => "?",
        };
        if self.additions > 0 || self.deletions > 0 {
            format!("{} {} (+{}, -{})", symbol, self.path, self.additions, self.deletions)
        } else {
            format!("{} {}", symbol, self.path)
        }
    }
}

/// Pijul diff result output.
pub struct PijulDiffOutput {
    /// Repository ID.
    pub repo_id: String,
    /// First channel (base).
    pub channel: String,
    /// Second channel if comparing channels, None if comparing working dir.
    pub channel2: Option<String>,
    /// Working directory path if comparing working dir.
    pub working_dir: Option<String>,
    /// List of changes (hunks).
    pub hunks: Vec<DiffHunk>,
    /// Total files added.
    pub files_added: u32,
    /// Total files deleted.
    pub files_deleted: u32,
    /// Total files modified.
    pub files_modified: u32,
    /// Total lines added.
    pub lines_added: u32,
    /// Total lines deleted.
    pub lines_deleted: u32,
    /// Whether there are no changes.
    pub no_changes: bool,
}

impl Outputable for PijulDiffOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "channel2": self.channel2,
            "working_dir": self.working_dir,
            "hunks": self.hunks.iter().map(|h| h.to_json()).collect::<Vec<_>>(),
            "files_added": self.files_added,
            "files_deleted": self.files_deleted,
            "files_modified": self.files_modified,
            "lines_added": self.lines_added,
            "lines_deleted": self.lines_deleted,
            "no_changes": self.no_changes
        })
    }

    fn to_human(&self) -> String {
        if self.no_changes {
            return "No changes".to_string();
        }

        let mut output = String::new();

        // Header
        if let Some(ref channel2) = self.channel2 {
            output.push_str(&format!("Diff between channels '{}' and '{}'\n\n", self.channel, channel2));
        } else if let Some(ref working_dir) = self.working_dir {
            output.push_str(&format!("Diff of '{}' against channel '{}'\n\n", working_dir, self.channel));
        }

        // List of changes
        for hunk in &self.hunks {
            output.push_str(&format!("  {}\n", hunk.to_human()));
        }

        // Summary
        output.push_str(&format!(
            "\n{} files changed: {} added, {} deleted, {} modified\n",
            self.files_added + self.files_deleted + self.files_modified,
            self.files_added,
            self.files_deleted,
            self.files_modified
        ));
        output.push_str(&format!("{} insertions(+), {} deletions(-)", self.lines_added, self.lines_deleted));

        output
    }
}

/// Pijul archive result output.
pub struct PijulArchiveOutput {
    /// Repository ID.
    pub repo_id: String,
    /// Channel archived.
    pub channel: String,
    /// Output path.
    pub output_path: String,
    /// Archive format (directory, tar, tar.gz).
    pub format: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Number of conflicts.
    pub conflicts: u32,
}

impl Outputable for PijulArchiveOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "output_path": self.output_path,
            "format": self.format,
            "size_bytes": self.size_bytes,
            "conflicts": self.conflicts
        })
    }

    fn to_human(&self) -> String {
        if self.conflicts > 0 {
            format!(
                "Archive created for '{}'\n\
                 Output:    {}\n\
                 Format:    {}\n\
                 Size:      {} bytes\n\
                 Conflicts: {} (review manually)",
                self.channel, self.output_path, self.format, self.size_bytes, self.conflicts
            )
        } else {
            format!(
                "Archive created for '{}'\n\
                 Output:    {}\n\
                 Format:    {}\n\
                 Size:      {} bytes",
                self.channel, self.output_path, self.format, self.size_bytes
            )
        }
    }
}

/// Pijul show result output - details of a specific change.
pub struct PijulShowOutput {
    /// Full change hash (hex-encoded BLAKE3).
    pub change_hash: String,
    /// Repository ID.
    pub repo_id: String,
    /// Channel this change was recorded on.
    pub channel: String,
    /// Change message/description.
    pub message: String,
    /// Authors of the change.
    pub authors: Vec<(String, Option<String>)>, // (name, email)
    /// Hashes of changes this change depends on.
    pub dependencies: Vec<String>,
    /// Size of the change in bytes.
    pub size_bytes: u64,
    /// Timestamp when the change was recorded (milliseconds since Unix epoch).
    pub recorded_at_ms: u64,
}

impl Outputable for PijulShowOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "change_hash": self.change_hash,
            "repo_id": self.repo_id,
            "channel": self.channel,
            "message": self.message,
            "authors": self.authors.iter().map(|(name, email)| {
                serde_json::json!({ "name": name, "email": email })
            }).collect::<Vec<_>>(),
            "dependencies": self.dependencies,
            "size_bytes": self.size_bytes,
            "recorded_at_ms": self.recorded_at_ms
        })
    }

    fn to_human(&self) -> String {
        use std::time::Duration;
        use std::time::UNIX_EPOCH;

        // Format authors
        let authors_str = if self.authors.is_empty() {
            "unknown".to_string()
        } else {
            self.authors
                .iter()
                .map(|(name, email)| {
                    if let Some(e) = email {
                        format!("{} <{}>", name, e)
                    } else {
                        name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Format timestamp
        let timestamp = UNIX_EPOCH + Duration::from_millis(self.recorded_at_ms);
        let datetime = chrono::DateTime::<chrono::Utc>::from(timestamp);
        let time_str = datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        // Format dependencies
        let deps_str = if self.dependencies.is_empty() {
            "(none)".to_string()
        } else {
            self.dependencies
                .iter()
                .map(|d| format!("  {}", &d[..16.min(d.len())]))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Format message with indentation
        let message_str = self.message.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n");

        format!(
            "change {}\n\
             Author:       {}\n\
             Channel:      {}\n\
             Date:         {}\n\
             Size:         {} bytes\n\
             Dependencies:\n{}\n\n\
             {}",
            &self.change_hash[..16.min(self.change_hash.len())],
            authors_str,
            self.channel,
            time_str,
            self.size_bytes,
            deps_str,
            message_str
        )
    }
}

/// Pijul blame output - change attribution for a file.
pub struct PijulBlameOutput {
    /// File path being blamed.
    pub path: String,
    /// Channel the blame was performed on.
    pub channel: String,
    /// Repository ID.
    pub repo_id: String,
    /// List of changes that contributed to the file.
    pub attributions: Vec<BlameEntry>,
    /// Whether the file currently exists.
    pub file_exists: bool,
}

/// A single entry in blame output.
pub struct BlameEntry {
    /// Change hash (hex-encoded).
    pub change_hash: String,
    /// Author name.
    pub author: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// Change message (first line).
    pub message: String,
    /// Timestamp when recorded.
    pub recorded_at_ms: u64,
    /// Type of change.
    pub change_type: String,
}

impl Outputable for PijulBlameOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "channel": self.channel,
            "repo_id": self.repo_id,
            "file_exists": self.file_exists,
            "attributions": self.attributions.iter().map(|a| {
                serde_json::json!({
                    "change_hash": a.change_hash,
                    "author": a.author,
                    "author_email": a.author_email,
                    "message": a.message,
                    "recorded_at_ms": a.recorded_at_ms,
                    "change_type": a.change_type
                })
            }).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        use std::time::Duration;
        use std::time::UNIX_EPOCH;

        if self.attributions.is_empty() {
            return format!("No changes found for '{}' on channel '{}'", self.path, self.channel);
        }

        let mut output = format!(
            "Blame for '{}' on channel '{}'\n\
             {} change(s) found:\n\n",
            self.path,
            self.channel,
            self.attributions.len()
        );

        for attr in &self.attributions {
            let author = attr.author.as_deref().unwrap_or("unknown");
            let timestamp = UNIX_EPOCH + Duration::from_millis(attr.recorded_at_ms);
            let datetime = chrono::DateTime::<chrono::Utc>::from(timestamp);
            let time_str = datetime.format("%Y-%m-%d").to_string();

            output.push_str(&format!(
                "  {} {} {} {}\n",
                &attr.change_hash[..8.min(attr.change_hash.len())],
                author,
                time_str,
                if attr.message.is_empty() {
                    "(no message)"
                } else {
                    &attr.message
                }
            ));
        }

        output
    }
}

/// Pijul config get/set output.
pub struct PijulConfigOutput {
    /// Configuration key.
    pub key: Option<String>,
    /// Configuration value.
    pub value: Option<String>,
    /// All configuration entries (for list command).
    pub entries: Vec<(String, String)>,
    /// Whether it's a global config.
    pub global: bool,
}

impl Outputable for PijulConfigOutput {
    fn to_json(&self) -> serde_json::Value {
        if !self.entries.is_empty() {
            serde_json::json!({
                "global": self.global,
                "entries": self.entries.iter().map(|(k, v)| {
                    serde_json::json!({ "key": k, "value": v })
                }).collect::<Vec<_>>()
            })
        } else {
            serde_json::json!({
                "key": self.key,
                "value": self.value,
                "global": self.global
            })
        }
    }

    fn to_human(&self) -> String {
        if !self.entries.is_empty() {
            let scope = if self.global { "global" } else { "local" };
            let mut output = format!("Pijul configuration ({}):\n", scope);
            if self.entries.is_empty() {
                output.push_str("  (no configuration set)\n");
            } else {
                for (key, value) in &self.entries {
                    output.push_str(&format!("  {} = {}\n", key, value));
                }
            }
            output
        } else if let Some(value) = &self.value {
            value.clone()
        } else if let Some(key) = &self.key {
            format!("(not set: {})", key)
        } else {
            "(no output)".to_string()
        }
    }
}

/// Pijul remote output.
pub struct PijulRemoteOutput {
    /// Remote name (for single remote display).
    pub name: Option<String>,
    /// Remote URL (for single remote display).
    pub url: Option<String>,
    /// All remotes (for list display).
    pub remotes: Vec<(String, String)>,
}

impl Outputable for PijulRemoteOutput {
    fn to_json(&self) -> serde_json::Value {
        if !self.remotes.is_empty() {
            serde_json::json!({
                "remotes": self.remotes.iter().map(|(name, url)| {
                    serde_json::json!({ "name": name, "url": url })
                }).collect::<Vec<_>>()
            })
        } else {
            serde_json::json!({
                "name": self.name,
                "url": self.url
            })
        }
    }

    fn to_human(&self) -> String {
        if !self.remotes.is_empty() {
            let mut output = String::from("Remotes:\n");
            if self.remotes.is_empty() {
                output.push_str("  (no remotes configured)\n");
            } else {
                for (name, url) in &self.remotes {
                    output.push_str(&format!("  {} -> {}\n", name, url));
                }
            }
            output
        } else if let (Some(name), Some(url)) = (&self.name, &self.url) {
            format!("{} -> {}", name, url)
        } else if let Some(name) = &self.name {
            format!("Remote '{}' not found", name)
        } else {
            "(no output)".to_string()
        }
    }
}

// =============================================================================
// Working Directory Output Types
// =============================================================================

/// Pijul working directory init result.
pub struct PijulWdInitOutput {
    pub path: String,
    pub repo_id: String,
    pub channel: String,
    pub remote: Option<String>,
}

impl Outputable for PijulWdInitOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "repo_id": self.repo_id,
            "channel": self.channel,
            "remote": self.remote
        })
    }

    fn to_human(&self) -> String {
        let remote_str = self.remote.as_deref().unwrap_or("(none)");
        format!(
            "Initialized Pijul working directory\n\
             Path:    {}\n\
             Repo:    {}\n\
             Channel: {}\n\
             Remote:  {}",
            self.path,
            &self.repo_id[..16.min(self.repo_id.len())],
            self.channel,
            remote_str
        )
    }
}

/// Pijul working directory add result.
pub struct PijulWdAddOutput {
    pub added: Vec<String>,
    pub already_staged: Vec<String>,
    pub not_found: Vec<String>,
}

impl Outputable for PijulWdAddOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "added": self.added,
            "already_staged": self.already_staged,
            "not_found": self.not_found
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::new();

        if !self.added.is_empty() {
            output.push_str(&format!("Staged {} file(s):\n", self.added.len()));
            for path in &self.added {
                output.push_str(&format!("  + {}\n", path));
            }
        }

        if !self.already_staged.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Already staged {} file(s):\n", self.already_staged.len()));
            for path in &self.already_staged {
                output.push_str(&format!("  = {}\n", path));
            }
        }

        if !self.not_found.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Not found {} file(s):\n", self.not_found.len()));
            for path in &self.not_found {
                output.push_str(&format!("  ? {}\n", path));
            }
        }

        if output.is_empty() {
            output = "No files to stage".to_string();
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory reset result.
pub struct PijulWdResetOutput {
    pub removed: Vec<String>,
    pub not_staged: Vec<String>,
}

impl Outputable for PijulWdResetOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "removed": self.removed,
            "not_staged": self.not_staged
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::new();

        if !self.removed.is_empty() {
            output.push_str(&format!("Unstaged {} file(s):\n", self.removed.len()));
            for path in &self.removed {
                output.push_str(&format!("  - {}\n", path));
            }
        }

        if !self.not_staged.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("Not staged {} file(s):\n", self.not_staged.len()));
            for path in &self.not_staged {
                output.push_str(&format!("  ? {}\n", path));
            }
        }

        if output.is_empty() {
            output = "No files to unstage".to_string();
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory status output.
pub struct PijulWdStatusOutput {
    pub path: String,
    pub repo_id: String,
    pub channel: String,
    pub staged: Vec<String>,
    pub remote: Option<String>,
    pub last_synced_head: Option<String>,
}

impl Outputable for PijulWdStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "repo_id": self.repo_id,
            "channel": self.channel,
            "staged": self.staged,
            "remote": self.remote,
            "last_synced_head": self.last_synced_head
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!(
            "Working directory: {}\n\
             Repository: {}\n\
             Channel:    {}\n",
            self.path,
            &self.repo_id[..16.min(self.repo_id.len())],
            self.channel
        );

        if let Some(ref remote) = self.remote {
            output.push_str(&format!("Remote:     {}\n", remote));
        }

        if let Some(ref head) = self.last_synced_head {
            output.push_str(&format!("Last sync:  {}\n", &head[..16.min(head.len())]));
        }

        output.push('\n');

        if self.staged.is_empty() {
            output.push_str("No staged files\n");
        } else {
            output.push_str(&format!("Staged files ({}):\n", self.staged.len()));
            for path in &self.staged {
                output.push_str(&format!("  + {}\n", path));
            }
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory conflicts output.
pub struct PijulWdConflictsOutput {
    pub path: String,
    pub channel: String,
    pub conflicts: Vec<ConflictInfo>,
    pub total: usize,
}

/// Information about a single conflict.
pub struct ConflictInfo {
    pub file_path: String,
    pub kind: String,
    pub status: String,
    pub markers: Option<MarkerInfo>,
}

/// Conflict marker information.
pub struct MarkerInfo {
    pub start_line: u32,
    pub end_line: u32,
}

impl Outputable for PijulWdConflictsOutput {
    fn to_json(&self) -> serde_json::Value {
        let conflicts: Vec<serde_json::Value> = self
            .conflicts
            .iter()
            .map(|c| {
                let mut obj = serde_json::json!({
                    "path": c.file_path,
                    "kind": c.kind,
                    "status": c.status,
                });
                if let Some(ref m) = c.markers {
                    obj["markers"] = serde_json::json!({
                        "start_line": m.start_line,
                        "end_line": m.end_line
                    });
                }
                obj
            })
            .collect();

        serde_json::json!({
            "path": self.path,
            "channel": self.channel,
            "conflicts": conflicts,
            "total": self.total
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!(
            "Working directory: {}\n\
             Channel:           {}\n\n",
            self.path, self.channel
        );

        if self.conflicts.is_empty() {
            output.push_str("No conflicts detected\n");
        } else {
            output.push_str(&format!("Conflicts ({}):\n", self.total));
            for c in &self.conflicts {
                let status_indicator = match c.status.as_str() {
                    "unresolved" => "!",
                    "in_progress" => "~",
                    "pending" => "?",
                    "resolved" => "+",
                    _ => " ",
                };
                output.push_str(&format!("  {} {} [{}] {}\n", status_indicator, c.file_path, c.kind, c.status));
                if let Some(ref m) = c.markers {
                    output.push_str(&format!("      lines {}-{}\n", m.start_line, m.end_line));
                }
            }
        }

        output.trim_end().to_string()
    }
}

/// Pijul working directory solve output.
pub struct PijulWdSolveOutput {
    pub path: String,
    pub resolved: Vec<String>,
    pub strategy: String,
    pub remaining: usize,
}

impl Outputable for PijulWdSolveOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "resolved": self.resolved,
            "strategy": self.strategy,
            "remaining": self.remaining
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!("Working directory: {}\n\n", self.path);

        if self.resolved.is_empty() {
            output.push_str("No conflicts resolved\n");
        } else {
            output.push_str(&format!(
                "Resolved {} conflict(s) using '{}' strategy:\n",
                self.resolved.len(),
                self.strategy
            ));
            for path in &self.resolved {
                output.push_str(&format!("  + {}\n", path));
            }
        }

        if self.remaining > 0 {
            output.push_str(&format!("\n{} conflict(s) remaining\n", self.remaining));
        }

        output.trim_end().to_string()
    }
}

/// Pijul resolution strategies list output.
pub struct PijulStrategiesOutput {
    pub strategies: Vec<StrategyInfo>,
}

/// Information about a resolution strategy.
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
}

impl Outputable for PijulStrategiesOutput {
    fn to_json(&self) -> serde_json::Value {
        let strategies: Vec<serde_json::Value> = self
            .strategies
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "description": s.description
                })
            })
            .collect();

        serde_json::json!({
            "strategies": strategies
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::from("Available resolution strategies:\n\n");

        for s in &self.strategies {
            output.push_str(&format!("  {:8} - {}\n", s.name, s.description));
        }

        output.trim_end().to_string()
    }
}

/// Pijul pull result output.
pub struct PijulPullOutput {
    pub repo_id: String,
    pub channel: Option<String>,
    pub changes_fetched: u32,
    pub changes_applied: u32,
    pub already_up_to_date: bool,
    pub conflicts: u32,
    pub cache_dir: String,
}

impl Outputable for PijulPullOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "changes_fetched": self.changes_fetched,
            "changes_applied": self.changes_applied,
            "already_up_to_date": self.already_up_to_date,
            "conflicts": self.conflicts,
            "cache_dir": self.cache_dir
        })
    }

    fn to_human(&self) -> String {
        let channel_str = self.channel.as_deref().unwrap_or("all channels");
        if self.already_up_to_date {
            format!(
                "Pull complete for {}\n\
                 Status:    Already up to date\n\
                 Cache:     {}",
                channel_str, self.cache_dir
            )
        } else if self.conflicts > 0 {
            format!(
                "Pull complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Conflicts: {} (review after checkout)\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.conflicts, self.cache_dir
            )
        } else {
            format!(
                "Pull complete for {}\n\
                 Fetched:   {} changes\n\
                 Applied:   {} changes\n\
                 Cache:     {}",
                channel_str, self.changes_fetched, self.changes_applied, self.cache_dir
            )
        }
    }
}

/// Pijul push result output.
pub struct PijulPushOutput {
    pub repo_id: String,
    pub channel: String,
    pub changes_pushed: u32,
    pub already_up_to_date: bool,
    pub cache_dir: String,
}

impl Outputable for PijulPushOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repo_id": self.repo_id,
            "channel": self.channel,
            "changes_pushed": self.changes_pushed,
            "already_up_to_date": self.already_up_to_date,
            "cache_dir": self.cache_dir
        })
    }

    fn to_human(&self) -> String {
        if self.already_up_to_date {
            format!(
                "Push complete for '{}'\n\
                 Status:    Already up to date\n\
                 Cache:     {}",
                self.channel, self.cache_dir
            )
        } else {
            format!(
                "Push complete for '{}'\n\
                 Pushed:    {} changes\n\
                 Cache:     {}",
                self.channel, self.changes_pushed, self.cache_dir
            )
        }
    }
}
