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

    /// A grant for `action` on this reservation and attempt whose file name and single-use nonce
    /// derive from `label`.
    fn grant(&self, label: &str, action: &str, generation: u64, admitted: bool) -> PathBuf {
        self.promoted.operator_grant(&format!("{label}.json"), &signed::OperatorGrant {
            action,
            reservation_ref: self.reservation_ref.as_str(),
            attempt_ref: Some(self.attempt_ref.as_str()),
            generation,
            admitted,
            nonce_label: label,
        })
    }
}

struct OperatorGrants {
    current: PathBuf,
    stale: PathBuf,
    revoked: PathBuf,
    expired: PathBuf,
    other_action: PathBuf,
}

impl OperatorGrants {
    fn new(interrupted: &InterruptedPromotion, action: &str, other_action: &str) -> Self {
        let grant = |purpose: &str, action: &str, generation: u64, admitted: bool| {
            interrupted.grant(&format!("{purpose}-for-{action}"), action, generation, admitted)
        };
        let expired = grant("expired", action, 2, true);
        signed::edit_authority(&expired, |authority| authority.expires_at = 1);
        Self {
            current: grant("current", action, 2, true),
            stale: grant("stale", action, 1, true),
            revoked: grant("revoked", action, 2, false),
            expired,
            // Labelled by the action it is presented to, so its nonce is distinct from every
            // grant issued for `other_action` itself.
            other_action: interrupted.grant(&format!("wrong-action-for-{action}"), other_action, 2, true),
        }
    }
}

fn attempt_state(interrupted: &InterruptedPromotion) -> WorldReleaseState {
    interrupted
        .promoted
        .store()
        .read_attempt(&interrupted.attempt_ref)
        .expect("read")
        .expect("attempt")
        .state
}

fn assert_retry_planning(interrupted: &InterruptedPromotion) {
    let grants = OperatorGrants::new(interrupted, "plan-retry", "abandon-uncertain");
    for (authority, is_acknowledged) in [
        (None, true),
        (Some(grants.stale.as_path()), true),
        (Some(grants.revoked.as_path()), true),
        (Some(grants.expired.as_path()), true),
        (Some(grants.other_action.as_path()), true),
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
    // The single-use grant was consumed by the plan it authorized.
    let replayed = interrupted.retry(Some(grants.current.as_path()), true);
    assert!(!replayed.status.success());
    assert!(String::from_utf8_lossy(&replayed.stderr).contains("AuthorityNonceConsumed"));
    assert_eq!(attempt_state(interrupted), WorldReleaseState::Uncertain);
}

fn assert_abandon(interrupted: &InterruptedPromotion) {
    let grants = OperatorGrants::new(interrupted, "abandon-uncertain", "plan-retry");
    for (authority, is_acknowledged) in [
        (grants.stale.as_path(), true),
        (grants.revoked.as_path(), true),
        (grants.expired.as_path(), true),
        (grants.other_action.as_path(), true),
        (grants.current.as_path(), false),
    ] {
        assert!(!interrupted.abandon(Some(authority), is_acknowledged).status.success());
    }
    assert!(!interrupted.abandon(None, true).status.success());
    assert_eq!(attempt_state(interrupted), WorldReleaseState::Uncertain);
    let abandoned = interrupted.abandon(Some(grants.current.as_path()), true);
    assert!(abandoned.status.success(), "stderr={}", String::from_utf8_lossy(&abandoned.stderr));
    let abandoned = String::from_utf8(abandoned.stdout).expect("utf-8 stdout");
    assert!(abandoned.contains("attempt_state=abandoned\n"), "stdout={abandoned}");
    assert!(abandoned.contains("reservation_state=abandoned\n"), "stdout={abandoned}");
    assert!(abandoned.contains("external_completion_proven=false\n"), "stdout={abandoned}");
    let store = interrupted.promoted.store();
    let attempt_record = store.read_attempt(&interrupted.attempt_ref).expect("read").expect("attempt");
    assert_eq!(attempt_record.state, WorldReleaseState::Abandoned);
    assert_eq!(attempt_record.outcome, WorldAttemptOutcome::Unknown);
    // The reservation left `uncertain` with its attempt, so reconcile reports nothing unresolved.
    assert_eq!(store.list_reservations().expect("reservations")[0].state, WorldReleaseState::Abandoned);
    drop(store);
    let reconciled = run_reconcile(interrupted.promoted.temporary.path());
    assert!(reconciled.contains("unresolved_count=0\n"), "stdout={reconciled}");
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.world_promotion.dispatch]
// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
// r[verify molten.world_commit.promotion_atomic_dispatch_and_readback]
#[test]
fn retry_planning_and_abandon_after_lost_completion_require_current_operator_authority() {
    let interrupted = InterruptedPromotion::new();
    assert_retry_planning(&interrupted);
    assert_abandon(&interrupted);
}

// r[verify molten.world_promotion.reconciliation]
// r[verify molten.world_commit.promotion_authority_action_and_canonical_binding]
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
    let grant = |name: &str, action: &str, generation: u64, admitted: bool| {
        promoted.operator_grant(name, &signed::OperatorGrant {
            action,
            reservation_ref: reservation_ref.as_str(),
            attempt_ref: (action != "deny-reservation").then_some(reservation_ref.as_str()),
            generation,
            admitted,
            nonce_label: name,
        })
    };
    let refused = [
        grant("deny-stale.json", "deny-reservation", 1, true),
        grant("deny-revoked.json", "deny-reservation", 2, false),
        grant("deny-as-retry.json", "plan-retry", 2, true),
        promoted.authority_path.clone(),
    ];
    for authority in &refused {
        assert!(!deny(authority).status.success());
        assert_eq!(promoted.store().list_reservations().expect("reservations")[0].state, WorldReleaseState::Committed);
    }
    let current = grant("deny-current.json", "deny-reservation", 2, true);
    let denied = deny(&current);
    assert!(denied.status.success(), "stderr={}", String::from_utf8_lossy(&denied.stderr));
    assert!(String::from_utf8_lossy(&denied.stdout).contains("decision=denied\n"));
    assert_eq!(promoted.store().list_reservations().expect("reservations")[0].state, WorldReleaseState::Denied);
    let replayed = deny(&current);
    assert!(!replayed.status.success());
    assert!(String::from_utf8_lossy(&replayed.stderr).contains("AuthorityNonceConsumed"));
}
