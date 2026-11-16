//! Aspen's direct-stream same-host qualification uses separate processes and real protected Unix
//! sockets. It does not execute or qualify two-host transport or the external Kiln/Cloud Hypervisor
//! cohort. Set MOLTEN_TEST_DIRECT_LAN_IP to an explicit local unicast IP to select a non-loopback
//! bind.

#![cfg(unix)]

#[path = "src/test/support.rs"]
mod test_support;

use std::os::unix::fs::PermissionsExt;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

const CHILD_ROLE: &str = "direct_stream_child_role";
const CONFIG_ENV: &str = "MOLTEN_DIRECT_STREAM_CHILD_CONFIG";
const POLL: std::time::Duration = std::time::Duration::from_millis(10);
const CHILD_BUDGET: std::time::Duration = std::time::Duration::from_secs(20);
const MAX_ARTIFACT_BYTES: u64 = 65_536;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

include!("fixtures/direct-lan-streams/process.rs");
include!("fixtures/direct-lan-streams/evidence.rs");

#[path = "fixtures/direct-lan-streams/config.rs"]
mod config;
#[path = "fixtures/direct-lan-streams/raw.rs"]
mod raw;
#[path = "fixtures/direct-lan-streams/role.rs"]
mod role;
#[path = "fixtures/direct-lan-streams/service.rs"]
mod service;

fn atomic_value(path: &std::path::Path, value: &preserves::IOValue) -> TestResult {
    let temporary = path.with_extension("pending");
    std::fs::write(&temporary, molten::preserves_rail::canonical_bytes(value)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn read_endpoint(path: &std::path::Path) -> TestResult<molten::fabric_transport::Endpoint> {
    let bytes = std::fs::read(path)?;
    let value = molten::preserves_rail::parse_canonical_bytes(&bytes)?;
    assert_eq!(molten::preserves_rail::canonical_bytes(&value)?, bytes);
    Ok(molten::fabric_transport::parse_endpoint(&value)?)
}

fn write_config(config: &config::RoleConfig) -> TestResult<std::path::PathBuf> {
    let path = config.root.join(format!("{}-{}.json", config.role, config.generation));
    std::fs::write(&path, serde_json::to_vec(config)?)?;
    Ok(path)
}

fn assert_native_output(path: &std::path::Path) -> TestResult {
    let bytes = std::fs::read(path)?;
    assert!(!bytes.is_empty());
    assert!(bytes.len() as u64 <= MAX_ARTIFACT_BYTES);
    let value = molten::preserves_rail::parse_canonical_bytes(&bytes)?;
    assert_eq!(molten::preserves_rail::canonical_bytes(&value)?, bytes);
    assert!(value.collect_simple_record("fabric-direct-stream-evidence-v1", Some(31)).is_some());
    let text = molten::preserves_rail::to_text(&value)?;
    assert!(!text.contains(std::str::from_utf8(config::REQUEST)?));
    assert!(!text.contains(std::str::from_utf8(config::RESPONSE)?));
    assert!(!text.contains("service.sock"));
    assert!(text.contains("not-two-host-qualification"));
    assert!(text.contains("not-kiln-or-vm-migration-qualification"));
    Ok(())
}

fn run_case(case: &str, bind_ip: std::net::IpAddr, evidence_dir: Option<&std::path::Path>) -> TestResult {
    let workspace = test_support::process_workspace(&format!("direct-stream-{case}"))?;
    let mut listener_config = config::RoleConfig {
        role: "listener".to_owned(),
        case: case.to_owned(),
        root: workspace.to_path_buf(),
        generation: 1,
        client_generation: 1,
        bind_ip,
        bind_port: 0,
    };
    if case == "stale" {
        listener_config.generation = 2;
    }
    let listener_log = workspace.join("listener.log");
    let mut listener = OwnedChild::spawn(&write_config(&listener_config)?, listener_log, &workspace)?;
    let handoff_path = workspace.join("handoff.preserves");
    listener.await_file(&handoff_path)?;
    if matches!(case, "cancel" | "drain") {
        listener.finish()?;
        assert_native_output(&workspace.join("listener-evidence.preserves"))?;
        retain_evidence(&workspace, case, evidence_dir)?;
        return Ok(());
    }
    assert!(listener.child.try_wait()?.is_none(), "listener exited before independent peer launch");
    if case == "restart" {
        restart_listener(&workspace, &handoff_path, &mut listener_config, &mut listener)?;
    }
    let is_raw = matches!(
        case,
        "stale" | "alpn" | "over-bound" | "slow" | "disconnect" | "ack-loss" | "restart" | "concurrent"
    );
    let mut client_config = listener_config.clone();
    client_config.role = if is_raw { "raw" } else { "client" }.to_owned();
    client_config.client_generation = if matches!(case, "stale" | "restart") {
        1
    } else {
        listener_config.generation
    };
    let mut client = OwnedChild::spawn(&write_config(&client_config)?, workspace.join("client.log"), &workspace)?;
    if case == "queue-saturation" {
        listener.await_file(&workspace.join("receiver-paused"))?;
        client.await_file(&workspace.join("producer-blocked"))?;
        assert!(
            !workspace.join("producer-finished").exists(),
            "producer escaped blocked receiver before control probe"
        );
        let mut probe_config = listener_config.clone();
        probe_config.role = "raw".to_owned();
        probe_config.case = "queue-probe".to_owned();
        let mut probe = OwnedChild::spawn(&write_config(&probe_config)?, workspace.join("probe.log"), &workspace)?;
        probe.finish()?;
        assert!(
            !workspace.join("producer-finished").exists(),
            "control rejection did not precede bulk producer completion"
        );
        atomic_value(&workspace.join("release-receiver"), &molten::preserves_rail::string("release"))?;
    }
    client.finish()?;
    listener.finish()?;
    assert_native_output(&workspace.join("listener-evidence.preserves"))?;
    if !is_raw {
        assert_native_output(&workspace.join("client-evidence.preserves"))?;
    }
    retain_evidence(&workspace, case, evidence_dir)?;
    Ok(())
}

// This entrypoint never derives authority or default configuration from the ambient host.
#[test]
#[ignore = "role entrypoint: launched only by the parent with explicit provisioned configuration"]
fn direct_stream_child_role() -> TestResult {
    let path = std::env::var_os(CONFIG_ENV).ok_or("missing explicit direct stream role configuration")?;
    let metadata = std::fs::metadata(&path)?;
    if !metadata.is_file() || metadata.len() > MAX_ARTIFACT_BYTES {
        return Err("invalid explicit role configuration type or size".into());
    }
    let config = serde_json::from_slice::<config::RoleConfig>(&std::fs::read(path)?)?;
    if !matches!(config.role.as_str(), "listener" | "client" | "raw") {
        return Err("unknown direct stream child role".into());
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(role::run(config))
}

macro_rules! qualification_case {
    ($name:ident, $case:literal) => {
        // r[verify molten.fabric_transport.direct_lan]
        // r[verify molten.fabric_transport.direct_stream_bounds]
        #[test]
        fn $name() -> TestResult {
            let bind_ip = match std::env::var("MOLTEN_TEST_DIRECT_LAN_IP") {
                Ok(value) => value.parse::<std::net::IpAddr>()?,
                Err(std::env::VarError::NotPresent) => std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                Err(error) => return Err(error.into()),
            };
            let evidence_dir = std::env::var_os("MOLTEN_DIRECT_STREAM_EVIDENCE_DIR").map(std::path::PathBuf::from);
            run_case($case, bind_ip, evidence_dir.as_deref())
        }
    };
}

qualification_case!(direct_stream_separate_process_protected_consumer_roundtrip, "positive");
qualification_case!(direct_stream_wrong_authenticated_peer_never_opens_protected_socket, "wrong-peer");
qualification_case!(direct_stream_stale_operation_generation_never_opens_protected_socket, "stale");
qualification_case!(direct_stream_revocation_fences_already_open_operation, "revoked");
qualification_case!(direct_stream_unsupported_alpn_never_opens_protected_socket, "alpn");
qualification_case!(direct_stream_resource_scope_denial_never_opens_protected_socket, "scope");
qualification_case!(direct_stream_oversized_prefix_denies_without_payload, "over-bound");
qualification_case!(direct_stream_partial_header_obeys_idle_deadline, "slow");
qualification_case!(direct_stream_authenticated_disconnect_is_terminal, "disconnect");
qualification_case!(direct_stream_lost_ack_preserves_consumed_byte_uncertainty, "ack-loss");
qualification_case!(direct_stream_restart_does_not_replay_stale_operation, "restart");
qualification_case!(direct_stream_cancel_before_open_is_terminal, "cancel");
qualification_case!(direct_stream_drained_listener_refuses_new_operation, "drain");
qualification_case!(direct_stream_concurrent_transfer_refuses_second_connection, "concurrent");
qualification_case!(direct_stream_saturated_receiver_preserves_control_admission_and_bulk_delivery, "queue-saturation");

// r[verify molten.fabric_transport.direct_lan]
#[tokio::test]
async fn direct_stream_crossed_listener_context_denies_before_binding() -> TestResult {
    let config = config::RoleConfig {
        role: "listener".to_owned(),
        case: "crossed-context".to_owned(),
        root: std::path::PathBuf::new(),
        generation: 1,
        client_generation: 1,
        bind_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        bind_port: 0,
    };
    let mut peer = config::peer(false, config.generation)?;
    peer.peer_context_ref = reference("crossed-listener-context")?;
    let result = molten::fabric_transport::direct::Acceptor::bind(molten::fabric_transport::direct::Config {
        listener: config::listener_input(&config)?,
        peer,
        bounds: config::bounds(),
        receive_budget: molten::fabric_transport::ReceiveBudget {
            burst: 1,
            refill_ticks: 100,
            max_sources: 1,
        },
        clock: std::time::Instant::now,
    })
    .await;
    let error = match result {
        Err(error) => error,
        Ok(listener) => {
            listener.drain_and_close(molten::fabric_transport::ListenerDrainReason::OperatorRequest).await?;
            panic!("crossed listener context published a handoff");
        }
    };
    assert_eq!(error, molten::fabric::FabricPortError::malformed("direct-context-mismatch"));
    Ok(())
}
