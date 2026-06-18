#[derive(Debug, clap::Args)]
pub(crate) struct Init {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long, default_value = "node:local")]
    pub(crate) node_id: String,
    #[arg(long)]
    pub(crate) config_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) identity_receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Run {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long)]
    pub(crate) startup_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RunLoop {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LOOP_REQUESTS)]
    pub(crate) max_requests: u64,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) heartbeat_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Serve {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long, default_value = molten::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC)]
    pub(crate) topic: String,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_SERVICE_TICKS)]
    pub(crate) max_ticks: u64,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LOOP_REQUESTS)]
    pub(crate) max_requests_per_tick: u64,
    #[arg(long)]
    pub(crate) live_iroh: bool,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LIVE_LISTENER_EVENTS)]
    pub(crate) live_max_events: u64,
    #[arg(long, default_value_t = molten::node_daemon::DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS)]
    pub(crate) live_event_timeout_ms: u64,
    #[arg(long)]
    pub(crate) service_receipt_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) live_ticket_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) supervisor_policy: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Status {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long)]
    pub(crate) health_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Stop {
    #[arg(long)]
    pub(crate) state_root: std::path::PathBuf,
    #[arg(long)]
    pub(crate) shutdown_out: Option<std::path::PathBuf>,
    #[arg(long)]
    pub(crate) receipt_out: Option<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Show {
    pub(crate) artifact: std::path::PathBuf,
}
