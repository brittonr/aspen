/// Receiver-initiated inventory exchange. Implementations must return only crypto-verified records.
pub trait InventoryPort {
    fn request(
        &mut self,
        peer: &molten_core::federation::PeerHint,
        generation: u64,
        now: u64,
    ) -> crate::error::Result<crate::mesh::envelope::network::VerifiedRecord>;
}
pub struct TickInput<'a> {
    pub durable: &'a crate::node_state::Scope,
    pub local: &'a [molten_core::federation::InventoryEntry],
    pub generation: u64,
}

/// One bounded periodic turn. The owner repeats this until shutdown; no ambient clock or spawn is
/// used.
pub fn tick<T: crate::fabric_time::TimerClockAdapter, P: InventoryPort>(
    service: &mut super::Service,
    clock: &mut T,
    inventory: &mut P,
    input: TickInput<'_>,
) -> crate::error::Result<Vec<String>> {
    service.require_running()?;
    if input.generation != service.host.state().generation
        || !service
            .manifest
            .extension()
            .manifest()
            .policy_refs
            .iter()
            .any(|reference| reference == clock.profile_ref())
    {
        return Err(invalid("periodic timer generation or profile is not admitted"));
    }
    let mut now = clock.now_ticks().map_err(|error| invalid(&error.to_string()))?;
    let path = crate::node_state::RelativePath::parse("federation-schedule.preserves")?;
    if input.durable.try_exists(&path)? {
        let value = crate::preserves_rail::parse_canonical_bytes(&input.durable.read(&path, 1024)?)?;
        let fields = value
            .collect_simple_record("federation-schedule-v1", Some(3))
            .ok_or_else(|| invalid("malformed periodic checkpoint"))?;
        if super::checkpoint::string(&fields[0])? != service.manifest.manifest_ref()
            || super::checkpoint::integer(&fields[1])? != input.generation
        {
            return Err(invalid("periodic checkpoint fenced"));
        }
        let target = super::checkpoint::integer(&fields[2])?;
        if now < target {
            now = clock.await_ticks(target).map_err(|error| invalid(&error.to_string()))?;
        }
        if now < target {
            return Err(invalid("admitted timer woke before deadline"));
        }
    }
    let next = now
        .checked_add(service.manifest.bounds().query_interval)
        .ok_or_else(|| invalid("periodic tick overflow"))?;
    let value = crate::preserves_rail::record("federation-schedule-v1", vec![
        crate::preserves_rail::string(service.manifest.manifest_ref()),
        crate::preserves_rail::u64_value(input.generation),
        crate::preserves_rail::u64_value(next),
    ]);
    input.durable.write(&path, &crate::preserves_rail::canonical_bytes(&value)?)?;
    expire(service, now, input.durable)?;
    let peers = service.manifest.peers().to_vec();
    let mut evidence = Vec::with_capacity(peers.len());
    for peer in peers {
        if service.active.len() >= service.manifest.bounds().concurrent_sessions {
            break;
        }
        if !peer.available || peer.observed_at > now || peer.expires_at < now {
            continue;
        }
        let outcome = inventory.request(&peer, input.generation, now).and_then(|verified| {
            let observed = crate::mesh::envelope::records::verified_inventory(&verified, service.manifest.bounds())?;
            if observed.peer_ref != peer.peer_ref || observed.signer_ref != peer.signer_ref {
                return Err(invalid("periodic inventory response differs from requested peer"));
            }
            service.plan_inventory(&verified, input.local, now, input.durable)
        });
        match outcome {
            Ok(session) => evidence.push(session.session_ref),
            Err(_) => evidence.push(retain_denial(service, &peer.peer_ref, now, input.durable)?),
        }
    }
    Ok(evidence)
}
fn expire(service: &mut super::Service, now: u64, durable: &crate::node_state::Scope) -> crate::error::Result<()> {
    let expired: Vec<_> = service.active.values().filter(|session| now > session.plan.deadline).cloned().collect();
    for mut session in expired {
        let denial = super::status::terminal_denial(&session.session_ref, session.generation, "deadline", now)?;
        let reference = crate::preserves_rail::canonical_hash(&denial)?;
        let slot = service.status_slot()?;
        let path = crate::node_state::RelativePath::parse(&format!("federation-terminal-{slot}.preserves"))?;
        durable.write(&path, &crate::preserves_rail::canonical_bytes(&denial)?)?;
        session = molten_core::federation::advance_session(
            &session,
            &molten_core::federation::SessionEvent::Denied {
                evidence_ref: reference,
            },
            service.manifest.generation(),
            now,
            service.manifest.bounds(),
        )
        .map_err(|_| invalid("terminal deadline transition rejected"))?;
        service.retain_status(&session, durable)?;
        service.active.remove(&session.session_ref);
    }
    Ok(())
}
fn retain_denial(
    service: &super::Service,
    peer: &str,
    now: u64,
    durable: &crate::node_state::Scope,
) -> crate::error::Result<String> {
    let value = super::status::terminal_denial(peer, service.manifest.generation(), "inventory-rejected", now)?;
    let path = crate::node_state::RelativePath::parse(&format!("federation-denial-{}.preserves", &peer[7..]))?;
    durable.write(&path, &crate::preserves_rail::canonical_bytes(&value)?)?;
    crate::preserves_rail::canonical_hash(&value)
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
