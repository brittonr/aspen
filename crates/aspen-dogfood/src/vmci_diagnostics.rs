//! VM-CI post-registration diagnostic classification and evidence capture.
//!
//! This module is intentionally mostly pure: tests feed bounded host/guest log
//! text and assert the highest reached boundary before the dogfood cleanup path
//! preserves redacted excerpts.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs;
use std::io::BufRead as _;
use std::io::Read as _;
use std::path::Path;
use std::path::PathBuf;

use crate::error::redact_credential_fragments;

const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_PROGRESS_MARKER_BYTES: usize = 64 * 1024;
const MAX_PROGRESS_MARKER_LINES: usize = 512;
const NODE_LOG_NAME: &str = "node1.log";
const VM_SERIAL_GLOB_PREFIX: &str = "ci-n1-vm";
const VM_SERIAL_GLOB_SUFFIX: &str = "-serial.log";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmCiBoundary {
    Setup,
    GuestTicketScoped,
    WorkerRegistered,
    JobAssigned,
    PreExecutor,
    WorkspaceMaterialized,
    ExecutorStarted,
    JobResultPublished,
}

impl VmCiBoundary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Setup => "setup",
            Self::GuestTicketScoped => "guest_ticket_scoped",
            Self::WorkerRegistered => "worker_registered",
            Self::JobAssigned => "job_assigned",
            Self::PreExecutor => "pre_executor",
            Self::WorkspaceMaterialized => "workspace_materialized",
            Self::ExecutorStarted => "executor_started",
            Self::JobResultPublished => "job_result_published",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmCiFailureClass {
    ConnectivityRegression,
    WorkspaceSourceMaterialization,
    NixSourceStoreFdPressure,
    PostRegistrationCiExecution,
    Unknown,
}

impl VmCiFailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectivityRegression => "connectivity_regression",
            Self::WorkspaceSourceMaterialization => "workspace_source_materialization",
            Self::NixSourceStoreFdPressure => "nix_source_store_fd_pressure",
            Self::PostRegistrationCiExecution => "post_registration_ci_execution",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmCiDiagnosticSummary {
    pub boundary: VmCiBoundary,
    pub class: VmCiFailureClass,
    pub evidence: Vec<&'static str>,
}

impl VmCiDiagnosticSummary {
    pub fn is_post_registration(&self) -> bool {
        matches!(
            self.class,
            VmCiFailureClass::PostRegistrationCiExecution
                | VmCiFailureClass::WorkspaceSourceMaterialization
                | VmCiFailureClass::NixSourceStoreFdPressure
        )
    }
}

pub fn classify_vmci_logs(host_log: &str, guest_logs: &[String]) -> VmCiDiagnosticSummary {
    let mut evidence = Vec::new();
    let mut boundary = VmCiBoundary::Setup;
    let combined_guest = guest_logs.join("\n");
    let combined = format!("{host_log}\n{combined_guest}");
    let lower = combined.to_ascii_lowercase();

    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::GuestTicketScoped,
        contains_bridge_ticket(&lower),
        "bridge_scoped_guest_ticket",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::WorkerRegistered,
        contains_any(&lower, &[
            "worker registered",
            "registered with cluster",
            "worker registration succeeded",
        ]),
        "worker_registered",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::JobAssigned,
        contains_any(&lower, &[
            "ci_nix_build",
            "assigned job",
            "received job",
            "already-running polled job",
            "invalid job state: running",
        ]),
        "job_assigned",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::PreExecutor,
        contains_any(&lower, &[
            "aspen_ci_command_progress phase=job_spec_parse_enter",
            "aspen_ci_command_progress phase=job_spec_parse_done",
            "aspen_ci_command_progress phase=job_spec_parse_timeout",
            "aspen_ci_command_progress phase=nix_payload_transform_enter",
            "aspen_ci_command_progress phase=nix_payload_transform_done",
            "aspen_ci_command_progress phase=working_dir_rewrite_enter",
            "aspen_ci_command_progress phase=working_dir_rewrite_done",
            "aspen_ci_command_progress phase=job_construct_enter",
            "aspen_ci_command_progress phase=job_construct_done",
            "aspen_ci_command_progress phase=active_log_job_enter",
            "aspen_ci_command_progress phase=active_log_job_done",
            "aspen_ci_command_progress phase=visibility_extender_spawn_enter",
            "aspen_ci_command_progress phase=visibility_extender_spawn_done",
        ]),
        "pre_executor_progress",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::WorkspaceMaterialized,
        contains_any(&lower, &[
            "workspace materialized",
            "workspace ready",
            "mounted /workspace",
            "source hash",
            "blob fetched",
            "workspace blob fetched",
            "tar extraction complete",
            "streamed blob retrieved",
            "aspen_ci_command_progress phase=workspace_materialization_enter",
            "aspen_ci_command_progress phase=source_blob_fetch_enter",
            "aspen_ci_command_progress phase=source_blob_fetch_done",
            "aspen_ci_command_progress phase=archive_decode_enter",
            "aspen_ci_command_progress phase=archive_decode_done",
            "aspen_ci_command_progress phase=workspace_unpack_enter",
            "aspen_ci_command_progress phase=workspace_unpack_done",
            "aspen_ci_command_progress phase=workspace_materialization_done",
            "aspen_ci_command_progress phase=workspace_materialization_failed",
            "aspen_ci_command_progress phase=workspace_preflight_enter",
            "aspen_ci_command_progress phase=workspace_preflight_done",
            "aspen_ci_command_progress phase=workspace_materialization_timeout",
        ]),
        "workspace_materialized",
    );
    let missing_source_root = contains_workspace_source_materialization_failure(&lower);
    if missing_source_root {
        evidence.push("workspace_source_materialization_failed");
    }
    let nix_source_fd_pressure = contains_nix_source_store_fd_pressure(&lower);
    if nix_source_fd_pressure {
        evidence.push("nix_source_store_fd_pressure");
    }
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::ExecutorStarted,
        contains_any(&lower, &[
            "starting nix",
            "nix build",
            "executor started",
            "running command",
            "aspen_ci_command_progress phase=command_started",
            "aspen_ci_command_progress phase=command_running",
            "aspen_ci_command_progress phase=command_timeout",
            "aspen_ci_command_progress phase=cache_proxy_start_enter",
            "aspen_ci_command_progress phase=cache_proxy_start_done",
            "aspen_ci_command_progress phase=cache_proxy_start_failed",
            "aspen_ci_command_progress phase=cache_proxy_start_timeout",
            "aspen_ci_command_progress phase=cache_proxy_skipped",
            "aspen_ci_command_progress phase=executor_watchdog_timeout",
            "aspen_ci_command_progress phase=command_execute_enter",
            "aspen_ci_command_progress phase=command_execute_returned",
            "aspen_ci_command_progress phase=executor_enter",
            "aspen_ci_command_progress phase=local_executor_execute_enter",
            "aspen_ci_command_progress phase=local_executor_payload_parse_enter",
            "aspen_ci_command_progress phase=local_executor_payload_parse_done",
            "aspen_ci_command_progress phase=local_executor_payload_validate_enter",
            "aspen_ci_command_progress phase=local_executor_payload_validate_done",
            "aspen_ci_command_progress phase=local_executor_execute_job_enter",
            "aspen_ci_command_progress phase=local_executor_execute_job_returned",
            "aspen_ci_command_progress phase=local_executor_execute_job_failed",
            "aspen_ci_command_progress phase=executor_job_timeout",
        ]),
        "executor_started",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::JobResultPublished,
        contains_any(&lower, &[
            "job completed",
            "job result",
            "ci build completed",
            "status=success",
            "aspen_ci_command_progress phase=result_publish_enter",
            "aspen_ci_command_progress phase=result_published",
        ]),
        "job_result_published",
    );

    let class = if nix_source_fd_pressure {
        VmCiFailureClass::NixSourceStoreFdPressure
    } else if missing_source_root {
        VmCiFailureClass::WorkspaceSourceMaterialization
    } else if boundary >= VmCiBoundary::WorkerRegistered
        || (boundary == VmCiBoundary::JobAssigned)
        || contains_any(&lower, &[
            "ci_nix_build",
            "already-running polled job",
            "invalid job state: running",
        ])
    {
        VmCiFailureClass::PostRegistrationCiExecution
    } else if contains_any(&lower, &[
        "registration timed out",
        "connection timed out",
        "no route to host",
        "network is unreachable",
        "address lookup failed",
    ]) {
        VmCiFailureClass::ConnectivityRegression
    } else {
        VmCiFailureClass::Unknown
    };

    VmCiDiagnosticSummary {
        boundary,
        class,
        evidence,
    }
}

pub fn preserve_vmci_diagnostics(
    cluster_dir: &Path,
    project_dir: &Path,
    run_id: &str,
) -> std::io::Result<Option<PathBuf>> {
    let host_log_path = cluster_dir.join(NODE_LOG_NAME);
    let serial_log_paths = serial_log_paths(cluster_dir)?;
    if !host_log_path.exists() && serial_log_paths.is_empty() {
        return Ok(None);
    }

    let host_log = read_tail_lossy(&host_log_path, MAX_DIAGNOSTIC_BYTES).unwrap_or_default();
    let mut progress_logs = Vec::new();
    if let Ok(markers) = read_progress_markers_lossy(&host_log_path) {
        progress_logs.push(markers);
    }
    let guest_logs: Vec<String> = serial_log_paths
        .iter()
        .map(|path| read_tail_lossy(path, MAX_DIAGNOSTIC_BYTES).unwrap_or_default())
        .collect();
    for path in &serial_log_paths {
        if let Ok(markers) = read_progress_markers_lossy(path) {
            progress_logs.push(markers);
        }
    }
    let mut classification_guest_logs = guest_logs.clone();
    classification_guest_logs.extend(progress_logs.iter().cloned());
    let summary = classify_vmci_logs(&host_log, &classification_guest_logs);

    let output_dir = project_dir.join("target/runtime-proof/vmci-diagnostics").join(run_id);
    fs::create_dir_all(&output_dir)?;

    let mut summary_text = String::new();
    let _ = writeln!(summary_text, "vm_ci_boundary={}", summary.boundary.as_str());
    let _ = writeln!(summary_text, "vm_ci_failure_class={}", summary.class.as_str());
    let _ = writeln!(summary_text, "post_registration={}", summary.is_post_registration());
    let _ = writeln!(summary_text, "evidence={}", summary.evidence.join(","));
    fs::write(output_dir.join("summary.txt"), summary_text)?;

    if !host_log.is_empty() {
        fs::write(output_dir.join(NODE_LOG_NAME), redact_log_excerpt(&host_log))?;
    }
    for path in serial_log_paths {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let content = read_tail_lossy(&path, MAX_DIAGNOSTIC_BYTES).unwrap_or_default();
        if !content.is_empty() {
            fs::write(output_dir.join(file_name), redact_log_excerpt(&content))?;
        }
    }
    let progress_markers =
        progress_logs.into_iter().filter(|markers| !markers.is_empty()).collect::<Vec<_>>().join("\n");
    if !progress_markers.is_empty() {
        fs::write(output_dir.join("progress-markers.txt"), redact_log_excerpt(&progress_markers))?;
    }

    Ok(Some(output_dir))
}

pub fn redact_log_excerpt(content: &str) -> String {
    let mut redacted = redact_credential_fragments(content);
    redacted = redact_flag_value(&redacted, "--iroh-secret-key");
    redacted = redact_flag_value(&redacted, "--cluster-ticket");
    redacted = redact_nix_store_source_subpaths(&redacted);
    redacted
}

fn serial_log_paths(cluster_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let vm_dir = cluster_dir.join("node1/ci/vms");
    let mut paths = Vec::new();
    let Ok(entries) = fs::read_dir(vm_dir) else {
        return Ok(paths);
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.starts_with(VM_SERIAL_GLOB_PREFIX) && file_name.ends_with(VM_SERIAL_GLOB_SUFFIX) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_tail_lossy(path: &Path, max_bytes: usize) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes as u64);
    if start > 0 {
        use std::io::Seek as _;
        file.seek(std::io::SeekFrom::Start(start))?;
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_progress_markers_lossy(path: &Path) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut retained = VecDeque::new();
    let mut retained_bytes = 0usize;
    for line in reader.lines() {
        let line = line?;
        if !line.contains("ASPEN_CI_COMMAND_PROGRESS") {
            continue;
        }
        retained_bytes = retained_bytes.saturating_add(line.len() + 1);
        retained.push_back(line);
        while retained.len() > MAX_PROGRESS_MARKER_LINES || retained_bytes > MAX_PROGRESS_MARKER_BYTES {
            if let Some(removed) = retained.pop_front() {
                retained_bytes = retained_bytes.saturating_sub(removed.len() + 1);
            } else {
                break;
            }
        }
    }
    Ok(retained.into_iter().collect::<Vec<_>>().join("\n"))
}

fn promote_if(
    boundary: &mut VmCiBoundary,
    evidence: &mut Vec<&'static str>,
    candidate: VmCiBoundary,
    present: bool,
    label: &'static str,
) {
    if present {
        *boundary = (*boundary).max(candidate);
        evidence.push(label);
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_bridge_ticket(log: &str) -> bool {
    log.contains("10.200.0.1:") || log.contains("bridge-scoped") || log.contains("bridge scoped")
}

fn contains_workspace_source_materialization_failure(log: &str) -> bool {
    contains_any(log, &[
        "does not contain a 'flake.nix'",
        "does not contain a flake.nix",
        "could not find a flake.nix",
        "source_hash_present=false",
        "source hash missing",
        "source_hash missing",
        "missing source archive",
        "workspace seeding failed",
        "workspace source materialization timed out",
        "phase=workspace_materialization_timeout",
        "phase=workspace_materialization_failed",
        "failed to download blob",
    ])
}

fn contains_nix_source_store_fd_pressure(log: &str) -> bool {
    log.contains("too many open files in system")
        && contains_any(log, &["/nix/store/", "github:nixos/nixpkgs", "nixpkgs"])
        && contains_any(log, &[
            "-source",
            "copying path",
            "unpacking 'github:",
            "chmod",
            "reading directory",
        ])
}

fn redact_flag_value(input: &str, flag: &str) -> String {
    let mut output = Vec::new();
    let mut redact_next = false;
    for token in input.split_whitespace() {
        if redact_next {
            output.push("[REDACTED]".to_string());
            redact_next = false;
            continue;
        }
        if token == flag {
            output.push(token.to_string());
            redact_next = true;
        } else if let Some((prefix, _value)) = token.split_once('=') {
            if prefix == flag {
                output.push(format!("{flag}=[REDACTED]"));
            } else {
                output.push(token.to_string());
            }
        } else {
            output.push(token.to_string());
        }
    }
    output.join(" ")
}

fn redact_nix_store_source_subpaths(input: &str) -> String {
    const STORE_PREFIX: &str = "/nix/store/";
    const SOURCE_SUFFIX: &str = "-source";
    const REDACTED_SUFFIX: &str = "/[path-redacted]";

    let mut output = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(prefix_index) = remaining.find(STORE_PREFIX) {
        output.push_str(&remaining[..prefix_index]);
        remaining = &remaining[prefix_index..];

        let Some(source_index) = remaining.find(SOURCE_SUFFIX) else {
            output.push_str(STORE_PREFIX);
            remaining = &remaining[STORE_PREFIX.len()..];
            continue;
        };
        let source_end = source_index + SOURCE_SUFFIX.len();
        output.push_str(&remaining[..source_end]);
        remaining = &remaining[source_end..];

        if !remaining.starts_with('/') {
            continue;
        }
        output.push_str(REDACTED_SUFFIX);
        let skip_to = remaining
            .char_indices()
            .find_map(|(index, ch)| (index > 0 && (ch.is_whitespace() || ch == '"' || ch == '\'')).then_some(index))
            .unwrap_or(remaining.len());
        remaining = &remaining[skip_to..];
    }
    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_connectivity_regression_before_registration() {
        let summary = classify_vmci_logs("guest ticket scoped to 10.200.0.1:1234 but registration timed out", &[]);

        assert_eq!(summary.boundary, VmCiBoundary::GuestTicketScoped);
        assert_eq!(summary.class, VmCiFailureClass::ConnectivityRegression);
    }

    #[test]
    fn classifies_job_assigned_timeout_as_post_registration() {
        let guest = "worker registered with cluster\nreceived job ci_nix_build\nworkspace ticket pending".to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::JobAssigned);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
    }

    #[test]
    fn classifies_preexecutor_marker_before_workspace_materialization() {
        let guest = concat!(
            "worker registered with cluster\n",
            "received job ci_nix_build\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=job_spec_parse_enter job_id=abc\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=job_spec_parse_timeout job_id=abc\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::PreExecutor);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
        assert!(summary.evidence.contains(&"pre_executor_progress"));
    }

    #[test]
    fn classifies_executor_enter_marker_as_executor_started() {
        let guest = concat!(
            "worker registered with cluster\n",
            "received job ci_nix_build\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=executor_enter job_id=abc\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::ExecutorStarted);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
        assert!(summary.evidence.contains(&"executor_started"));
    }

    #[test]
    fn classifies_workspace_materialization_timeout() {
        let guest =
            "worker registered with cluster\nci_nix_build\nworkspace blob fetched\nwaiting for executor".to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::WorkspaceMaterialized);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
    }

    #[test]
    fn classifies_workspace_materialization_timeout_marker_as_materialization_failure() {
        let guest = concat!(
            "worker registered with cluster\n",
            "ci_nix_build\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=source_blob_fetch_done job_id=abc bytes=42\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=workspace_materialization_timeout job_id=abc timeout_secs=120\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::WorkspaceMaterialized);
        assert_eq!(summary.class, VmCiFailureClass::WorkspaceSourceMaterialization);
        assert!(summary.evidence.contains(&"workspace_source_materialization_failed"));
    }

    #[test]
    fn classifies_executor_started_failure() {
        let guest = "ci_nix_build\nworkspace ready\nstarting nix build\nerror: derivation failed".to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::ExecutorStarted);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
    }

    #[test]
    fn classifies_command_progress_marker_as_executor_started() {
        let guest = concat!(
            "worker registered with cluster\n",
            "received job ci_nix_build\n",
            "RpcBlobStore: streamed blob retrieved size=22917755\n",
            "tar extraction complete extracted=9937 skipped=0\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=command_started job_id=abc command=nix args_count=2 timeout_secs=7200\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=command_running job_id=abc elapsed_secs=60\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=executor_watchdog_timeout job_id=abc timeout_secs=1800 grace_secs=15\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::ExecutorStarted);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
        assert!(summary.evidence.contains(&"workspace_materialized"));
        assert!(summary.evidence.contains(&"executor_started"));
    }

    #[test]
    fn classifies_execute_return_marker_as_executor_started() {
        let guest = concat!(
            "worker registered with cluster\n",
            "received job ci_nix_build\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=command_execute_enter job_id=abc\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=command_execute_returned job_id=abc\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::ExecutorStarted);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
        assert!(summary.evidence.contains(&"executor_started"));
    }

    #[test]
    fn classifies_nix_source_store_fd_pressure_after_command_start() {
        let guest = concat!(
            "worker registered with cluster\n",
            "received job ci_nix_build\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=workspace_materialization_done job_id=abc\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=command_started job_id=abc command=nix args_count=13 timeout_secs=1800\n",
            "copying path '/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source' from 'https://cache.nixos.org'...\n",
            "error: chmod \"/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source/pkgs/by-name/ad/adoptopenjdk-icedtea-web/patches\": Too many open files in system\n",
            "unpacking 'github:NixOS/nixpkgs/b86751bc4085f48661017fa226dee99fab6c651b?narHash=sha256-a8BY' into the Git cache...\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::ExecutorStarted);
        assert_eq!(summary.class, VmCiFailureClass::NixSourceStoreFdPressure);
        assert!(summary.is_post_registration());
        assert!(summary.evidence.contains(&"workspace_materialized"));
        assert!(summary.evidence.contains(&"executor_started"));
        assert!(summary.evidence.contains(&"nix_source_store_fd_pressure"));
    }

    #[test]
    fn redacts_nix_store_source_subpaths_but_keeps_source_handle() {
        let input = concat!(
            "error: chmod \"/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source/pkgs/by-name/ad/adoptopenjdk-icedtea-web/patches\": ",
            "Too many open files in system --iroh-secret-key secret-value",
        );
        let redacted = redact_log_excerpt(input);

        assert!(redacted.contains("/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source/[path-redacted]"));
        assert!(!redacted.contains("adoptopenjdk-icedtea-web"));
        assert!(!redacted.contains("secret-value"));
        assert!(redacted.contains("--iroh-secret-key [REDACTED]"));
    }

    #[test]
    fn classifies_result_publish_marker_as_job_result_published() {
        let guest = concat!(
            "worker registered with cluster\n",
            "received job ci_nix_build\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=result_publish_enter job_id=abc\n",
            "ASPEN_CI_COMMAND_PROGRESS phase=result_published job_id=abc\n",
        )
        .to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::JobResultPublished);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
        assert!(summary.evidence.contains(&"job_result_published"));
    }

    #[test]
    fn classifies_stale_running_queue_redelivery_as_post_registration() {
        let host = concat!(
            "consumed stale queue item for already-running polled job ",
            "worker_id=\"vm-worker-idle\" job_id=a061a340 ",
            "current_worker=Some(\"vm-worker-active\") ",
            "error=Invalid job state: Running for operation: mark_started",
        );

        let summary = classify_vmci_logs(host, &[]);

        assert_eq!(summary.boundary, VmCiBoundary::JobAssigned);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
        assert!(summary.is_post_registration());
    }

    #[test]
    fn classifies_missing_flake_as_workspace_source_materialization() {
        let host = "VM-CI job result published\npath '/tmp/workspaces/abc' does not contain a 'flake.nix', searching up\nerror: could not find a flake.nix file";
        let guest =
            "worker registered with cluster\nreceived job ci_nix_build\nVM-CI guest executor started".to_string();
        let summary = classify_vmci_logs(host, &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::JobResultPublished);
        assert_eq!(summary.class, VmCiFailureClass::WorkspaceSourceMaterialization);
        assert!(summary.evidence.contains(&"workspace_source_materialization_failed"));
    }

    #[test]
    fn redacts_tickets_and_secret_flags() {
        let marker = "synthetic-dogfood-ticket-marker-0123456789";
        let input = format!("remote aspen://{marker}/repo --iroh-secret-key abc123 --cluster-ticket={marker}");
        let redacted = redact_log_excerpt(&input);

        assert!(!redacted.contains(marker));
        assert!(!redacted.contains("abc123"));
        assert!(redacted.contains("aspen://<cluster-ticket>/repo"));
        assert!(redacted.contains("--iroh-secret-key [REDACTED]"));
        assert!(redacted.contains("--cluster-ticket=[REDACTED]"));
    }

    #[test]
    fn preserves_redacted_evidence_before_cleanup() {
        let unique = format!(
            "vmci-diag-test-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let cluster_dir = root.join("cluster");
        let project_dir = root.join("project");
        let vm_dir = cluster_dir.join("node1/ci/vms");
        fs::create_dir_all(&vm_dir).unwrap();
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(cluster_dir.join(NODE_LOG_NAME), "ticket 10.200.0.1:2222 --iroh-secret-key host-secret").unwrap();
        fs::write(
            vm_dir.join("ci-n1-vm0-serial.log"),
            "worker registered with cluster\nreceived job ci_nix_build\nworkspace ready\nstarting nix build",
        )
        .unwrap();

        let output = preserve_vmci_diagnostics(&cluster_dir, &project_dir, "run-1")
            .unwrap()
            .expect("diagnostics should be preserved");
        let summary = fs::read_to_string(output.join("summary.txt")).unwrap();
        let node_log = fs::read_to_string(output.join(NODE_LOG_NAME)).unwrap();

        assert!(summary.contains("vm_ci_boundary=executor_started"));
        assert!(summary.contains("vm_ci_failure_class=post_registration_ci_execution"));
        assert!(!node_log.contains("host-secret"));
        assert!(output.join("ci-n1-vm0-serial.log").exists());

        fs::remove_dir_all(root).unwrap();
    }
}
