//! VM-CI post-registration diagnostic classification and evidence capture.
//!
//! This module is intentionally mostly pure: tests feed bounded host/guest log
//! text and assert the highest reached boundary before the dogfood cleanup path
//! preserves redacted excerpts.

use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::path::PathBuf;

use crate::error::redact_credential_fragments;

const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const NODE_LOG_NAME: &str = "node1.log";
const VM_SERIAL_GLOB_PREFIX: &str = "ci-n1-vm";
const VM_SERIAL_GLOB_SUFFIX: &str = "-serial.log";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmCiBoundary {
    Setup,
    GuestTicketScoped,
    WorkerRegistered,
    JobAssigned,
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
            Self::WorkspaceMaterialized => "workspace_materialized",
            Self::ExecutorStarted => "executor_started",
            Self::JobResultPublished => "job_result_published",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmCiFailureClass {
    ConnectivityRegression,
    PostRegistrationCiExecution,
    Unknown,
}

impl VmCiFailureClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectivityRegression => "connectivity_regression",
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
        self.class == VmCiFailureClass::PostRegistrationCiExecution
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
        contains_any(&lower, &["ci_nix_build", "assigned job", "received job"]),
        "job_assigned",
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
        ]),
        "workspace_materialized",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::ExecutorStarted,
        contains_any(&lower, &["starting nix", "nix build", "executor started", "running command"]),
        "executor_started",
    );
    promote_if(
        &mut boundary,
        &mut evidence,
        VmCiBoundary::JobResultPublished,
        contains_any(&lower, &["job completed", "job result", "ci build completed", "status=success"]),
        "job_result_published",
    );

    let class = if boundary >= VmCiBoundary::WorkerRegistered
        || (boundary == VmCiBoundary::JobAssigned)
        || contains_any(&lower, &["ci_nix_build"])
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
    let guest_logs: Vec<String> = serial_log_paths
        .iter()
        .map(|path| read_tail_lossy(path, MAX_DIAGNOSTIC_BYTES).unwrap_or_default())
        .collect();
    let summary = classify_vmci_logs(&host_log, &guest_logs);

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

    Ok(Some(output_dir))
}

pub fn redact_log_excerpt(content: &str) -> String {
    let mut redacted = redact_credential_fragments(content);
    redacted = redact_flag_value(&redacted, "--iroh-secret-key");
    redacted = redact_flag_value(&redacted, "--cluster-ticket");
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
    fn classifies_workspace_materialization_timeout() {
        let guest =
            "worker registered with cluster\nci_nix_build\nworkspace blob fetched\nwaiting for executor".to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::WorkspaceMaterialized);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
    }

    #[test]
    fn classifies_executor_started_failure() {
        let guest = "ci_nix_build\nworkspace ready\nstarting nix build\nerror: derivation failed".to_string();
        let summary = classify_vmci_logs("", &[guest]);

        assert_eq!(summary.boundary, VmCiBoundary::ExecutorStarted);
        assert_eq!(summary.class, VmCiFailureClass::PostRegistrationCiExecution);
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
