pub(crate) fn explain(args: super::command::ops::Explain) -> molten::error::Result<()> {
    let super::command::ops::Explain {
        root,
        object_ref,
        object_kind,
        retention_class,
        action,
        subsystem,
        out,
    } = args;
    let explain = molten::retention::explain_candidate(molten::retention::CandidateExplainInput {
        root: &root,
        object_ref: &object_ref,
        object_kind: object_kind.as_deref(),
        retention_class: retention_class.as_deref(),
        action: action.as_deref(),
        subsystem: subsystem.as_deref(),
    })?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &explain.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention explain ref={} object={} pins={} admissions={} clearances={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
            explain.explain_ref,
            explain.object_ref,
            explain.pin_refs.len(),
            explain.admission_refs.len(),
            explain.remote_clearance_refs.len(),
            explain.gc_plan_refs.len(),
            explain.gc_apply_refs.len(),
            explain.gc_execution_refs.len(),
            explain.gc_audit_refs.len(),
            explain.retention_receipt_refs.len(),
            explain.tombstone_refs.len(),
            explain.diagnostics.len()
        ),
    );
    Ok(())
}

pub(crate) fn bundle_export(args: super::command::ops::BundleExport) -> molten::error::Result<()> {
    let super::command::ops::BundleExport {
        root,
        explain,
        out,
        profile,
    } = args;
    let explain_value = super::io::read_preserves_file(&explain)?;
    let profile = molten::retention::CandidateBundleExportProfile::parse(&profile)?;
    let bundle = molten::retention::export_candidate_bundle(molten::retention::CandidateBundleExportInput {
        root: &root,
        explain_value: &explain_value,
        out: &out,
        profile,
    })?;
    eprintln!(
        "retention bundle ref={} explain={} profile={} artifacts={} diagnostics={} out={}",
        bundle.bundle_ref,
        bundle.explain_ref,
        profile.as_str(),
        bundle.artifact_refs.len(),
        bundle.diagnostics.len(),
        out.display()
    );
    Ok(())
}

pub(crate) fn bundle_verify(args: super::command::ops::BundleVerify) -> molten::error::Result<()> {
    let super::command::ops::BundleVerify { bundle, receipt_out } = args;
    let verify = molten::retention::verify_candidate_bundle(molten::retention::CandidateBundleVerifyInput {
        bundle_dir: &bundle,
    })?;
    let text = molten::preserves_rail::to_text(&verify.value)?;
    if let Some(path) = receipt_out {
        super::io::write_file(&path, &text)?;
        eprintln!("retention bundle verify receipt {} written to {}", verify.verify_ref, path.display());
    } else {
        println!("{text}");
    }
    eprintln!(
        "retention bundle verify ref={} decision={} bundle={} files={} diagnostics={}",
        verify.verify_ref,
        verify.decision,
        verify.bundle_ref,
        verify.file_refs.len(),
        verify.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn gc_plan(args: super::command::ops::GcPlan) -> molten::error::Result<()> {
    let super::command::ops::GcPlan {
        root,
        subsystem,
        object_ref,
        object_kind,
        retention_class,
        action,
        retention,
        out,
    } = args;
    let evidence = retention.into_retention_evidence();
    let plan = molten::retention::store_gc_plan(molten::retention::GcPlanInput {
        root: &root,
        subsystem: &subsystem,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retention_class: &retention_class,
        action: &action,
        evidence: &evidence,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &plan.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention gc plan ref={} decision={} subsystem={} action={} object={} gates={} diagnostics={}",
            plan.plan_ref,
            plan.decision,
            plan.subsystem,
            plan.action,
            plan.object_ref,
            plan.gates.len(),
            plan.diagnostics.len()
        ),
    );
    Ok(())
}

pub(crate) fn gc_apply_plan(args: super::command::ops::GcApplyPlan) -> molten::error::Result<()> {
    let super::command::ops::GcApplyPlan {
        root,
        plan_ref,
        receipt_out,
    } = args;
    let apply = molten::retention::apply_gc_plan(molten::retention::GcApplyFromPlanInput {
        root: &root,
        plan_ref: &plan_ref,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &apply.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention gc apply ref={} decision={} plan={} recomputed={} receipt={} tombstone={} diagnostics={}",
            apply.apply_ref,
            apply.decision,
            apply.plan_ref,
            apply.recomputed_plan_ref,
            apply.retention_receipt_ref.as_deref().unwrap_or("none"),
            apply.tombstone_ref.as_deref().unwrap_or("none"),
            apply.diagnostics.len()
        ),
    );
    Ok(())
}

pub(crate) fn gc_audit(args: super::command::ops::GcAudit) -> molten::error::Result<()> {
    let super::command::ops::GcAudit {
        root,
        execution_ref,
        out,
    } = args;
    let audit = molten::retention::audit_gc_execution(molten::retention::GcAuditInput {
        root: &root,
        execution_ref: &execution_ref,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &audit.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention gc audit ref={} decision={} plan={} apply={} execution={} receipt={} tombstone={} diagnostics={}",
            audit.audit_ref,
            audit.decision,
            audit.plan_ref.as_deref().unwrap_or("none"),
            audit.apply_ref.as_deref().unwrap_or("none"),
            audit.execution_ref,
            audit.retention_receipt_ref.as_deref().unwrap_or("none"),
            audit.tombstone_ref.as_deref().unwrap_or("none"),
            audit.diagnostics.len()
        ),
    );
    Ok(())
}

pub(crate) fn check(args: super::command::ops::Check) -> molten::error::Result<()> {
    let super::command::ops::Check {
        root,
        object_ref,
        object_kind,
        retention_class,
        action,
        requester_ref,
        is_reference_index_complete,
        retained_refs,
        remote_refs,
        policy_refs,
        evidence_refs,
        has_delete_authority,
        has_remote_gc_clearance,
        receipt_out,
    } = args;
    let evaluation = molten::retention::evaluate(molten::retention::EvaluationInput {
        root: &root,
        object_ref: &object_ref,
        object_kind: &object_kind,
        retention_class: &retention_class,
        action: &action,
        requester_ref: &requester_ref,
        is_reference_index_complete,
        retained_refs: &retained_refs,
        remote_refs: &remote_refs,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        has_delete_authority,
        has_remote_gc_clearance,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &evaluation.receipt.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention decision={} action={} object={} receipt={} tombstone={}",
            evaluation.receipt.decision,
            evaluation.receipt.action,
            evaluation.receipt.object_ref,
            evaluation.receipt.receipt_ref,
            evaluation.receipt.tombstone_ref.as_deref().unwrap_or("none")
        ),
    );
    Ok(())
}

pub(crate) fn run_fixture(args: super::command::ops::RunFixture) -> molten::error::Result<()> {
    let super::command::ops::RunFixture { out } = args;
    let artifacts = molten::retention::run_fixture(&out)?;
    println!("retention fixture artifacts={} out={}", artifacts.len(), out.display());
    Ok(())
}

pub(crate) fn show(args: super::command::ops::Show) -> molten::error::Result<()> {
    let super::command::ops::Show { artifact } = args;
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::retention::summary(&value)?);
    Ok(())
}
