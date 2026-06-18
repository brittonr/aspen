use molten::error::Result;

use super::command::base;
use super::io;

pub(crate) struct Items<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> Items<T> {
    pub(crate) fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    pub(crate) fn push(&mut self, value: T) -> Result<()> {
        if self.values.len() >= self.maximum {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "{} count exceeds {}",
                self.label, self.maximum
            )));
        }
        self.values.push(value);
        Ok(())
    }

    pub(crate) fn as_slice(&self) -> &[T] {
        &self.values
    }

    pub(crate) fn into_vec(self) -> Vec<T> {
        self.values
    }
}

impl<T: PartialEq> Items<T> {
    pub(crate) fn push_unique(&mut self, value: T) -> Result<()> {
        if !self.values.contains(&value) {
            self.push(value)?;
        }
        Ok(())
    }
}

pub(crate) fn install(args: base::Install) -> Result<()> {
    let value = io::read_preserves_file(&args.dag)?;
    let installed = molten::job_dag::install_job_dag(&args.registry, &value)?;
    if let Some(path) = args.artifact_out.as_ref() {
        io::write_file(path, &molten::preserves_rail::to_text(&value)?)?;
    }
    io::emit_named_receipt(args.receipt_out.as_ref(), "job receipt", &installed.receipt_value)?;
    println!(
        "job install {} job={} artifact={} registry={}",
        installed.decision,
        installed.job_ref,
        installed.artifact_ref,
        args.registry.display()
    );
    Ok(())
}

pub(crate) fn show(args: base::Show) -> Result<()> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(&args.registry, &args.job)?;
    println!("{}", molten::job_dag::dag_summary(&dag));
    println!("{}", molten::preserves_rail::to_text(&dag.value)?);
    Ok(())
}

pub(crate) fn run(args: base::Run) -> Result<()> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(&args.registry, &args.job)?;
    let request = args.output_request.as_ref().map(|path| io::read_preserves_file(path)).transpose()?;
    let chunk_root = args.chunks.unwrap_or_else(|| args.registry.join("job-chunks"));
    let run = molten::job_dag::run_job_dag(&dag, &molten::job_dag::JobRunOptions {
        registry_root: &args.registry,
        storage_root: &args.storage,
        cache_root: &args.cache,
        chunk_root: &chunk_root,
        ledger_root: args.ledger.as_deref(),
        output_request: request,
    })?;
    let output_text = molten::preserves_rail::to_text(&run.output_value)?;
    io::write_optional_output(args.out.as_ref(), &output_text)?;
    io::emit_named_receipt(args.receipt_out.as_ref(), "job receipt", &run.receipt_value)?;
    eprintln!(
        "job run ok job={} request={} outputs={} stages={}",
        run.job_ref,
        run.request_ref,
        run.output_refs.len(),
        run.stage_receipt_refs.len()
    );
    Ok(())
}

pub(crate) fn plan(args: base::Plan) -> Result<()> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(&args.registry, &args.job)?;
    let request = args.output_request.as_ref().map(|path| io::read_preserves_file(path)).transpose()?;
    let plan = molten::job_dag::plan_job_dag(&dag, request.as_ref())?;
    io::emit_job_analysis(&plan.value, args.out.as_ref())?;
    io::emit_named_receipt(args.receipt_out.as_ref(), "job plan receipt", &plan.receipt_value)?;
    eprintln!("job plan ok job={} plan={} stages={}", plan.job_ref, plan.plan_ref, plan.stage_order.len());
    Ok(())
}

pub(crate) fn profile(args: base::Profile) -> Result<()> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(&args.registry, &args.job)?;
    let request = args.output_request.as_ref().map(|path| io::read_preserves_file(path)).transpose()?;
    let profile = molten::job_dag::profile_job_dag(&dag, request.as_ref(), args.cache.as_deref())?;
    io::emit_job_analysis(&profile.value, args.out.as_ref())?;
    io::emit_named_receipt(args.receipt_out.as_ref(), "job profile receipt", &profile.receipt_value)?;
    eprintln!(
        "job profile ok job={} profile={} stages={} edges={}",
        profile.job_ref, profile.profile_ref, profile.stage_count, profile.edge_count
    );
    Ok(())
}

pub(crate) fn fusion_preview(args: base::FusionPreview) -> Result<()> {
    let dag = molten::job_dag::read_job_dag_file_or_registry(&args.registry, &args.job)?;
    let request = args.output_request.as_ref().map(|path| io::read_preserves_file(path)).transpose()?;
    let fusion = molten::job_dag::fusion_preview_job_dag(&dag, request.as_ref())?;
    io::emit_job_analysis(&fusion.value, args.out.as_ref())?;
    io::emit_named_receipt(args.receipt_out.as_ref(), "job fusion receipt", &fusion.receipt_value)?;
    eprintln!(
        "job fusion-preview ok job={} fusion={} chains={}",
        fusion.job_ref,
        fusion.fusion_ref,
        fusion.chains.len()
    );
    Ok(())
}
