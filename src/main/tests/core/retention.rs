#[test]
fn cli_retention_commands_work() {
    let dir = temp_dir("retention-cli");
    let fixture = RetentionFixture::new(dir.join("store"));
    define_retention_class(&dir, &fixture);
    admit_retention_authority(&dir, &fixture);
    let clearance = remote_clearance_roundtrip(&dir, &fixture);
    retained_clearance_is_denied(&dir, &fixture, clearance);
    pin_unpin_and_tombstone(&dir, &fixture);
    audit_retention_gc(&dir, &fixture.root);
    run_retention_fixture(&dir);
}

struct RetentionFixture {
    root: PathBuf,
    policy_ref: String,
    evidence_ref: String,
    authority_ref: String,
    owner_ref: String,
    object_ref: String,
}

impl RetentionFixture {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            policy_ref: cli_synthetic_ref("retention-policy").expect("policy ref"),
            evidence_ref: cli_synthetic_ref("retention-evidence").expect("evidence ref"),
            authority_ref: cli_synthetic_ref("retention-authority").expect("authority ref"),
            owner_ref: cli_synthetic_ref("retention-owner").expect("owner ref"),
            object_ref: cli_synthetic_ref("retention-object").expect("object ref"),
        }
    }
}

struct ClearanceFixture {
    request_out: PathBuf,
    remote_ref: String,
    peer_ref: String,
}

include!("retention/base.rs");
include!("retention/clearance.rs");
include!("retention/lifecycle.rs");
include!("retention/audit.rs");
