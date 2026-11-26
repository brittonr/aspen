use super::*;

const TAMPERED_INPUT_BYTES: &[u8] = b"tampered\n";
const ECHO_SCRIPT: &str = "IFS= read -r value; printf '%s:%s' \"$FIXTURE\" \"$value\"";

// r[verify molten.bounded_execution.live_adapter_binding_and_no_replay]
#[test]
fn live_adapter_denies_stdin_bytes_that_do_not_hash_to_the_admitted_reference() {
    let (profile, request) = canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(ECHO_SCRIPT));
    let mut adapter = Adapter::new(profile, MemoryPublisher::default()).expect("live adapter");
    let mut tampered = resolved(Some(INPUT_BYTES.to_vec()));
    tampered.stdin_bytes = Some(TAMPERED_INPUT_BYTES.to_vec());

    let failure = adapter.execute(&request, &tampered, None).expect_err("substituted stdin bytes must deny");
    assert_eq!(failure.kind, PortFailureKind::ResolutionDenied);
    assert!(failure.detail.contains("stdin bytes do not hash"), "{}", failure.detail);
    assert!(failure.process_observation.is_none());
    assert!(adapter.publisher().published.is_empty());
    assert_eq!(adapter.reconcile(HASH_B, GENERATION), ReconciliationStatus::NotFound);

    let receipt = adapter
        .execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("exact stdin bytes still execute");
    assert_eq!(receipt.process.stdout.retained_bytes, EXPECTED_STDOUT);
}

// r[verify molten.bounded_execution.live_adapter_binding_and_no_replay]
#[test]
fn live_adapter_measures_the_opened_executable_before_spawning() {
    let (profile, request) = canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(ECHO_SCRIPT));
    let mut adapter = Adapter::new(profile, MemoryPublisher::default()).expect("live adapter");
    let mut substitute = tempfile::NamedTempFile::new().expect("substitute executable");
    std::io::Write::write_all(&mut substitute, b"#!/bin/sh\nexit 0\n").expect("write substitute executable");
    let mut substituted = resolved(Some(INPUT_BYTES.to_vec()));
    substituted.executable_path = substitute.path().to_path_buf();
    assert!(substituted.executable_path.is_absolute());

    let failure = adapter
        .execute(&request, &substituted, None)
        .expect_err("an executable with other bytes must not spawn");
    assert_eq!(failure.kind, PortFailureKind::ResolutionDenied);
    assert_eq!(failure.diagnostic_code, "execution-executable-measurement-mismatch");
    assert!(failure.process_observation.is_none());
    assert!(adapter.publisher().published.is_empty());
    assert_eq!(adapter.reconcile(HASH_B, GENERATION), ReconciliationStatus::NotFound);
}

// r[verify molten.bounded_execution.live_adapter_binding_and_no_replay]
#[test]
fn live_adapter_refuses_terminal_retries_but_retries_definite_prestart_failures() {
    let (profile, request) = canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(ECHO_SCRIPT));
    let mut adapter = Adapter::new(profile, MemoryPublisher::default()).expect("live adapter");

    let mut missing = resolved(Some(INPUT_BYTES.to_vec()));
    missing.executable_path = std::path::PathBuf::from("/definitely/not/an/executable");
    let prestart = adapter.execute(&request, &missing, None).expect_err("missing executable fails before start");
    assert_eq!(prestart.kind, PortFailureKind::RejectedBeforeStart);
    assert_eq!(adapter.reconcile(HASH_B, GENERATION), ReconciliationStatus::DefinitePreStartFailure);

    let receipt = adapter
        .execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect("a definite pre-start failure may be retried");
    let published = adapter.publisher().published.len();
    assert_eq!(published, 2);

    let retry = adapter
        .execute(&request, &resolved(Some(INPUT_BYTES.to_vec())), None)
        .expect_err("terminal work is never re-executed");
    assert_eq!(retry.kind, PortFailureKind::DuplicateOperation);
    assert!(retry.process_observation.is_none());
    assert!(retry.receipt.is_none());
    assert_eq!(adapter.publisher().published.len(), published);
    assert_eq!(adapter.reconcile(HASH_B, GENERATION), ReconciliationStatus::Terminal {
        receipt_ref: receipt.receipt_ref,
    });
}

// r[verify molten.bounded_execution.live_adapter_binding_and_no_replay]
#[test]
fn live_adapter_rejects_a_fifo_executable_without_blocking() {
    let (profile, request) = canonical_request(ExecutionProfileKind::LiveBoundedProcess, script_arguments(ECHO_SCRIPT));
    let mut adapter = Adapter::new(profile, MemoryPublisher::default()).expect("live adapter");
    let directory = tempfile::tempdir().expect("fixture directory");
    let fifo = directory.path().join("executable-fifo");
    rustix::fs::mkfifoat(rustix::fs::CWD, &fifo, rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR)
        .expect("create fixture fifo");
    let mut resolved = resolved(Some(INPUT_BYTES.to_vec()));
    resolved.executable_path = fifo;

    let failure = adapter.execute(&request, &resolved, None).expect_err("a fifo is not a measurable executable");
    assert_eq!(failure.kind, PortFailureKind::RejectedBeforeStart);
    assert!(failure.detail.contains("not a regular file"), "{}", failure.detail);
}

// r[verify molten.bounded_execution.live_adapter_binding_and_no_replay]
#[test]
fn live_adapter_remeasures_an_executable_changed_after_a_cached_measurement() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().expect("fixture directory");
    let executable = directory.path().join("shell-copy");
    std::fs::write(&executable, std::fs::read(SHELL_PATH).expect("read fixture shell")).expect("copy shell");
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).expect("mark executable");
    let identity = crate::preserves_rail::content_ref_from_bytes(&std::fs::read(&executable).expect("read copy"));
    let admitted = |operation_ref: &str| {
        let mut bounded = request(script_arguments(ECHO_SCRIPT));
        bounded.operation_ref = operation_ref.to_string();
        bounded.executable_identity_ref.clone_from(&identity);
        canonicalize_request(ExecutionProfileKind::LiveBoundedProcess, bounded)
    };
    let mut context = resolved(Some(INPUT_BYTES.to_vec()));
    context.executable_path.clone_from(&executable);
    context.executable_identity_ref.clone_from(&identity);

    let (profile, first) = admitted(HASH_B);
    let mut adapter = Adapter::new(profile, MemoryPublisher::default()).expect("live adapter");
    adapter.execute(&first, &context, None).expect("measured copy executes");
    let (_, second) = admitted(HASH_C);
    adapter.execute(&second, &context, None).expect("unchanged copy reuses its measurement");

    let mut appended = std::fs::OpenOptions::new()
        .append(true)
        .open(&executable)
        .expect("open copy for an in-place change");
    std::io::Write::write_all(&mut appended, b"\n").expect("change copy in place");
    drop(appended);
    let (_, third) = admitted(HASH_D);
    let failure = adapter.execute(&third, &context, None).expect_err("a changed executable must be measured again");
    assert_eq!(failure.kind, PortFailureKind::ResolutionDenied);
    assert_eq!(failure.diagnostic_code, "execution-executable-measurement-mismatch");
}
