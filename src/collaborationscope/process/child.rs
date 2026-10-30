use super::*;

pub(super) async fn run(root: &std::path::Path, node_id: &str, phase: &str) -> crate::error::Result<()> {
    let storage = root.join(node_id);
    let mut node = start_node(root, node_id, phase, &storage).await?;
    let listener = node.listener.take().ok_or_else(|| invalid("participant listener missing"))?;
    let mut ingress = ReplicaIngressPump::spawn(listener, ReplicaIngressConfig {
        session_ref: node.session_ref.clone(),
        accept_timeout: std::time::Duration::from_secs(30),
        event_capacity: 32,
        delivery_limit: 2048,
    })?;
    signal(&file(root, phase, node_id, "ready"));
    wait_signal(&root.join(format!("{phase}-start.preserves"))).await?;
    if phase == "fresh" && node_id == "node-a"
        || phase == "recover" && matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_))
    {
        election(&mut node).await?;
    }
    let mut progress = open_progress(&storage, phase)?;
    let mut has_campaigned = false;
    let mut has_reported = false;
    for step in 0..1800 {
        if phase == "recover" {
            has_campaigned = recover(&mut node, root, node_id, step, has_campaigned).await?;
        }
        if node.service.state().role == ReplicaRole::Leader {
            progress.tick(&mut node, root).await?;
        }
        if !has_reported && completed(&node, root, phase, &progress) {
            checkpoint(&mut node, root, phase, node_id).await?;
            has_reported = true;
        }
        if phase == "recover" && root.join("recover-stop.preserves").is_file() {
            node.listener = Some(ingress.shutdown().await?);
            live_cluster::setup::close_node(node).await;
            return Ok(());
        }
        let event = tokio::select! {
            received = ingress.next() => Some(received?),
            () = tokio::time::sleep(std::time::Duration::from_millis(50)) => None,
        };
        if let Some(event) = event {
            if phase == "fresh" && root.join("partition.preserves").is_file() {
                continue;
            }
            let is_lagging = phase == "fresh"
                && node_id == "node-c"
                && node.service.state().snapshot.is_none()
                && matches!(&event.event, ReplicaEvent::Message { envelope: ReplicaMessageEnvelope { message: Message::AppendEntries { entries, .. }, .. } } if !entries.is_empty());
            // A lagging peer legitimately refuses inbound messages it cannot honor (for example a
            // read probe beyond its log); the refusal retains state. Execution failures still abort.
            if !is_lagging {
                delivered(node.service.handle_event(event.event).await)?;
            }
        }
    }
    Err(invalid("collaboration process event bound"))
}
async fn start_node(
    root: &std::path::Path,
    node_id: &str,
    phase: &str,
    storage: &std::path::Path,
) -> crate::error::Result<live_cluster::Node> {
    let secret = match node_id {
        "node-a" => 101,
        "node-b" => 103,
        _ => 107,
    };
    let listener = crate::fabric_transport::cross_process::tests::listener_with_secret(secret).await;
    write(&file(root, phase, node_id, "endpoint"), &listener.handoff().value)?;
    wait_nodes(root, phase, "endpoint");
    let endpoints = NODES
        .iter()
        .map(|node| {
            Ok((
                node.to_string(),
                crate::fabric_transport::parse_endpoint(&read(&file(root, phase, node, "endpoint"))?)?,
            ))
        })
        .collect::<crate::error::Result<std::collections::BTreeMap<_, _>>>()?;
    let group = crate::fabric_consistency::raft::tests::active_group_for(
        crate::collaboration_scope::EXTENSION_ID,
        &crate::collaboration_scope::fixtures::reference("collaboration-process-manifest"),
    );
    if phase == "fresh" {
        live_cluster::setup::build_node_at_root(&group, node_id, listener, &endpoints, storage).await
    } else {
        live_cluster::setup::recover_node_at_root(&group, node_id, listener, &endpoints, storage).await
    }
}
fn open_progress(storage: &std::path::Path, phase: &str) -> crate::error::Result<flow::Progress> {
    let replay_root = storage.join("replay");
    let replay = crate::collaboration_scope::NonceLedger::new(if phase == "fresh" {
        crate::collaboration_scope::fixtures::open_store(&replay_root)?
    } else {
        crate::collaboration_scope::fixtures::recover_store(&replay_root)?
    });
    Ok(flow::Progress::new(replay, phase == "recover"))
}
async fn wait_signal(path: &std::path::Path) -> crate::error::Result<()> {
    for _ in 0..POLLS {
        if path.is_file() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    Err(invalid("participant start deadline"))
}
async fn election(node: &mut live_cluster::Node) -> crate::error::Result<()> {
    let timer_ref = node.service.state().active_election_timer_ref.clone();
    applied(node.service.handle_event(ReplicaEvent::ElectionTimeout { timer_ref }).await)
}
async fn recover(
    node: &mut live_cluster::Node,
    root: &std::path::Path,
    node_id: &str,
    step: usize,
    mut has_campaigned: bool,
) -> crate::error::Result<bool> {
    if !matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_)) {
        signal(&file(root, "recover", node_id, "reconciled"));
    }
    if node_id == "node-b"
        && !has_campaigned
        && NODES.iter().all(|node| file(root, "recover", node, "reconciled").is_file())
    {
        election(node).await?;
        has_campaigned = true;
    }
    if step > 0 && step.is_multiple_of(10) && matches!(node.service.state().rejoin, ReplicaRejoin::Recovering(_)) {
        election(node).await?;
    }
    Ok(has_campaigned)
}
fn completed(node: &live_cluster::Node, root: &std::path::Path, phase: &str, progress: &flow::Progress) -> bool {
    if phase == "fresh" {
        node.service.state().last_applied == 3
            && root.join("partition.preserves").is_file()
            && (node.service.state().role != ReplicaRole::Leader || progress.partition_pending(node))
    } else {
        node.service.state().last_applied == 4 && node.service.state().rejoin == ReplicaRejoin::Voter
    }
}
async fn checkpoint(
    node: &mut live_cluster::Node,
    root: &std::path::Path,
    phase: &str,
    node_id: &str,
) -> crate::error::Result<()> {
    let application = node
        .service
        .ports()
        .application
        .handler()
        .collaboration
        .as_ref()
        .ok_or_else(|| invalid("collaboration application missing"))?;
    let state = crate::collaboration_scope::state_value(application.state());
    let state_ref = application.state_ref().as_str().to_string();
    if phase == "fresh"
        && node.service.state().snapshot.as_ref().map(|snapshot| snapshot.last_included_index) != Some(3)
    {
        applied(
            node.service
                .handle_event(ReplicaEvent::CreateSnapshot {
                    application_state_ref: state_ref,
                })
                .await,
        )?;
    }
    if node.service.production_admitted() {
        return Err(invalid("experimental service was production promoted"));
    }
    write(&file(root, phase, node_id, "state"), &state)?;
    let evidence = crate::preserves_rail::record("collaboration-process-evidence-v1", vec![
        crate::preserves_rail::u64_value(u64::from(std::process::id())),
        crate::preserves_rail::string(node_id),
        crate::preserves_rail::string(phase),
        crate::preserves_rail::u64_value(node.service.state().current_term),
        crate::preserves_rail::u64_value(node.service.state().last_applied),
        crate::preserves_rail::string("actual-iroh-redb-not-production-admission"),
    ]);
    write(&file(root, phase, node_id, "evidence"), &evidence)?;
    signal(&file(root, phase, node_id, "done"));
    Ok(())
}
