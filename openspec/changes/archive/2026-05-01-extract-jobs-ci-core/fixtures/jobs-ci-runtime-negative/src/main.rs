//! Negative fixture: this package depends only on reusable jobs/CI defaults.
//! Runtime shells and concrete adapters must not be available from that surface.

use aspen_ci::PipelineExecutor;
use aspen_ci_executor_nix::NixExecutor;
use aspen_ci_executor_shell::ShellExecutor;
use aspen_ci_executor_vm::VmExecutor;
use aspen_ci_handler::CiHandler;
use aspen_job_handler::JobHandler;
use aspen_jobs::JobManager;
use aspen_jobs_worker_shell::ShellWorker;

fn main() {
    let _ = core::any::type_name::<PipelineExecutor>();
    let _ = core::any::type_name::<NixExecutor>();
    let _ = core::any::type_name::<ShellExecutor>();
    let _ = core::any::type_name::<VmExecutor>();
    let _ = core::any::type_name::<CiHandler>();
    let _ = core::any::type_name::<JobHandler>();
    let _ = core::any::type_name::<JobManager>();
    let _ = core::any::type_name::<ShellWorker>();
}
