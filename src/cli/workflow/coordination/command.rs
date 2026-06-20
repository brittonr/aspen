type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Manifest {
        #[arg(long, default_value = "coordination:local")]
        service_id: String,
        #[arg(long = "service")]
        services: Vec<String>,
        #[arg(long)]
        control_group_ref: Option<String>,
        #[arg(long, default_value_t = molten::coordination::DEFAULT_COORDINATION_QUEUE_CAPACITY)]
        queue_capacity: u64,
        #[arg(long, default_value_t = molten::coordination::DEFAULT_COORDINATION_SEMAPHORE_CAPACITY)]
        semaphore_capacity: u64,
        #[arg(long, default_value_t = molten::coordination::DEFAULT_COORDINATION_RATE_LIMIT)]
        rate_limit: u64,
        #[arg(long, default_value_t = molten::coordination::DEFAULT_COORDINATION_BARRIER_PARTIES)]
        barrier_parties: u64,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Request {
        #[arg(long)]
        service: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        client_session: String,
        #[arg(long)]
        operation_id_ref: String,
        #[arg(long)]
        payload: Option<FilePath>,
        #[arg(long = "authority-ref")]
        authority_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Apply {
        #[arg(long)]
        manifest: FilePath,
        #[arg(long = "request")]
        requests: Vec<FilePath>,
        #[arg(long)]
        out: FilePath,
    },
    RunFixture {
        #[arg(long)]
        out: FilePath,
    },
    Show {
        artifact: FilePath,
    },
}
