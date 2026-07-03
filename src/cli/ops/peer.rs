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
        Command::InviteCreate { peer, out } => {
            write_transition(peer, molten::peer_bootstrap::PeerSessionStateKind::Invited, out)
        }
        Command::InviteAccept { peer, out } => {
            write_transition(peer, molten::peer_bootstrap::PeerSessionStateKind::Admitted, out)
        }
        Command::Connect { peer, out } => {
            write_transition(peer, molten::peer_bootstrap::PeerSessionStateKind::Connected, out)
        }
        Command::Status { peer } => {
            let session = base_session(&peer, molten::peer_bootstrap::PeerSessionStateKind::Discovered)?;
            println!("peer status peer={} state={} session={}", peer, session.state.as_str(), session.session_ref);
            Ok(())
        }
        Command::Revoke { peer, out } => {
            write_transition(peer, molten::peer_bootstrap::PeerSessionStateKind::Revoked, out)
        }
        Command::Diagnose { peer } => {
            let denial = molten::peer_bootstrap::peer_session_as_authority_denial(&synthetic_ref(&peer)?, "diagnose")?;
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

fn write_transition(peer: String, target: molten::peer_bootstrap::PeerSessionStateKind, out: FilePath) -> Outcome<()> {
    let authority_ref = synthetic_ref("authority")?;
    let bootstrap_ref = synthetic_ref("bootstrap")?;
    let prior = base_session(&peer, prior_state(target))?;
    let receipt = molten::peer_bootstrap::apply_peer_transition(&molten::peer_bootstrap::PeerTransitionInput {
        prior,
        target,
        observed_topic: "node-control".to_string(),
        at_tick: CLI_TICK,
        required_bootstrap_ref: Some(bootstrap_ref),
        required_authority_ref: Some(authority_ref),
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

fn prior_state(target: molten::peer_bootstrap::PeerSessionStateKind) -> molten::peer_bootstrap::PeerSessionStateKind {
    match target {
        molten::peer_bootstrap::PeerSessionStateKind::Invited => {
            molten::peer_bootstrap::PeerSessionStateKind::Discovered
        }
        molten::peer_bootstrap::PeerSessionStateKind::Admitted => {
            molten::peer_bootstrap::PeerSessionStateKind::Negotiated
        }
        molten::peer_bootstrap::PeerSessionStateKind::Connected => {
            molten::peer_bootstrap::PeerSessionStateKind::Admitted
        }
        molten::peer_bootstrap::PeerSessionStateKind::Revoked => {
            molten::peer_bootstrap::PeerSessionStateKind::Connected
        }
        other => other,
    }
}

fn base_session(
    peer: &str,
    state: molten::peer_bootstrap::PeerSessionStateKind,
) -> Outcome<molten::peer_bootstrap::PeerSession> {
    Ok(molten::peer_bootstrap::PeerSession {
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
