use std::path::Path;
use std::path::PathBuf;

use super::*;

struct InterruptedPromotion {
    promoted: Promoted,
    reservation_ref: String,
    attempt_ref: WorldReleaseAttemptRef,
    next_attempt_ref: String,
}

impl InterruptedPromotion {
    fn new() -> Self {
        let promoted = Promoted::new();
        let storage = promoted.root.namespace(NamespaceKind::Storage).expect("storage");
        let (reservation_ref, attempt_ref) = interrupt_attempt(&storage);
        let reconciled = run_reconcile(promoted.temporary.path());
        assert!(reconciled.contains("unresolved_count=1\n"), "stdout={reconciled}");
        assert!(reconciled.contains("automatic_retry=false\n"), "stdout={reconciled}");
        let outbox = run_success(&["outbox-inspect", "--state-root", promoted.path()]);
        assert!(outbox.contains(&format!("reservation.uncertain={reservation_ref}\n")), "stdout={outbox}");
        assert!(outbox.contains("dispatch_authorized=false\n"), "stdout={outbox}");
        Self {
            promoted,
            reservation_ref,
            attempt_ref,
            next_attempt_ref: reference("operator-next-attempt"),
        }
    }

    fn retry(&self, authority: Option<&Path>, is_acknowledged: bool) -> std::process::Output {
        let mut arguments: Vec<&str> = vec![
            "retry-plan",
            "--state-root",
            self.promoted.path(),
            "--request",
            self.promoted.request_path.to_str().expect("path"),
            "--reservation-ref",
            self.reservation_ref.as_str(),
            "--attempt-ref",
            self.attempt_ref.as_str(),
            "--next-attempt-ref",
            self.next_attempt_ref.as_str(),
        ];
        if let Some(authority) = authority {
            arguments.extend([
                "--authority",
                authority.to_str().expect("path"),
                "--trusted-signer",
                self.promoted.signer_hex.as_str(),
            ]);
        }
        if is_acknowledged {
            arguments.push("--acknowledge-duplicate-risk");
        }
        command::run_output(&arguments)
    }

    fn abandon(&self, authority: Option<&Path>, is_acknowledged: bool) -> std::process::Output {
        let mut arguments: Vec<&str> = vec![
            "abandon",
            "--state-root",
            self.promoted.path(),
            "--request",
            self.promoted.request_path.to_str().expect("path"),
            "--attempt-ref",
            self.attempt_ref.as_str(),
        ];
        if let Some(authority) = authority {
            arguments.extend([
                "--authority",
                authority.to_str().expect("path"),
                "--trusted-signer",
                self.promoted.signer_hex.as_str(),
            ]);
        }
        if is_acknowledged {
            arguments.push("--acknowledge-unknown-outcome");
        }
        command::run_output(&arguments)
    }
}

struct OperatorGrants {
    current: PathBuf,
    stale: PathBuf,
    revoked: PathBuf,
}

impl OperatorGrants {
    fn new(promoted: &Promoted) -> Self {
        Self {
            current: promoted.operator_grant("operator-current.json", 2, true),
            stale: promoted.operator_grant("operator-stale.json", 1, true),
            revoked: promoted.operator_grant("operator-revoked.json", 2, false),
        }
    }
}

fn assert_retry_planning(interrupted: &InterruptedPromotion, grants: &OperatorGrants) {
    for (authority, is_acknowledged) in [
        (None, true),
        (Some(grants.stale.as_path()), true),
        (Some(grants.revoked.as_path()), true),
        (Some(grants.current.as_path()), false),
    ] {
        assert!(!interrupted.retry(authority, is_acknowledged).status.success());
    }
    let planned = interrupted.retry(Some(grants.current.as_path()), true);
    assert!(planned.status.success(), "stderr={}", String::from_utf8_lossy(&planned.stderr));
    let planned = String::from_utf8(planned.stdout).expect("utf-8 stdout");
    assert!(planned.contains("same_logical_release=true\n"), "stdout={planned}");
    assert!(planned.contains("duplicate_risk_acknowledged=true\n"), "stdout={planned}");
    assert!(planned.contains("external_completion_proven=false\n"), "stdout={planned}");
    assert_eq!(
        interrupted
            .promoted
            .store()
            .read_attempt(&interrupted.attempt_ref)
            .expect("read")
            .expect("attempt")
            .state,
        WorldReleaseState::Uncertain
    );
}

fn assert_abandon(interrupted: &InterruptedPromotion, grants: &OperatorGrants) {
    for (authority, is_acknowledged) in [
        (grants.stale.as_path(), true),
        (grants.revoked.as_path(), true),
        (grants.current.as_path(), false),
    ] {
        assert!(!interrupted.abandon(Some(authority), is_acknowledged).status.success());
    }
    assert!(!interrupted.abandon(None, true).status.success());
    assert_eq!(
        interrupted
            .promoted
            .store()
            .read_attempt(&interrupted.attempt_ref)
            .expect("read")
            .expect("attempt")
            .state,
        WorldReleaseState::Uncertain
    );
    let abandoned = interrupted.abandon(Some(grants.current.as_path()), true);
    assert!(abandoned.status.success(), "stderr={}", String::from_utf8_lossy(&abandoned.stderr));
    let abandoned = String::from_utf8(abandoned.stdout).expect("utf-8 stdout");
    assert!(abandoned.contains("attempt_state=abandoned\n"), "stdout={abandoned}");
    assert!(abandoned.contains("external_completion_proven=false\n"), "stdout={abandoned}");
    let attempt_record =
        interrupted.promoted.store().read_attempt(&interrupted.attempt_ref).expect("read").expect("attempt");
    assert_eq!(attempt_record.state, WorldReleaseState::Abandoned);
    assert_eq!(attempt_record.outcome, WorldAttemptOutcome::Unknown);
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.world_promotion.dispatch]
#[test]
fn retry_planning_and_abandon_after_lost_completion_require_current_operator_authority() {
    let interrupted = InterruptedPromotion::new();
    let grants = OperatorGrants::new(&interrupted.promoted);
    assert_retry_planning(&interrupted, &grants);
    assert_abandon(&interrupted, &grants);
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn deny_command_requires_current_operator_authority() {
    let promoted = Promoted::new();
    let reservation_ref = promoted.store().list_reservations().expect("reservations")[0].reservation_ref.to_string();
    let deny = |authority: &std::path::Path| {
        command::run_output(&[
            "deny",
            "--state-root",
            promoted.path(),
            "--request",
            promoted.request_path.to_str().expect("path"),
            "--reservation-ref",
            reservation_ref.as_str(),
            "--authority",
            authority.to_str().expect("path"),
            "--trusted-signer",
            promoted.signer_hex.as_str(),
        ])
    };
    for (name, generation, admitted) in [("deny-stale.json", 1, true), ("deny-revoked.json", 2, false)] {
        assert!(!deny(promoted.operator_grant(name, generation, admitted).as_path()).status.success());
        assert_eq!(promoted.store().list_reservations().expect("reservations")[0].state, WorldReleaseState::Committed);
    }
    let denied = deny(promoted.operator_grant("deny-current.json", 2, true).as_path());
    assert!(denied.status.success(), "stderr={}", String::from_utf8_lossy(&denied.stderr));
    assert!(String::from_utf8_lossy(&denied.stdout).contains("decision=denied\n"));
    assert_eq!(promoted.store().list_reservations().expect("reservations")[0].state, WorldReleaseState::Denied);
}
