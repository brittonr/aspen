fn define_retention_class(dir: &Path, fixture: &RetentionFixture) {
    let class_out = dir.join("class.preserves");
    run_retention_command(RetentionCommand::Class(cli_retention::command::base::Class {
        class_name: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
        minimum_age_seconds: 0,
        maximum_age_seconds: Some(3600),
        deletion_authority_ref: fixture.authority_ref.clone(),
        policy_refs: vec![fixture.policy_ref.clone()],
        has_secret_redaction_hook: true,
        has_remote_gc_plan: true,
        has_compaction: false,
        out: Some(class_out.clone()),
    }))
    .expect("retention class");
    show_retention_artifact(class_out, "show retention class");
}

fn admit_retention_authority(dir: &Path, fixture: &RetentionFixture) {
    let admission_out = dir.join("authority-admission.preserves");
    run_retention_command(RetentionCommand::Admit(cli_retention::command::base::Admit {
        root: fixture.root.clone(),
        kind: retention::ADMISSION_KIND_AUTHORITY.to_string(),
        decision: "pass".to_string(),
        requester_ref: fixture.owner_ref.clone(),
        object_ref: fixture.object_ref.clone(),
        object_kind: "encrypted-ref".to_string(),
        retention_class: retention::CLASS_PRIVATE_SECRET_REF.to_string(),
        action: retention::ACTION_DELETE.to_string(),
        bound_refs: vec![fixture.authority_ref.clone()],
        retained_refs: Vec::new(),
        remote_refs: Vec::new(),
        is_reference_index_complete: true,
        is_stale: false,
        revoked_refs: Vec::new(),
        diagnostics: Vec::new(),
        out: Some(admission_out.clone()),
    }))
    .expect("retention admission");
    show_retention_artifact(admission_out, "show retention admission");
}

fn show_retention_artifact(artifact: PathBuf, label: &str) {
    run_retention_command(RetentionCommand::Show(cli_retention::command::ops::Show { artifact })).expect(label);
}
