//! Disposable live rail commands; none grants production or release admission.

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Adapter {
        #[arg(long)] request: std::path::PathBuf,
        #[arg(long)] response: std::path::PathBuf,
        #[arg(long)] authority_root: std::path::PathBuf,
        #[arg(long)] client_key: std::path::PathBuf,
    },
    Generate {
        #[arg(long)] profile: std::path::PathBuf,
        #[arg(long)] plan: std::path::PathBuf,
        #[arg(long)] metadata: Option<std::path::PathBuf>,
    },
    Check {
        #[arg(long)] checker: std::path::PathBuf,
        #[arg(long)] history: std::path::PathBuf,
        #[arg(long)] work_root: std::path::PathBuf,
        #[arg(long)] report: std::path::PathBuf,
    },
    Import {
        #[arg(long)] expected: std::path::PathBuf,
        #[arg(long)] evidence: std::path::PathBuf,
        #[arg(long)] checker: std::path::PathBuf,
        #[arg(long)] reference_checker: Option<std::path::PathBuf>,
        #[arg(long)] work_root: std::path::PathBuf,
        #[arg(long)] ledger: std::path::PathBuf,
        #[arg(long)] receipt: std::path::PathBuf,
    },
    Compare {
        #[arg(long)] profiles: std::path::PathBuf,
        #[arg(long)] report: std::path::PathBuf,
    },
    Provision {
        #[arg(long)] state_root: std::path::PathBuf,
        #[arg(long)] operator_key: std::path::PathBuf,
        #[arg(long)] policy: std::path::PathBuf,
        #[arg(long)] grant: std::path::PathBuf,
        #[arg(long)] probe_key: std::path::PathBuf,
    },
    VerifyReadiness {
        #[arg(long)] state_root: std::path::PathBuf,
        #[arg(long)] node_id: String,
        #[arg(long)] source_binding_ref: String,
        #[arg(long)] startup_receipt: std::path::PathBuf,
        #[arg(long)] health_receipt: std::path::PathBuf,
        #[arg(long)] endpoint_proof: std::path::PathBuf,
    },
    Service { #[command(subcommand)] command: ServiceCommand },
    MechanismProfile {
        #[arg(long)] input: std::path::PathBuf,
        #[arg(long)] out: std::path::PathBuf,
    },
    ExperimentalConfig {
        #[arg(long)] input: std::path::PathBuf,
        #[arg(long)] out: std::path::PathBuf,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum ServiceCommand {
    Init { #[arg(long)] state_root: std::path::PathBuf },
    Serve { #[arg(long)] state_root: std::path::PathBuf },
    Status { #[arg(long)] state_root: std::path::PathBuf },
    Stop { #[arg(long)] state_root: std::path::PathBuf },
}

pub fn run(command: Command) -> molten::error::Result<()> {
    match command {
        Command::Adapter { request, response, authority_root, client_key } =>
            authenticated::run(request, response, authority_root, client_key),
        Command::Generate { profile, plan, metadata } => generated(profile, plan, metadata),
        Command::Check { checker, history, work_root, report } => {
            let checked = checked(&checker, &read(&history)?, &work_root)?;
            write(&report, checked.report())
        },
        Command::Import { expected, evidence, checker, reference_checker, work_root, ledger, receipt } => {
            let paths = importing::Paths { expected, evidence, checker, reference_checker, work_root, ledger, receipt };
            importing::run(paths)
        },
        Command::Compare { profiles, report } => {
            let profiles: Vec<molten::live_reliability::comparison::ProfileEvidence> = read(&profiles)?;
            let comparison = molten::live_reliability::comparison::compare(&profiles)
                .map_err(|issue| molten::error::Failure::invalid_harness(format!("comparison-denied:{issue:?}")))?;
            write(&report, &comparison)
        },
        Command::Provision { state_root, operator_key, policy, grant, probe_key } =>
            provisioning::run(state_root, operator_key, policy, grant, probe_key),
        Command::VerifyReadiness { state_root, node_id, source_binding_ref, startup_receipt, health_receipt, endpoint_proof } =>
            verifying::run(verifying::Paths { state_root, node_id, source_binding_ref, startup_receipt, health_receipt, endpoint_proof }),
        Command::Service { command } => match command {
            ServiceCommand::Init { state_root } => service("init", state_root),
            ServiceCommand::Serve { state_root } => service("serve", state_root),
            ServiceCommand::Status { state_root } => service("status", state_root),
            ServiceCommand::Stop { state_root } => service("stop", state_root),
        },
        Command::MechanismProfile { input, out } => {
            let input = read(&input)?;
            write(&out, &molten::live_reliability::service::profiles::experimental(input)?)
        },
        Command::ExperimentalConfig { input, out } => {
            let input = read(&input)?;
            write(&out, &molten::live_reliability::service::recipe::materialize(input)?)
        },
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> molten::error::Result<T> {
    let file = std::fs::File::open(path).map_err(molten::error::Failure::from)?;
    let mut bounded = std::io::Read::take(file, 1_048_577);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut bounded, &mut bytes).map_err(molten::error::Failure::from)?;
    if bytes.len() > 1_048_576 { return Err(molten::error::Failure::invalid_harness("input-frame-bound")); }
    serde_json::from_slice(&bytes).map_err(|error| molten::error::Failure::invalid_harness(error.to_string()))
}

fn write<T: serde::Serialize>(path: &std::path::Path, value: &T) -> molten::error::Result<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| molten::error::Failure::invalid_harness(error.to_string()))?;
    std::fs::write(path, bytes).map_err(molten::error::Failure::from)
}

mod importing;
mod authenticated;
mod provisioning;
mod verifying;

fn checked(config: &std::path::Path, history: &serde_json::Value, work_root: &std::path::Path)
    -> molten::error::Result<molten::live_reliability::checker::CheckedReport> {
    let checker = read(config)?;
    let root = cap_std::fs::Dir::open_ambient_dir(work_root, cap_std::ambient_authority())
        .map_err(molten::error::Failure::from)?;
    molten::live_reliability::checker::run(&checker,
        &molten::live_reliability::checker::Files { root: &root, history })
        .map_err(molten::error::Failure::invalid_harness)
}

fn generated(profile: std::path::PathBuf, plan: std::path::PathBuf, metadata: Option<std::path::PathBuf>) -> molten::error::Result<()> {
    let profile = read::<molten::live_reliability::profile::Register>(&profile)?;
    let admitted = molten::live_reliability::profile::admit(&profile)
        .map_err(|error| molten::error::Failure::invalid_harness(format!("profile-denied:{error:?}")))?;
    let generated = molten::live_reliability::generator::generate(&admitted)
        .map_err(|error| molten::error::Failure::invalid_harness(format!("plan-denied:{error:?}")))?;
    write(&plan, &generated)?;
    if let Some(metadata) = metadata {
        let plan_ref = molten::live_reliability::profile::canonical_ref(&generated)
            .map_err(|error| molten::error::Failure::invalid_harness(format!("plan-encoding:{error:?}")))?;
        write(&metadata, &serde_json::json!({"profile_ref":admitted.profile_ref(),"plan_ref":plan_ref}))?;
    }
    Ok(())
}

fn service(operation: &str, state_root: std::path::PathBuf) -> molten::error::Result<()> {
    let root = molten::node_state::DirectoryRoot::open_existing(&state_root)?;
    let result = molten::live_reliability::service::command(operation, &root)?;
    println!("{}", serde_json::to_string(&result)
        .map_err(|error| molten::error::Failure::invalid_harness(error.to_string()))?);
    Ok(())
}
