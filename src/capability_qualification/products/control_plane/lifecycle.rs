//! The producer side of the replica processes: spawn the fresh generation,
//! signal its phases, crash it, restart it from durable state, and read every
//! process's published receipts.

/// The fresh phase as the three replica processes reported it at the checkpoint.
pub(super) struct Fresh {
    pub(super) peers: Vec<super::super::peer::Process>,
    pub(super) process_ids: std::collections::BTreeSet<u64>,
    pub(super) replicas: Vec<super::receipt::Replica>,
}

/// The recovered phase: per-replica startup recovery and terminal receipts.
pub(super) struct Recovered {
    pub(super) process_ids: std::collections::BTreeSet<u64>,
    pub(super) started: Vec<super::receipt::Replica>,
    pub(super) terminal: Vec<super::receipt::Replica>,
    pub(super) probes: Vec<super::receipt::ProbeOutcome>,
    pub(super) rejections: Vec<super::receipt::Rejection>,
}

// r[impl molten.fabric_consistency.live_raft]
pub(super) fn fresh_phase(
    launcher: &super::super::peer::Launcher,
    directory: &std::path::Path,
) -> crate::error::Result<Fresh> {
    let mut peers = Vec::with_capacity(crate::fabric_consistency::raft::STATIC_VOTER_COUNT);
    for node_id in super::fixture::NODES {
        peers.push(launcher.spawn(&format!("{}{}{node_id}", super::ROLE_PREFIX, super::FRESH_ROLE), directory)?);
    }
    for node_id in super::fixture::NODES {
        super::super::peer::wait_value(&super::receipt::node_file(directory, node_id, "ready"), super::PHASE_TIMEOUT)?;
    }
    signal(directory, super::receipt::START_FILE, "start")?;
    super::super::peer::wait_value(&directory.join(super::receipt::LEADER_DONE_FILE), super::PHASE_TIMEOUT)?;
    signal(directory, super::receipt::CHECKPOINT_FILE, "checkpoint")?;
    let replicas = read_replicas(directory, "checkpoint")?;
    let process_ids = peers.iter().filter_map(super::super::peer::Process::id).map(u64::from).collect();
    Ok(Fresh {
        peers,
        process_ids,
        replicas,
    })
}

/// Crash every replica process abruptly, then restart all three from their
/// durable stores and drive reconciliation, re-election and the stale probes.
// r[impl molten.fabric_consistency.live_raft]
pub(super) fn crash_and_recover(
    launcher: &super::super::peer::Launcher,
    directory: &std::path::Path,
    fresh: &mut Fresh,
    stale_term: u64,
) -> crate::error::Result<Recovered> {
    for peer in &mut fresh.peers {
        peer.kill()?;
    }
    clear_fresh_signals(directory)?;
    super::super::peer::write_value(
        &directory.join(super::receipt::STALE_LEADER_FILE),
        &super::receipt::term_value(stale_term),
    )?;
    let mut peers = Vec::with_capacity(crate::fabric_consistency::raft::STATIC_VOTER_COUNT);
    for node_id in super::fixture::NODES {
        peers.push(launcher.spawn(&format!("{}{}{node_id}", super::ROLE_PREFIX, super::RECOVER_ROLE), directory)?);
    }
    let started = read_replicas(directory, "recovered")?;
    for node_id in super::fixture::NODES {
        super::super::peer::wait_value(&super::receipt::node_file(directory, node_id, "ready"), super::PHASE_TIMEOUT)?;
    }
    signal(directory, super::receipt::RESTART_FILE, "restart")?;
    let mut probes = Vec::with_capacity(super::receipt::Probe::ALL.len());
    for probe in super::receipt::Probe::ALL {
        probes.push(super::receipt::ProbeOutcome::parse(&super::super::peer::wait_value(
            &super::receipt::probe_file(directory, probe),
            super::PHASE_TIMEOUT,
        )?)?);
    }
    let mut rejections = Vec::with_capacity(2);
    for probe in [super::receipt::Probe::TermAppend, super::receipt::Probe::TermVote] {
        rejections.push(super::receipt::Rejection::parse(&super::super::peer::wait_value(
            &super::receipt::rejection_file(directory, probe),
            super::PHASE_TIMEOUT,
        )?)?);
    }
    signal(directory, super::receipt::STOP_FILE, "stop")?;
    let terminal = read_replicas(directory, "terminal")?;
    let process_ids = peers.iter().filter_map(super::super::peer::Process::id).map(u64::from).collect();
    for peer in &mut peers {
        peer.wait_success(super::EXIT_TIMEOUT)?;
    }
    Ok(Recovered {
        process_ids,
        started,
        terminal,
        probes,
        rejections,
    })
}

fn read_replicas(directory: &std::path::Path, phase: &str) -> crate::error::Result<Vec<super::receipt::Replica>> {
    super::fixture::NODES
        .iter()
        .map(|node_id| {
            let replica = super::receipt::Replica::parse(&super::super::peer::wait_value(
                &super::receipt::node_file(directory, node_id, phase),
                super::PHASE_TIMEOUT,
            )?)?;
            if replica.node_id != *node_id {
                return Err(super::super::super::invalid("live Raft replica receipt names another replica"));
            }
            Ok(replica)
        })
        .collect()
}

fn signal(directory: &std::path::Path, file: &str, label: &str) -> crate::error::Result<()> {
    super::super::peer::write_value(&directory.join(file), &super::receipt::signal(label))
}

/// Remove the crashed generation's signals and endpoints so the recovered
/// processes exchange only their own fresh endpoints and readiness.
fn clear_fresh_signals(directory: &std::path::Path) -> crate::error::Result<()> {
    let mut paths = vec![
        directory.join(super::receipt::START_FILE),
        directory.join(super::receipt::CATCH_UP_FILE),
        directory.join(super::receipt::LEADER_DONE_FILE),
        directory.join(super::receipt::CHECKPOINT_FILE),
    ];
    for node_id in super::fixture::NODES {
        for phase in ["endpoint", "ready", "checkpoint"] {
            paths.push(super::receipt::node_file(directory, node_id, phase));
        }
    }
    for path in paths {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(crate::error::Failure::from(error)),
        }
    }
    Ok(())
}
