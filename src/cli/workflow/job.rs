use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::coordination;
use molten::error::MoltenError;
use molten::error::Result;
use molten::job_dag;
use molten::ledger;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::preserves_rail::u64_value;
use molten::remote_dataspace;

const COORDINATION_CLI_BATCH_REF_LIMIT: usize = 4096;
const COORDINATION_CLI_BATCH_EVIDENCE_LIMIT: usize = 16384;
const JOB_CLI_EVIDENCE_LIMIT: usize = 64;
const JOB_WORKER_CLI_REF_LIMIT: usize = 4096;
const _: () = assert!(COORDINATION_CLI_BATCH_REF_LIMIT <= 100_000);
const _: () = assert!(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT <= 100_000);
const _: () = assert!(JOB_CLI_EVIDENCE_LIMIT <= 100_000);
const _: () = assert!(JOB_WORKER_CLI_REF_LIMIT <= 100_000);

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum JobCommand {
    Install {
        dag: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        artifact_out: Option<PathBuf>,
    },
    Show {
        job: String,
        #[arg(long)]
        registry: PathBuf,
    },
    Run {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Plan {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Profile {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    FusionPreview {
        job: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        output_request: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SyncPlan {
        job: String,
        #[arg(long)]
        source_registry: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SyncLoopback {
        job: String,
        #[arg(long)]
        source_registry: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "provenance")]
        provenance_paths: Vec<PathBuf>,
        #[arg(long = "build-verification")]
        build_verification_paths: Vec<PathBuf>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    AdmitPlan {
        job: String,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        sync_ref: Option<String>,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    AdmitLoopback {
        job: String,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        sync_ref: Option<String>,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ExecuteLoopback {
        job: String,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "capability-ref")]
        capability_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        request_out: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    WorkerRequest {
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long)]
        execution_request: PathBuf,
        #[arg(long)]
        sync_ref: Option<String>,
        #[arg(long, default_value = "peer:loopback")]
        target_peer: String,
        #[arg(long = "stage")]
        stages: Vec<String>,
        #[arg(long = "authority-ref")]
        authority_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long = "peer-bootstrap-ref")]
        peer_bootstrap_refs: Vec<String>,
        #[arg(long = "node-identity-ref")]
        node_identity_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    WorkerRunLocal {
        request: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long)]
        execution_request: PathBuf,
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long, default_value = "peer:source")]
        from_peer: String,
        #[arg(long, default_value = "source-worker")]
        from_actor: String,
        #[arg(long, default_value = "molten.job.worker")]
        topic: String,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    WorkerScheduleLocal {
        request: PathBuf,
        #[arg(long)]
        target_registry: PathBuf,
        #[arg(long)]
        storage: PathBuf,
        #[arg(long)]
        cache: PathBuf,
        #[arg(long)]
        chunks: Option<PathBuf>,
        #[arg(long)]
        admission_receipt: PathBuf,
        #[arg(long)]
        execution_request: PathBuf,
        #[arg(long)]
        transport_root: PathBuf,
        #[arg(long, default_value = "queue:job-worker")]
        queue_key: String,
        #[arg(long)]
        lease_key: Option<String>,
        #[arg(long, default_value = "scheduler")]
        scheduler_session: String,
        #[arg(long, default_value = "worker")]
        worker_session: String,
        #[arg(long)]
        lease_token: Option<u64>,
        #[arg(long, default_value = "peer:source")]
        from_peer: String,
        #[arg(long, default_value = "source-worker")]
        from_actor: String,
        #[arg(long, default_value = "molten.job.worker")]
        topic: String,
        #[arg(long = "coordination-authority-ref")]
        coordination_authority_refs: Vec<String>,
        #[arg(long = "coordination-resource-ref")]
        coordination_resource_refs: Vec<String>,
        #[arg(long = "coordination-policy-ref")]
        coordination_policy_refs: Vec<String>,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    RefSubmit {
        #[arg(long)]
        job_id: String,
        #[arg(long)]
        operation_id: String,
        #[arg(long)]
        executable: String,
        #[arg(long = "input")]
        inputs: Vec<String>,
        #[arg(long, default_value = "chunk-manifest")]
        output_mode: String,
        #[arg(long = "input-schema-ref")]
        input_schema_refs: Vec<String>,
        #[arg(long = "output-schema-ref")]
        output_schema_refs: Vec<String>,
        #[arg(long = "effect-ref")]
        effect_manifest_refs: Vec<String>,
        #[arg(long, default_value = "local-echo-v1")]
        handler_profile: String,
        #[arg(long)]
        authority_context_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "provenance-ref")]
        provenance_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    RefExecute {
        submission: PathBuf,
        #[arg(long)]
        chunks: PathBuf,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        job: Option<String>,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
}

struct CliBoundedItems<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> CliBoundedItems<T> {
    fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    fn push(&mut self, value: T) -> Result<()> {
        if self.values.len() >= self.maximum {
            return Err(MoltenError::invalid_harness(format!("{} count exceeds {}", self.label, self.maximum)));
        }
        self.values.push(value);
        Ok(())
    }

    fn into_vec(self) -> Vec<T> {
        self.values
    }
}

impl<T: PartialEq> CliBoundedItems<T> {
    fn push_unique(&mut self, value: T) -> Result<()> {
        if !self.values.contains(&value) {
            self.push(value)?;
        }
        Ok(())
    }
}

pub(crate) fn run_job_command(command: JobCommand) -> Result<()> {
    match command {
        JobCommand::Install {
            dag,
            registry,
            receipt_out,
            artifact_out,
        } => {
            let value = read_preserves_file(&dag)?;
            let installed = job_dag::install_job_dag(&registry, &value)?;
            if let Some(path) = artifact_out.as_ref() {
                write_file(path, &to_text(&value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "job receipt", &installed.receipt_value)?;
            println!(
                "job install {} job={} artifact={} registry={}",
                installed.decision,
                installed.job_ref,
                installed.artifact_ref,
                registry.display()
            );
            Ok(())
        }
        JobCommand::Show { job, registry } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            println!("{}", job_dag::dag_summary(&dag));
            println!("{}", to_text(&dag.value)?);
            Ok(())
        }
        JobCommand::Run {
            job,
            registry,
            storage,
            cache,
            chunks,
            ledger,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let chunk_root = chunks.unwrap_or_else(|| registry.join("job-chunks"));
            let run = job_dag::run_job_dag(&dag, &job_dag::JobRunOptions {
                registry_root: &registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                ledger_root: ledger.as_deref(),
                output_request: request,
            })?;
            let output_text = to_text(&run.output_value)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &output_text)?;
            } else {
                println!("{output_text}");
            }
            emit_named_receipt(receipt_out.as_ref(), "job receipt", &run.receipt_value)?;
            eprintln!(
                "job run ok job={} request={} outputs={} stages={}",
                run.job_ref,
                run.request_ref,
                run.output_refs.len(),
                run.stage_receipt_refs.len()
            );
            Ok(())
        }
        JobCommand::Plan {
            job,
            registry,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let plan = job_dag::plan_job_dag(&dag, request.as_ref())?;
            emit_job_analysis(&plan.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job plan receipt", &plan.receipt_value)?;
            eprintln!("job plan ok job={} plan={} stages={}", plan.job_ref, plan.plan_ref, plan.stage_order.len());
            Ok(())
        }
        JobCommand::Profile {
            job,
            registry,
            cache,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let profile = job_dag::profile_job_dag(&dag, request.as_ref(), cache.as_deref())?;
            emit_job_analysis(&profile.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job profile receipt", &profile.receipt_value)?;
            eprintln!(
                "job profile ok job={} profile={} stages={} edges={}",
                profile.job_ref, profile.profile_ref, profile.stage_count, profile.edge_count
            );
            Ok(())
        }
        JobCommand::FusionPreview {
            job,
            registry,
            output_request,
            out,
            receipt_out,
        } => {
            let dag = job_dag::read_job_dag_file_or_registry(&registry, &job)?;
            let request = output_request.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let fusion = job_dag::fusion_preview_job_dag(&dag, request.as_ref())?;
            emit_job_analysis(&fusion.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job fusion receipt", &fusion.receipt_value)?;
            eprintln!(
                "job fusion-preview ok job={} fusion={} chains={}",
                fusion.job_ref,
                fusion.fusion_ref,
                fusion.chains.len()
            );
            Ok(())
        }
        JobCommand::SyncPlan {
            job,
            source_registry,
            target_registry,
            target_peer,
            stages,
            out,
            receipt_out,
        } => {
            let request = job_sync_cli_request(&source_registry, &job, &stages, &target_peer, &[])?;
            let plan = job_dag::sync_plan_value(&source_registry, &target_registry, &request)?;
            emit_job_analysis(&plan.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job sync receipt", &plan.receipt_value)?;
            eprintln!(
                "job sync-plan ok job={} plan={} missing={}",
                plan.request.job_ref,
                plan.plan_ref,
                plan.missing_refs.len()
            );
            Ok(())
        }
        JobCommand::SyncLoopback {
            job,
            source_registry,
            target_registry,
            target_peer,
            stages,
            provenance_paths,
            build_verification_paths,
            plan_out,
            receipt_out,
        } => {
            let provenance_values = read_preserves_files(&provenance_paths)?;
            let build_verification_values = read_preserves_files(&build_verification_paths)?;
            let mut evidence_refs = values_canonical_refs(&provenance_values)?;
            evidence_refs.extend(values_canonical_refs(&build_verification_values)?);
            let request = job_sync_cli_request(&source_registry, &job, &stages, &target_peer, &evidence_refs)?;
            let synced = job_dag::sync_loopback(job_dag::SyncLoopbackInput {
                source_registry: &source_registry,
                target_registry: &target_registry,
                request_value: &request,
                provenance_values: &provenance_values,
                build_verification_values: &build_verification_values,
            })?;
            emit_job_analysis(&synced.plan.value, plan_out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job sync receipt", &synced.receipt_value)?;
            eprintln!(
                "job sync-loopback decision={} job={} installed={} already_present={}",
                synced.decision,
                synced.plan.request.job_ref,
                synced.installed_refs.len(),
                synced.already_present_refs.len()
            );
            Ok(())
        }
        JobCommand::AdmitPlan {
            job,
            target_registry,
            sync_ref,
            target_peer,
            stages,
            policy_refs,
            capability_refs,
            evidence_refs,
            resource_refs,
            out,
            receipt_out,
        } => {
            let request = job_admission_cli_request(JobAdmissionCliInput {
                target_registry: &target_registry,
                job: &job,
                sync_ref: sync_ref.as_deref(),
                stages: &stages,
                target_peer: &target_peer,
                policy_refs,
                capability_refs,
                evidence_refs,
                resource_refs,
            })?;
            let plan = job_dag::admission_plan_value(&target_registry, &request)?;
            emit_job_analysis(&plan.value, out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job admission receipt", &plan.receipt_value)?;
            eprintln!(
                "job admit-plan {} job={} plan={} stages={}",
                plan.decision,
                plan.request.job_ref,
                plan.plan_ref,
                plan.stage_order.len()
            );
            Ok(())
        }
        JobCommand::AdmitLoopback {
            job,
            target_registry,
            sync_ref,
            target_peer,
            stages,
            policy_refs,
            capability_refs,
            evidence_refs,
            resource_refs,
            plan_out,
            receipt_out,
        } => {
            let request = job_admission_cli_request(JobAdmissionCliInput {
                target_registry: &target_registry,
                job: &job,
                sync_ref: sync_ref.as_deref(),
                stages: &stages,
                target_peer: &target_peer,
                policy_refs,
                capability_refs,
                evidence_refs,
                resource_refs,
            })?;
            let admitted = job_dag::admission_loopback(&target_registry, &request)?;
            emit_job_analysis(&admitted.plan.value, plan_out.as_ref())?;
            emit_named_receipt(receipt_out.as_ref(), "job admission receipt", &admitted.receipt_value)?;
            eprintln!(
                "job admit-loopback {} job={} receipt={} stages={}",
                admitted.plan.decision,
                admitted.plan.request.job_ref,
                admitted.receipt_ref,
                admitted.plan.stage_order.len()
            );
            Ok(())
        }
        JobCommand::ExecuteLoopback {
            job,
            target_registry,
            storage,
            cache,
            chunks,
            admission_receipt,
            target_peer,
            stages,
            policy_refs,
            capability_refs,
            resource_refs,
            request_out,
            out,
            receipt_out,
        } => {
            let admission_value = match read_preserves_file(&admission_receipt) {
                Ok(value) => value,
                Err(error) => {
                    let request = job_execution_cli_request_from_admission_ref(JobExecutionFromAdmissionCliInput {
                        target_registry: &target_registry,
                        job: &job,
                        admission_ref: None,
                        target_peer: &target_peer,
                        stages: &stages,
                        policy_refs,
                        capability_refs,
                        resource_refs,
                    })?;
                    if let Some(path) = request_out.as_ref() {
                        write_file(path, &to_text(&request)?)?;
                    }
                    let receipt = job_dag::missing_admission_execution_receipt_value(&request, &error.to_string())?;
                    emit_named_receipt(receipt_out.as_ref(), "job execution receipt", &receipt)?;
                    return Err(error);
                }
            };
            let request = job_execution_cli_request(JobExecutionCliInput {
                target_registry: &target_registry,
                job: &job,
                admission_value: &admission_value,
                target_peer: &target_peer,
                stages: &stages,
                policy_refs,
                capability_refs,
                resource_refs,
            })?;
            if let Some(path) = request_out.as_ref() {
                write_file(path, &to_text(&request)?)?;
            }
            let chunk_root = chunks.unwrap_or_else(|| target_registry.join("job-chunks"));
            let executed = job_dag::execution_loopback(job_dag::ExecutionLoopbackInput {
                target_registry: &target_registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                admission_receipt_value: &admission_value,
                request_value: &request,
            })?;
            if let Some(run) = executed.run.as_ref() {
                let output_text = to_text(&run.output_value)?;
                if let Some(path) = out.as_ref() {
                    write_file(path, &output_text)?;
                } else {
                    println!("{output_text}");
                }
            }
            emit_named_receipt(receipt_out.as_ref(), "job execution receipt", &executed.receipt_value)?;
            if executed.decision == "pass" {
                eprintln!(
                    "job execute-loopback pass job={} receipt={} outputs={}",
                    executed.request.job_ref,
                    executed.receipt_ref,
                    executed.run.as_ref().map(|run| run.output_refs.len()).unwrap_or_default()
                );
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job execute-loopback denied: {}",
                    executed.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::WorkerRequest {
            admission_receipt,
            execution_request,
            sync_ref,
            target_peer,
            stages,
            authority_refs,
            resource_refs,
            peer_bootstrap_refs,
            node_identity_refs,
            evidence_refs,
            out,
        } => {
            let admission_value = read_preserves_file(&admission_receipt)?;
            let execution_request_value = read_preserves_file(&execution_request)?;
            let request_value = job_worker_cli_request(JobWorkerRequestCliInput {
                admission_value: &admission_value,
                execution_request_value: &execution_request_value,
                sync_ref: sync_ref.as_deref(),
                target_peer: &target_peer,
                stages: &stages,
                authority_refs,
                resource_refs,
                peer_bootstrap_refs,
                node_identity_refs,
                evidence_refs,
            })?;
            let parsed = job_dag::parse_job_worker_request_value(&request_value)?;
            emit_job_analysis(&request_value, out.as_ref())?;
            eprintln!(
                "job worker-request ok job={} request={} target={} stages={}",
                parsed.job_ref,
                parsed.request_ref,
                parsed.target_peer,
                parsed.stage_ids.len()
            );
            Ok(())
        }
        JobCommand::WorkerRunLocal {
            request,
            target_registry,
            storage,
            cache,
            chunks,
            admission_receipt,
            execution_request,
            transport_root,
            from_peer,
            from_actor,
            topic,
            ledger,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let admission_value = read_preserves_file(&admission_receipt)?;
            let execution_request_value = read_preserves_file(&execution_request)?;
            let chunk_root = chunks.unwrap_or_else(|| target_registry.join("job-chunks"));
            let executed = job_worker_run_local(JobWorkerRunLocalInput {
                request_value: &request_value,
                target_registry: &target_registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                admission_value: &admission_value,
                execution_request_value: &execution_request_value,
                transport_root: &transport_root,
                from_peer: &from_peer,
                from_actor: &from_actor,
                topic: &topic,
                ledger_root: ledger.as_deref(),
                out: &out,
            })?;
            eprintln!(
                "job worker-run-local {} job={} receipt={} result={} out={}",
                executed.result.decision,
                executed.result.job_ref,
                executed.receipt_ref,
                executed.result.result_ref,
                out.display()
            );
            if executed.result.decision == "pass" {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job worker-run-local denied: {}",
                    executed.result.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::WorkerScheduleLocal {
            request,
            target_registry,
            storage,
            cache,
            chunks,
            admission_receipt,
            execution_request,
            transport_root,
            queue_key,
            lease_key,
            scheduler_session,
            worker_session,
            lease_token,
            from_peer,
            from_actor,
            topic,
            coordination_authority_refs,
            coordination_resource_refs,
            coordination_policy_refs,
            ledger,
            out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let admission_value = read_preserves_file(&admission_receipt)?;
            let execution_request_value = read_preserves_file(&execution_request)?;
            let chunk_root = chunks.unwrap_or_else(|| target_registry.join("job-chunks"));
            let scheduled = job_worker_schedule_local(JobWorkerScheduleLocalInput {
                request_value: &request_value,
                target_registry: &target_registry,
                storage_root: &storage,
                cache_root: &cache,
                chunk_root: &chunk_root,
                admission_value: &admission_value,
                execution_request_value: &execution_request_value,
                transport_root: &transport_root,
                queue_key: &queue_key,
                lease_key: lease_key.as_deref(),
                scheduler_session: &scheduler_session,
                worker_session: &worker_session,
                lease_token,
                from_peer: &from_peer,
                from_actor: &from_actor,
                topic: &topic,
                coordination_authority_refs,
                coordination_resource_refs,
                coordination_policy_refs,
                ledger_root: ledger.as_deref(),
                out: &out,
            })?;
            eprintln!(
                "job worker-schedule-local {} receipt={} worker={} out={}",
                scheduled.decision,
                scheduled.receipt_ref,
                scheduled.worker.as_ref().map(|worker| worker.receipt_ref.as_str()).unwrap_or("-"),
                out.display()
            );
            if scheduled.decision == "pass" {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job worker-schedule-local denied: {}",
                    job_dag::parse_job_worker_schedule_receipt_value(&scheduled.receipt_value)?.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::RefSubmit {
            job_id,
            operation_id,
            executable,
            inputs,
            output_mode,
            input_schema_refs,
            output_schema_refs,
            effect_manifest_refs,
            handler_profile,
            authority_context_ref,
            policy_refs,
            provenance_refs,
            evidence_refs,
            out,
        } => {
            let executable = parse_job_content_arg(&executable, "executable")?;
            let inputs =
                inputs.iter().map(|input| parse_job_content_arg(input, "input")).collect::<Result<Vec<_>>>()?;
            let value = job_dag::job_ref_submission_value(job_dag::BlobRefJobSubmissionValueInput {
                job_id: &job_id,
                operation_id: &operation_id,
                executable,
                inputs,
                output_mode: &output_mode,
                input_schema_refs: &input_schema_refs,
                output_schema_refs: &output_schema_refs,
                effect_manifest_refs: &effect_manifest_refs,
                handler_profile: &handler_profile,
                authority_context_ref: &authority_context_ref,
                policy_refs: &policy_refs,
                provenance_refs: &provenance_refs,
                evidence_refs: &evidence_refs,
            })?;
            let submission = job_dag::parse_job_ref_submission_value(&value)?;
            emit_job_analysis(&value, out.as_ref())?;
            eprintln!(
                "job ref-submit ok job={} submission={} inputs={}",
                submission.job_id,
                submission.submission_ref,
                submission.inputs.len()
            );
            Ok(())
        }
        JobCommand::RefExecute {
            submission,
            chunks,
            ledger,
            receipt_out,
        } => {
            let submission_value = read_preserves_file(&submission)?;
            let executed = job_dag::execute_blob_ref_job(job_dag::BlobRefJobExecuteInput {
                chunk_root: &chunks,
                submission_value: &submission_value,
                ledger_root: ledger.as_deref(),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "job ref receipt", &executed.receipt_value)?;
            eprintln!(
                "job ref-execute {} job={} receipt={} output={}",
                executed.decision,
                executed.submission.job_id,
                executed.receipt_ref,
                executed.output_manifest_ref.as_deref().unwrap_or("none")
            );
            if executed.decision == "pass" {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "job ref-execute denied: {}",
                    executed.diagnostics.join("; ")
                )))
            }
        }
        JobCommand::Status { ledger, job } => {
            for entry in ledger::list_artifacts(&ledger)? {
                let value = match entry.artifact_kind.as_str() {
                    "job-dag-receipt" | "job-ref-receipt" | "job-worker-receipt" | "job-worker-schedule-receipt" => {
                        ledger::read_artifact(&ledger, &entry.artifact_ref)?
                    }
                    _ => continue,
                };
                if let Ok(schedule) = job_dag::parse_job_worker_schedule_receipt_value(&value) {
                    if job.as_ref().is_none_or(|job_ref| schedule.job_ref == *job_ref) {
                        println!(
                            "{} worker-schedule {} {} {}",
                            entry.artifact_ref,
                            schedule.decision,
                            schedule.job_ref,
                            schedule.result_ref.unwrap_or_else(|| "-".to_string())
                        );
                    }
                    continue;
                }
                if let Ok(worker) = job_dag::parse_job_worker_receipt_value(&value) {
                    if job.as_ref().is_none_or(|job_ref| worker.job_ref.as_ref() == Some(job_ref)) {
                        println!(
                            "{} worker-execute {} {} {}",
                            entry.artifact_ref,
                            worker.decision,
                            worker.job_ref.unwrap_or_else(|| "-".to_string()),
                            worker.result_ref
                        );
                    }
                    continue;
                }
                let receipt = job_dag::parse_job_receipt(&value)
                    .or_else(|_| job_dag::parse_blob_ref_job_receipt_value(&value))?;
                if job.as_ref().is_none_or(|job_ref| receipt.job_ref.as_ref() == Some(job_ref)) {
                    println!(
                        "{} {} {} {} {}",
                        entry.artifact_ref,
                        receipt.operation,
                        receipt.decision,
                        receipt.job_ref.unwrap_or_else(|| "-".to_string()),
                        receipt.stage_id.unwrap_or_else(|| "-".to_string())
                    );
                }
            }
            Ok(())
        }
        JobCommand::ReceiptShow { receipt_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            println!("{}", job_dag::receipt_summary(&value)?);
            println!("{}", to_text(&value)?);
            Ok(())
        }
    }
}

fn job_sync_cli_request(
    source_registry: &Path,
    job: &str,
    stages: &[String],
    target_peer: &str,
    extra_evidence_refs: &[String],
) -> Result<preserves::IOValue> {
    let dag = job_dag::read_job_dag_file_or_registry(source_registry, job)?;
    let mut evidence_refs = vec![cli_job_ref("sync-evidence", &dag.job_ref)?];
    evidence_refs.extend(extra_evidence_refs.iter().cloned());
    job_dag::job_sync_request_value(job_dag::SyncRequestValueInput {
        job_ref: &dag.job_ref,
        stage_ids: stages,
        target_peer,
        policy_refs: &[cli_job_ref("sync-policy", &dag.job_ref)?],
        capability_refs: &[cli_job_ref("sync-capability", &dag.job_ref)?],
        evidence_refs: &evidence_refs,
    })
}

struct JobAdmissionCliInput<'a> {
    target_registry: &'a Path,
    job: &'a str,
    sync_ref: Option<&'a str>,
    stages: &'a [String],
    target_peer: &'a str,
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    evidence_refs: Vec<String>,
    resource_refs: Vec<String>,
}

struct JobExecutionCliInput<'a> {
    target_registry: &'a Path,
    job: &'a str,
    admission_value: &'a preserves::IOValue,
    target_peer: &'a str,
    stages: &'a [String],
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    resource_refs: Vec<String>,
}

struct JobExecutionFromAdmissionCliInput<'a> {
    target_registry: &'a Path,
    job: &'a str,
    admission_ref: Option<&'a str>,
    target_peer: &'a str,
    stages: &'a [String],
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    resource_refs: Vec<String>,
}

struct JobWorkerRequestCliInput<'a> {
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
    sync_ref: Option<&'a str>,
    target_peer: &'a str,
    stages: &'a [String],
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    peer_bootstrap_refs: Vec<String>,
    node_identity_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

struct JobWorkerRunLocalInput<'a> {
    request_value: &'a preserves::IOValue,
    target_registry: &'a Path,
    storage_root: &'a Path,
    cache_root: &'a Path,
    chunk_root: &'a Path,
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
    transport_root: &'a Path,
    from_peer: &'a str,
    from_actor: &'a str,
    topic: &'a str,
    ledger_root: Option<&'a Path>,
    out: &'a Path,
}

struct JobWorkerScheduleLocalInput<'a> {
    request_value: &'a preserves::IOValue,
    target_registry: &'a Path,
    storage_root: &'a Path,
    cache_root: &'a Path,
    chunk_root: &'a Path,
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
    transport_root: &'a Path,
    queue_key: &'a str,
    lease_key: Option<&'a str>,
    scheduler_session: &'a str,
    worker_session: &'a str,
    lease_token: Option<u64>,
    from_peer: &'a str,
    from_actor: &'a str,
    topic: &'a str,
    coordination_authority_refs: Vec<String>,
    coordination_resource_refs: Vec<String>,
    coordination_policy_refs: Vec<String>,
    ledger_root: Option<&'a Path>,
    out: &'a Path,
}

struct JobWorkerScheduleLocalResult {
    decision: String,
    receipt_ref: String,
    receipt_value: preserves::IOValue,
    worker: Option<job_dag::JobWorkerExecution>,
}

fn job_worker_cli_request(input: JobWorkerRequestCliInput<'_>) -> Result<preserves::IOValue> {
    let admission = job_dag::parse_job_admission_receipt_value(input.admission_value)?;
    let execution_request = job_dag::parse_job_execution_request_value(input.execution_request_value)?;
    let admission_ref = canonical_hash(input.admission_value)?;
    let execution_request_ref = canonical_hash(input.execution_request_value)?;
    if execution_request.admission_ref != admission_ref {
        return Err(MoltenError::invalid_harness("job worker execution request does not bind admission receipt"));
    }
    if execution_request.job_ref != admission.job_ref {
        return Err(MoltenError::invalid_harness("job worker execution request job ref mismatches admission"));
    }
    let sync_ref = input.sync_ref.map(str::to_string).unwrap_or_else(|| admission.sync_ref.clone());
    let stage_ids = if input.stages.is_empty() {
        execution_request.stage_ids.clone()
    } else {
        input.stages.to_vec()
    };
    let authority_refs = if input.authority_refs.is_empty() {
        admission.authority_receipt_refs.clone()
    } else {
        input.authority_refs
    };
    let resource_refs = if input.resource_refs.is_empty() {
        execution_request.resource_refs.clone()
    } else {
        input.resource_refs
    };
    let mut evidence_refs = CliBoundedItems::new(JOB_WORKER_CLI_REF_LIMIT, "job worker evidence refs");
    for reference in input.evidence_refs {
        evidence_refs.push_unique(reference)?;
    }
    for reference in [sync_ref.clone(), admission_ref.clone(), execution_request_ref.clone()] {
        evidence_refs.push_unique(reference)?;
    }
    for reference in &input.peer_bootstrap_refs {
        evidence_refs.push_unique(reference.clone())?;
    }
    for reference in &input.node_identity_refs {
        evidence_refs.push_unique(reference.clone())?;
    }
    job_dag::job_worker_request_value(job_dag::JobWorkerRequestValueInput {
        job_ref: &admission.job_ref,
        target_peer: input.target_peer,
        stage_ids: &stage_ids,
        sync_ref: &sync_ref,
        admission_ref: &admission_ref,
        execution_request_ref: &execution_request_ref,
        authority_refs: &authority_refs,
        resource_refs: &resource_refs,
        peer_bootstrap_refs: &input.peer_bootstrap_refs,
        node_identity_refs: &input.node_identity_refs,
        evidence_refs: &evidence_refs.into_vec(),
    })
}

fn job_worker_run_local(input: JobWorkerRunLocalInput<'_>) -> Result<job_dag::JobWorkerExecution> {
    let request = job_dag::parse_job_worker_request_value(input.request_value)?;
    let envelope = job_dag::job_worker_envelope(job_dag::JobWorkerEnvelopeInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: &request.target_peer,
        topic: input.topic,
        request_value: input.request_value,
    })?;
    let published = remote_dataspace::publish_local_gossip(input.transport_root, &envelope, input.from_peer)?;
    let delivery = remote_dataspace::deliver_local_gossip(
        input.transport_root,
        input.topic,
        &envelope.envelope_ref,
        &request.target_peer,
    )?;
    let delivery_log = remote_dataspace::delivery_log(std::slice::from_ref(&delivery), true)?;
    let executed = job_dag::execute_worker_delivery(job_dag::JobWorkerExecuteInput {
        target_registry: input.target_registry,
        storage_root: input.storage_root,
        cache_root: input.cache_root,
        chunk_root: input.chunk_root,
        delivery: &delivery,
        delivery_log: Some(&delivery_log),
        admission_receipt_value: input.admission_value,
        execution_request_value: input.execution_request_value,
        ledger_root: input.ledger_root,
    })?;
    fs::create_dir_all(input.out).map_err(MoltenError::from)?;
    write_file(&input.out.join("request.preserves"), &to_text(input.request_value)?)?;
    write_file(&input.out.join("envelope.preserves"), &to_text(&envelope.value)?)?;
    write_file(&input.out.join("publish-receipt.preserves"), &to_text(&published.receipt_value)?)?;
    write_file(&input.out.join("delivery-receipt.preserves"), &to_text(&delivery.receipt_value)?)?;
    write_file(&input.out.join("delivery-log.preserves"), &to_text(&delivery_log.value)?)?;
    write_file(&input.out.join("assignment.preserves"), &to_text(&executed.assignment_value)?)?;
    write_indexed_values(input.out, "status", &executed.status_values)?;
    write_file(&input.out.join("result.preserves"), &to_text(&executed.result.value)?)?;
    write_file(&input.out.join("worker-receipt.preserves"), &to_text(&executed.receipt_value)?)?;
    if let Some(execution) = executed.execution.as_ref() {
        write_file(&input.out.join("execution-receipt.preserves"), &to_text(&execution.receipt_value)?)?;
        if let Some(run) = execution.run.as_ref() {
            write_file(&input.out.join("output.preserves"), &to_text(&run.output_value)?)?;
        }
    }
    Ok(executed)
}

struct ScheduleCoordinationRefs {
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    policy_refs: Vec<String>,
}

struct ScheduleCoordinationRequestInput<'a> {
    service: &'a str,
    operation: &'a str,
    key: &'a str,
    client_session: &'a str,
    operation_label: &'a str,
    request_ref: &'a str,
    payload: Option<preserves::IOValue>,
    refs: &'a ScheduleCoordinationRefs,
}

fn job_schedule_coordination_request(input: ScheduleCoordinationRequestInput<'_>) -> Result<preserves::IOValue> {
    coordination::coordination_request_value(&coordination::CoordinationRequestInput {
        service: input.service.to_string(),
        operation: input.operation.to_string(),
        key: input.key.to_string(),
        client_session: input.client_session.to_string(),
        operation_id_ref: cli_job_ref(input.operation_label, input.request_ref)?,
        payload: input.payload,
        authority_refs: input.refs.authority_refs.clone(),
        resource_refs: input.refs.resource_refs.clone(),
        policy_refs: input.refs.policy_refs.clone(),
    })
}

fn push_schedule_coordination_result(
    result: &coordination::CoordinationApplyResult,
    evidence_values: &mut CliBoundedItems<preserves::IOValue>,
    receipt_refs: &mut CliBoundedItems<String>,
    assertion_refs: &mut CliBoundedItems<String>,
) -> Result<()> {
    receipt_refs.push(result.receipt.receipt_ref.clone())?;
    for assertion in &result.assertions {
        assertion_refs.push(assertion.assertion_ref.clone())?;
    }
    for value in &result.evidence_values {
        evidence_values.push(value.clone())?;
    }
    Ok(())
}

fn job_worker_schedule_local(input: JobWorkerScheduleLocalInput<'_>) -> Result<JobWorkerScheduleLocalResult> {
    let request = job_dag::parse_job_worker_request_value(input.request_value)?;
    let request_ref = request.request_ref.clone();
    let lease_key = input
        .lease_key
        .map(str::to_string)
        .unwrap_or_else(|| format!("lock:job-worker:{}", request.request_ref));
    let coordination_refs = ScheduleCoordinationRefs {
        authority_refs: if input.coordination_authority_refs.is_empty() {
            request.authority_refs.clone()
        } else {
            input.coordination_authority_refs.clone()
        },
        resource_refs: if input.coordination_resource_refs.is_empty() {
            request.resource_refs.clone()
        } else {
            input.coordination_resource_refs.clone()
        },
        policy_refs: if input.coordination_policy_refs.is_empty() {
            vec![cli_job_ref("worker-schedule-policy", &request_ref)?]
        } else {
            input.coordination_policy_refs.clone()
        },
    };
    let manifest_value = coordination::coordination_fixture_manifest_value()?;
    let mut runtime = coordination::new_coordination_runtime(&manifest_value)?;
    let manifest_ref = runtime.manifest.manifest_ref.clone();
    let mut evidence_values =
        CliBoundedItems::new(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "job worker schedule evidence");
    let mut receipt_refs = CliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule receipts");
    let mut assertion_refs = CliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule assertions");
    evidence_values.push(manifest_value.clone())?;

    let enqueue_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
        service: coordination::SERVICE_QUEUE,
        operation: coordination::OP_ENQUEUE,
        key: input.queue_key,
        client_session: input.scheduler_session,
        operation_label: "worker-schedule-enqueue",
        request_ref: &request_ref,
        payload: Some(record("item", vec![string(&request_ref)])),
        refs: &coordination_refs,
    })?;
    let enqueue = coordination::apply_coordination_request(&mut runtime, &enqueue_request)?;
    push_schedule_coordination_result(&enqueue, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
    let enqueue_duplicate = coordination::apply_coordination_request(&mut runtime, &enqueue_request)?;
    push_schedule_coordination_result(
        &enqueue_duplicate,
        &mut evidence_values,
        &mut receipt_refs,
        &mut assertion_refs,
    )?;

    let mut diagnostics = Vec::new();
    let mut dequeue: Option<coordination::CoordinationApplyResult> = None;
    let mut lease: Option<coordination::CoordinationApplyResult> = None;
    let mut release: Option<coordination::CoordinationApplyResult> = None;
    let mut worker: Option<job_dag::JobWorkerExecution> = None;
    if enqueue.receipt.decision != "pass" {
        diagnostics.extend(enqueue.receipt.diagnostics.clone());
    } else if enqueue_duplicate.receipt.receipt_ref != enqueue.receipt.receipt_ref {
        diagnostics.push("coordination duplicate enqueue did not replay prior receipt".to_string());
    } else {
        let dequeue_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
            service: coordination::SERVICE_QUEUE,
            operation: coordination::OP_DEQUEUE,
            key: input.queue_key,
            client_session: input.worker_session,
            operation_label: "worker-schedule-dequeue",
            request_ref: &request_ref,
            payload: None,
            refs: &coordination_refs,
        })?;
        let result = coordination::apply_coordination_request(&mut runtime, &dequeue_request)?;
        push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
        if result.receipt.decision != "pass" {
            diagnostics.extend(result.receipt.diagnostics.clone());
        }
        dequeue = Some(result);
    }
    if diagnostics.is_empty() {
        let lease_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
            service: coordination::SERVICE_LOCK,
            operation: coordination::OP_ACQUIRE,
            key: &lease_key,
            client_session: input.worker_session,
            operation_label: "worker-schedule-lease",
            request_ref: &request_ref,
            payload: None,
            refs: &coordination_refs,
        })?;
        let result = coordination::apply_coordination_request(&mut runtime, &lease_request)?;
        push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
        if result.receipt.decision != "pass" {
            diagnostics.extend(result.receipt.diagnostics.clone());
        }
        lease = Some(result);
    }
    let token = lease.as_ref().and_then(|result| result.token.as_ref());
    if diagnostics.is_empty() {
        let Some(token) = token else {
            diagnostics.push("coordination lease did not emit fencing token".to_string());
            return job_worker_schedule_finalize(JobWorkerScheduleFinalizeInput {
                input,
                request: &request,
                manifest_ref: &manifest_ref,
                runtime: &runtime,
                evidence_values,
                receipt_refs,
                assertion_refs,
                enqueue: Some(&enqueue),
                enqueue_duplicate: Some(&enqueue_duplicate),
                dequeue: dequeue.as_ref(),
                lease: lease.as_ref(),
                release: None,
                worker: None,
                diagnostics,
                lease_key: &lease_key,
            });
        };
        let effective_token = input.lease_token.unwrap_or(token.token);
        if effective_token != token.token {
            let release_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
                service: coordination::SERVICE_LOCK,
                operation: coordination::OP_RELEASE,
                key: &lease_key,
                client_session: input.worker_session,
                operation_label: "worker-schedule-release",
                request_ref: &request_ref,
                payload: Some(record("token", vec![u64_value(effective_token)])),
                refs: &coordination_refs,
            })?;
            let result = coordination::apply_coordination_request(&mut runtime, &release_request)?;
            push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
            diagnostics.extend(result.receipt.diagnostics.clone());
            if diagnostics.is_empty() {
                diagnostics.push(format!("stale fencing token {effective_token}; current token is {}", token.token));
            }
            release = Some(result);
        } else {
            let worker_out = input.out.join("worker");
            let executed = job_worker_run_local(JobWorkerRunLocalInput {
                request_value: input.request_value,
                target_registry: input.target_registry,
                storage_root: input.storage_root,
                cache_root: input.cache_root,
                chunk_root: input.chunk_root,
                admission_value: input.admission_value,
                execution_request_value: input.execution_request_value,
                transport_root: input.transport_root,
                from_peer: input.from_peer,
                from_actor: input.from_actor,
                topic: input.topic,
                ledger_root: input.ledger_root,
                out: &worker_out,
            })?;
            if executed.result.decision != "pass" {
                diagnostics.extend(executed.result.diagnostics.clone());
            }
            worker = Some(executed);
            let release_request = job_schedule_coordination_request(ScheduleCoordinationRequestInput {
                service: coordination::SERVICE_LOCK,
                operation: coordination::OP_RELEASE,
                key: &lease_key,
                client_session: input.worker_session,
                operation_label: "worker-schedule-release",
                request_ref: &request_ref,
                payload: Some(record("token", vec![u64_value(effective_token)])),
                refs: &coordination_refs,
            })?;
            let result = coordination::apply_coordination_request(&mut runtime, &release_request)?;
            push_schedule_coordination_result(&result, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
            if result.receipt.decision != "pass" {
                diagnostics.extend(result.receipt.diagnostics.clone());
            }
            release = Some(result);
        }
    }
    job_worker_schedule_finalize(JobWorkerScheduleFinalizeInput {
        input,
        request: &request,
        manifest_ref: &manifest_ref,
        runtime: &runtime,
        evidence_values,
        receipt_refs,
        assertion_refs,
        enqueue: Some(&enqueue),
        enqueue_duplicate: Some(&enqueue_duplicate),
        dequeue: dequeue.as_ref(),
        lease: lease.as_ref(),
        release: release.as_ref(),
        worker: worker.as_ref(),
        diagnostics,
        lease_key: &lease_key,
    })
}

struct JobWorkerScheduleFinalizeInput<'a> {
    input: JobWorkerScheduleLocalInput<'a>,
    request: &'a job_dag::JobWorkerRequest,
    manifest_ref: &'a str,
    runtime: &'a coordination::CoordinationRuntime,
    evidence_values: CliBoundedItems<preserves::IOValue>,
    receipt_refs: CliBoundedItems<String>,
    assertion_refs: CliBoundedItems<String>,
    enqueue: Option<&'a coordination::CoordinationApplyResult>,
    enqueue_duplicate: Option<&'a coordination::CoordinationApplyResult>,
    dequeue: Option<&'a coordination::CoordinationApplyResult>,
    lease: Option<&'a coordination::CoordinationApplyResult>,
    release: Option<&'a coordination::CoordinationApplyResult>,
    worker: Option<&'a job_dag::JobWorkerExecution>,
    diagnostics: Vec<String>,
    lease_key: &'a str,
}

fn pass_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn job_worker_schedule_finalize(input: JobWorkerScheduleFinalizeInput<'_>) -> Result<JobWorkerScheduleLocalResult> {
    let mut evidence_values = input.evidence_values;
    let receipt_refs = input.receipt_refs.into_vec();
    let assertion_refs = input.assertion_refs.into_vec();
    let final_state_value = coordination::coordination_state_snapshot_value(&input.runtime.state)?;
    let final_state_ref = canonical_hash(&final_state_value)?;
    evidence_values.push(final_state_value)?;
    let evidence_values = evidence_values.into_vec();
    let evidence_refs = evidence_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let report_value = coordination::coordination_apply_report_value(coordination::ApplyReportValueInput {
        decision,
        manifest_ref: input.manifest_ref,
        final_state_ref: &final_state_ref,
        receipt_refs: &receipt_refs,
        assertion_refs: &assertion_refs,
        evidence_refs: &evidence_refs,
    })?;
    let report_ref = canonical_hash(&report_value)?;
    let worker_receipt_ref = input.worker.map(|worker| worker.receipt_ref.as_str());
    let result_ref = input.worker.map(|worker| worker.result.result_ref.as_str());
    let token_ref = input.lease.and_then(|lease| lease.token.as_ref()).map(|token| token.token_ref.as_str());
    let mut refs = evidence_refs.clone();
    if let Some(worker) = input.worker {
        refs.push(worker.receipt_ref.clone());
        refs.push(worker.result.result_ref.clone());
    }
    let receipt_value = job_dag::job_worker_schedule_receipt_value(job_dag::JobWorkerScheduleReceiptValueInput {
        operation: "worker-schedule-local",
        decision,
        job_ref: &input.request.job_ref,
        request_ref: &input.request.request_ref,
        queue_key: input.input.queue_key,
        lease_key: input.lease_key,
        worker_session: input.input.worker_session,
        coordination_report_ref: &report_ref,
        enqueue_receipt_ref: input.enqueue.map(|result| result.receipt.receipt_ref.as_str()),
        enqueue_duplicate_receipt_ref: input.enqueue_duplicate.map(|result| result.receipt.receipt_ref.as_str()),
        dequeue_receipt_ref: input.dequeue.map(|result| result.receipt.receipt_ref.as_str()),
        lease_receipt_ref: input.lease.map(|result| result.receipt.receipt_ref.as_str()),
        release_receipt_ref: input.release.map(|result| result.receipt.receipt_ref.as_str()),
        token_ref,
        worker_receipt_ref,
        result_ref,
        diagnostics: &input.diagnostics,
        refs: &refs,
        checks: &[
            (
                "duplicate-operation-replay",
                pass_fail(input.enqueue_duplicate.is_some_and(|duplicate| {
                    input.enqueue.is_some_and(|enqueue| duplicate.receipt.receipt_ref == enqueue.receipt.receipt_ref)
                })),
            ),
            ("lease-checked-before-worker", pass_fail(input.worker.is_some() || !input.diagnostics.is_empty())),
            (
                "worker-result-bound",
                pass_fail(input.worker.is_some_and(|worker| worker.result.decision == "pass")),
            ),
        ],
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    fs::create_dir_all(input.input.out).map_err(MoltenError::from)?;
    write_file(&input.input.out.join("schedule-receipt.preserves"), &to_text(&receipt_value)?)?;
    let coordination_out = input.input.out.join("coordination");
    fs::create_dir_all(&coordination_out).map_err(MoltenError::from)?;
    write_file(&coordination_out.join("manifest.preserves"), &to_text(&evidence_values[0])?)?;
    write_file(&coordination_out.join("report.preserves"), &to_text(&report_value)?)?;
    write_indexed_values(&coordination_out, "evidence", &evidence_values)?;
    if let Some(result) = input.enqueue {
        write_file(&coordination_out.join("enqueue-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(result) = input.enqueue_duplicate {
        write_file(&coordination_out.join("enqueue-duplicate-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(result) = input.dequeue {
        write_file(&coordination_out.join("dequeue-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(result) = input.lease {
        write_file(&coordination_out.join("lease-receipt.preserves"), &to_text(&result.receipt.value)?)?;
        if let Some(token) = &result.token {
            write_file(&coordination_out.join("fencing-token.preserves"), &to_text(&token.value)?)?;
        }
    }
    if let Some(result) = input.release {
        write_file(&coordination_out.join("release-receipt.preserves"), &to_text(&result.receipt.value)?)?;
    }
    if let Some(ledger_root) = input.input.ledger_root {
        ledger::import_artifact(ledger_root, &report_value)?;
        ledger::import_artifact(ledger_root, &receipt_value)?;
    }
    Ok(JobWorkerScheduleLocalResult {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        worker: input.worker.cloned(),
    })
}

fn job_admission_cli_request(input: JobAdmissionCliInput<'_>) -> Result<preserves::IOValue> {
    let mut policy_refs = input.policy_refs;
    let mut capability_refs = input.capability_refs;
    let mut evidence_refs = input.evidence_refs;
    let mut resource_refs = input.resource_refs;
    let dag = job_dag::read_job_dag_file_or_registry(input.target_registry, input.job)?;
    let sync_ref = input.sync_ref.map(str::to_string).unwrap_or(cli_job_ref("sync-evidence", &dag.job_ref)?);
    if policy_refs.is_empty() {
        policy_refs.push(cli_job_ref("admission-policy", &dag.job_ref)?);
    }
    if capability_refs.is_empty() {
        capability_refs.push(cli_job_ref("admission-capability", &dag.job_ref)?);
    }
    if !evidence_refs.iter().any(|reference| reference == &sync_ref) {
        evidence_refs.push(sync_ref.clone());
    }
    if !evidence_refs.iter().any(|reference| reference != &sync_ref) {
        evidence_refs.push(cli_job_ref("strict-octet-gate", &dag.job_ref)?);
    }
    if resource_refs.is_empty() {
        let selected = if input.stages.is_empty() {
            dag.nodes.len()
        } else {
            input.stages.len()
        };
        for index in 0..selected.max(1) {
            resource_refs.push(cli_job_ref("admission-resource", &format!("{}:{index}", dag.job_ref))?);
        }
    }
    job_dag::job_admission_request_value(job_dag::AdmissionRequestValueInput {
        job_ref: &dag.job_ref,
        sync_ref: &sync_ref,
        stage_ids: input.stages,
        target_peer: input.target_peer,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        evidence_refs: &evidence_refs,
        resource_refs: &resource_refs,
    })
}

fn job_execution_cli_request(input: JobExecutionCliInput<'_>) -> Result<preserves::IOValue> {
    let admission = job_dag::parse_job_admission_receipt_value(input.admission_value)?;
    let selected_stages = if input.stages.is_empty() {
        admission.stage_order.clone()
    } else {
        input.stages.to_vec()
    };
    let mut capability_refs = input.capability_refs;
    if capability_refs.is_empty() {
        capability_refs.extend(admission.authority_receipt_refs.iter().cloned());
    }
    job_execution_cli_request_from_admission_ref(JobExecutionFromAdmissionCliInput {
        target_registry: input.target_registry,
        job: input.job,
        admission_ref: Some(&admission.receipt_ref),
        target_peer: input.target_peer,
        stages: &selected_stages,
        policy_refs: input.policy_refs,
        capability_refs,
        resource_refs: input.resource_refs,
    })
}

fn job_execution_cli_request_from_admission_ref(
    input: JobExecutionFromAdmissionCliInput<'_>,
) -> Result<preserves::IOValue> {
    let dag = job_dag::read_job_dag_file_or_registry(input.target_registry, input.job)?;
    let admission_ref = input
        .admission_ref
        .map(str::to_string)
        .unwrap_or(cli_job_ref("missing-admission-receipt", &dag.job_ref)?);
    let storage_profile = cli_job_ref("target-storage-profile", &dag.job_ref)?;
    let cache_profile = cli_job_ref("target-cache-profile", &dag.job_ref)?;
    let chunk_profile = cli_job_ref("target-chunk-profile", &dag.job_ref)?;
    job_dag::job_execution_request_value(job_dag::ExecutionRequestValueInput {
        job_ref: &dag.job_ref,
        admission_ref: &admission_ref,
        stage_ids: input.stages,
        target_peer: input.target_peer,
        storage_profile_ref: &storage_profile,
        cache_profile_ref: &cache_profile,
        chunk_profile_ref: &chunk_profile,
        policy_refs: &input.policy_refs,
        capability_refs: &input.capability_refs,
        resource_refs: &input.resource_refs,
    })
}

fn parse_job_content_arg(value: &str, label: &str) -> Result<job_dag::JobContentRef> {
    let parts = value.split('@').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(MoltenError::invalid_harness(format!(
            "job {label} must be formatted as <content-ref>@<size>@<format>[@<schema-ref>]"
        )));
    }
    let size = parts[1]
        .parse::<u64>()
        .map_err(|error| MoltenError::invalid_harness(format!("job {label} size is invalid: {error}")))?;
    let schema_ref = if parts.len() == 4 {
        Some(parts[3].to_string())
    } else {
        None
    };
    Ok(job_dag::JobContentRef {
        content_ref: parts[0].to_string(),
        size,
        format: parts[2].to_string(),
        schema_ref,
    })
}

fn cli_job_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("job-cli-ref", vec![string(kind), string(label)]))
}

fn emit_job_analysis(value: &preserves::IOValue, out: Option<&PathBuf>) -> Result<()> {
    let text = to_text(value)?;
    if let Some(path) = out {
        write_file(path, &text)?;
    } else {
        println!("{text}");
    }
    Ok(())
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn read_preserves_files(paths: &[PathBuf]) -> Result<Vec<preserves::IOValue>> {
    let mut values = CliBoundedItems::new(JOB_CLI_EVIDENCE_LIMIT, "Preserves input files");
    for path in paths {
        values.push(read_preserves_file(path)?)?;
    }
    Ok(values.into_vec())
}

fn values_canonical_refs(values: &[preserves::IOValue]) -> Result<Vec<String>> {
    let mut refs = CliBoundedItems::new(JOB_CLI_EVIDENCE_LIMIT, "Preserves input refs");
    for value in values {
        refs.push(canonical_hash(value)?)?;
    }
    Ok(refs.into_vec())
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
