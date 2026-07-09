type Outcome<T> = molten::error::Result<T>;
type FilePath = std::path::PathBuf;

const CLI_TICK: u64 = 1;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    InviteCreate {
        #[arg(long)]
        peer: String,
        #[arg(long)]
        out: FilePath,
    },
    InviteAccept {
        #[arg(long)]
        peer: String,
        #[arg(long)]
        out: FilePath,
    },
    Connect {
        #[arg(long)]
        peer: String,
        #[arg(long)]
        out: FilePath,
    },
    Status {
        #[arg(long)]
        peer: String,
    },
    Revoke {
        #[arg(long)]
        peer: String,
        #[arg(long)]
        out: FilePath,
    },
    Diagnose {
        #[arg(long)]
        peer: String,
    },
}

pub(crate) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::InviteCreate { peer, out } => write_transition(peer, molten::peer_bootstrap::StateKind::Invited, out),
        Command::InviteAccept { peer, out } => write_transition(peer, molten::peer_bootstrap::StateKind::Admitted, out),
        Command::Connect { peer, out } => write_transition(peer, molten::peer_bootstrap::StateKind::Connected, out),
        Command::Status { peer } => {
            let session = base_record(&peer, molten::peer_bootstrap::StateKind::Discovered)?;
            println!("peer status peer={} state={} session={}", peer, session.state.as_str(), session.session_ref);
            Ok(())
        }
        Command::Revoke { peer, out } => write_transition(peer, molten::peer_bootstrap::StateKind::Revoked, out),
        Command::Diagnose { peer } => {
            let denial = molten::peer_bootstrap::record_as_authority_denial(&synthetic_ref(&peer)?, "diagnose")?;
            println!(
                "peer diagnose peer={} decision={} diagnostics={}",
                peer,
                denial.decision,
                denial.diagnostics.len()
            );
            Ok(())
        }
    }
}

fn write_transition(peer: String, target: molten::peer_bootstrap::StateKind, out: FilePath) -> Outcome<()> {
    let authority_ref = synthetic_ref("authority")?;
    let bootstrap_ref = synthetic_ref("bootstrap")?;
    let prior = base_record(&peer, prior_state(target))?;
    let receipt = molten::peer_bootstrap::apply_transition(&molten::peer_bootstrap::TransitionInput {
        prior,
        event: event_for_target(target),
        target,
        observed_topic: "node-control".to_string(),
        at_tick: CLI_TICK,
        required_bootstrap_ref: Some(bootstrap_ref),
        required_authority_ref: Some(authority_ref.clone()),
        required_recovery_ref: None,
        revocation_ref: revocation_for_target(target, authority_ref),
    })?;
    std::fs::write(&out, molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "peer transition peer={} target={} decision={} receipt={} out={}",
        peer,
        target.as_str(),
        receipt.decision,
        receipt.receipt_ref,
        out.display()
    );
    Ok(())
}

fn event_for_target(target: molten::peer_bootstrap::StateKind) -> molten::peer_bootstrap::EventKind {
    match target {
        molten::peer_bootstrap::StateKind::Invited => molten::peer_bootstrap::EventKind::Invite,
        molten::peer_bootstrap::StateKind::Admitted => molten::peer_bootstrap::EventKind::Admit,
        molten::peer_bootstrap::StateKind::Connected => molten::peer_bootstrap::EventKind::Connect,
        molten::peer_bootstrap::StateKind::Revoked => molten::peer_bootstrap::EventKind::Revoke,
        molten::peer_bootstrap::StateKind::Expired => molten::peer_bootstrap::EventKind::Expire,
        molten::peer_bootstrap::StateKind::Quarantined => molten::peer_bootstrap::EventKind::Quarantine,
        _ => molten::peer_bootstrap::EventKind::Recover,
    }
}

fn revocation_for_target(target: molten::peer_bootstrap::StateKind, authority_ref: String) -> Option<String> {
    if target == molten::peer_bootstrap::StateKind::Revoked {
        Some(authority_ref)
    } else {
        None
    }
}

fn prior_state(target: molten::peer_bootstrap::StateKind) -> molten::peer_bootstrap::StateKind {
    match target {
        molten::peer_bootstrap::StateKind::Invited => molten::peer_bootstrap::StateKind::Discovered,
        molten::peer_bootstrap::StateKind::Admitted => molten::peer_bootstrap::StateKind::Negotiated,
        molten::peer_bootstrap::StateKind::Connected => molten::peer_bootstrap::StateKind::Admitted,
        molten::peer_bootstrap::StateKind::Revoked => molten::peer_bootstrap::StateKind::Connected,
        other => other,
    }
}

fn base_record(peer: &str, state: molten::peer_bootstrap::StateKind) -> Outcome<molten::peer_bootstrap::Record> {
    Ok(molten::peer_bootstrap::Record {
        peer_ref: synthetic_ref(peer)?,
        session_ref: synthetic_ref(&format!("session:{peer}"))?,
        topic: "node-control".to_string(),
        state,
        bootstrap_refs: vec![synthetic_ref("bootstrap")?],
        capability_refs: vec![synthetic_ref("capability")?],
        authority_refs: vec![synthetic_ref("authority")?],
        policy_refs: vec![synthetic_ref("policy")?],
        resource_refs: vec![synthetic_ref("resource")?],
        diagnostics: Vec::new(),
    })
}

fn synthetic_ref(label: &str) -> Outcome<String> {
    molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("peer-cli-ref", vec![
        molten::preserves_rail::string(label),
    ]))
}
