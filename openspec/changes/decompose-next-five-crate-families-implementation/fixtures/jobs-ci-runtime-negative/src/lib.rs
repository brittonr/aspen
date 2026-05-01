// This fixture intentionally depends only on reusable jobs/CI defaults.
// Imports from runtime handlers, root jobs, shell/VM/Nix executors, and concrete
// worker adapters must fail unless those runtime shells are explicitly added.
use aspen_ci::PipelineRunner;
use aspen_ci_executor_nix::NixExecutor;
use aspen_ci_executor_shell::ShellExecutor;
use aspen_ci_executor_vm::VmExecutor;
use aspen_ci_handler::CiHandler;
use aspen_job_handler::JobHandler;
use aspen_jobs::JobManager;
use aspen_jobs_worker_shell::ShellWorker;

pub fn runtime_shells_should_not_be_available() {
    let _ = core::any::type_name::<PipelineRunner>();
    let _ = core::any::type_name::<NixExecutor>();
    let _ = core::any::type_name::<ShellExecutor>();
    let _ = core::any::type_name::<VmExecutor>();
    let _ = core::any::type_name::<CiHandler>();
    let _ = core::any::type_name::<JobHandler>();
    let _ = core::any::type_name::<JobManager>();
    let _ = core::any::type_name::<ShellWorker>();
}
