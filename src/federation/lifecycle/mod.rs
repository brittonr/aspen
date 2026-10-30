pub(super) mod checkpoint;
pub mod extension;
mod recovery;
pub(super) mod resources;
pub mod scheduling;
pub mod status;

pub struct Service {
    pub(super) manifest: super::admission::Manifest,
    pub(super) host: crate::system_extension::Dispatcher<extension::Executor>,
    pub(super) active: std::collections::BTreeMap<String, molten_core::federation::Session>,
    pub(super) status_sequence: u64,
}
impl Service {
    // r[impl molten.federated_pull_runtime.manifest]
    pub fn start(
        manifest: super::admission::Manifest,
        host: crate::system_extension::Dispatcher<extension::Executor>,
        durable: &crate::node_state::Scope,
    ) -> crate::error::Result<Self> {
        let mut service = Self {
            manifest,
            host,
            active: std::collections::BTreeMap::new(),
            status_sequence: 0,
        };
        service.require_running()?;
        recovery::restore(&mut service, durable)?;
        Ok(service)
    }

    pub fn stop(&mut self, now: u64) -> crate::error::Result<crate::system_extension::LifecycleReceipt> {
        self.require_running()?;
        self.host.shutdown(now)
    }

    // r[impl molten.federated_pull_runtime.receiver_pull]
    // r[impl molten.federated_pull_runtime.bounds_freshness]
    pub fn plan_inventory(
        &mut self,
        verified: &super::envelope::network::VerifiedRecord,
        local: &[molten_core::federation::InventoryEntry],
        now: u64,
        durable: &crate::node_state::Scope,
    ) -> crate::error::Result<molten_core::federation::Session> {
        self.require_running()?;
        let inventory = super::envelope::records::verified_inventory(verified, self.manifest.bounds())?;
        let last_query = load_query(durable, &inventory.peer_ref)?;
        let resource_history = resources::load(durable)?;
        let plan = molten_core::federation::plan_pull(&molten_core::federation::PullInput {
            bounds: self.manifest.bounds(),
            configured: self.manifest.peers(),
            inventory: &inventory,
            local,
            generation: self.manifest.generation(),
            now,
            last_query,
            resource_history: &resource_history,
            active_sessions: self.active.len(),
            receiver_requested: true,
        })
        .map_err(|issue| invalid(&format!("receiver federation plan denied: {issue:?}")))?;
        let session = planned_session(self.manifest.manifest_ref(), self.manifest.generation(), plan, verified)?;
        let session_ref = session.session_ref.clone();
        if self.active.contains_key(&session_ref) {
            return Err(invalid("federation session is already active"));
        }
        let reserved =
            molten_core::federation::reserve_resources(&resource_history, &session.plan, self.manifest.bounds(), now)
                .map_err(|issue| invalid(&format!("federation resource reservation denied: {issue:?}")))?;
        resources::save(durable, &reserved)?;
        save_query(durable, &inventory.peer_ref, now)?;
        self.retain_status(&session, durable)?;
        if session.phase != molten_core::federation::SessionPhase::Committed {
            self.active.insert(session_ref, session.clone());
        }
        Ok(session)
    }

    /// The initial attempt is reserved during planning; subsequent attempts reserve again before
    /// any transfer.
    pub fn admit_fetch(
        &self,
        session: &molten_core::federation::Session,
        now: u64,
        durable: &crate::node_state::Scope,
    ) -> crate::error::Result<()> {
        self.require_running()?;
        if session.generation != self.manifest.generation()
            || session.phase != molten_core::federation::SessionPhase::Fetching
            || session.attempts == 0
            || now > session.plan.deadline
        {
            return Err(invalid("resource fetch reservation is fenced or expired"));
        }
        let history = resources::load(durable)?;
        if session.attempts == 1 {
            if session.plan.missing.iter().any(|entry| {
                !history.iter().any(|item| {
                    item.head_ref == entry.head_ref
                        && item.last_query <= now
                        && now - item.window_started < self.manifest.bounds().session_ticks
                        && item.bytes >= entry.bytes
                        && item.dag_nodes >= entry.dag_nodes
                        && item.chunks >= entry.chunks
                })
            }) {
                return Err(invalid("initial fetch lacks a retained resource reservation"));
            }
            return Ok(());
        }
        let reserved = molten_core::federation::reserve_resources(&history, &session.plan, self.manifest.bounds(), now)
            .map_err(|issue| invalid(&format!("federation retry resource reservation denied: {issue:?}")))?;
        resources::save(durable, &reserved)
    }

    pub fn require_running(&self) -> crate::error::Result<()> {
        if self.host.state().phase != crate::system_extension::LifecyclePhase::Running
            || self.host.state().generation != self.manifest.generation()
            || self.host.manifest().manifest_ref() != self.manifest.extension().manifest_ref()
        {
            return Err(invalid("federation service is not running under its admitted generation"));
        }
        Ok(())
    }

    pub fn retain_status(
        &mut self,
        session: &molten_core::federation::Session,
        durable: &crate::node_state::Scope,
    ) -> crate::error::Result<String> {
        self.require_running()?;
        let value = status::session_value(session)?;
        if session.generation != self.host.state().generation || self.manifest.bounds().retained_status == 0 {
            return Err(invalid("status generation or retention bound is invalid"));
        }
        let next = self.status_sequence.checked_add(1).ok_or_else(|| invalid("status sequence overflow"))?;
        let slot = self.status_slot()?;
        let path = crate::node_state::RelativePath::parse(&format!("federation-status-{slot}.preserves"))?;
        durable.write(&path, &crate::preserves_rail::canonical_bytes(&value)?)?;
        recovery::save(self, next, session, durable)?;
        self.status_sequence = next;
        crate::preserves_rail::canonical_hash(&value)
    }

    /// Ring slot for the next retained status record; division by the retention bound is checked.
    pub(super) fn status_slot(&self) -> crate::error::Result<u64> {
        let retained = u64::try_from(self.manifest.bounds().retained_status)
            .map_err(|_| invalid("status bound does not fit u64"))?;
        self.status_sequence.checked_rem(retained).ok_or_else(|| invalid("status retention bound is zero"))
    }
}

fn query_path(peer: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::preserves_rail::validate_content_ref(peer)?;
    crate::node_state::RelativePath::parse(&format!("federation-query-{}.preserves", &peer[7..]))
}
fn load_query(scope: &crate::node_state::Scope, peer: &str) -> crate::error::Result<Option<u64>> {
    let path = query_path(peer)?;
    if !scope.try_exists(&path)? {
        return Ok(None);
    }
    let value = crate::preserves_rail::parse_canonical_bytes(&scope.read(&path, 256)?)?;
    let fields = value
        .collect_simple_record("federation-query-v1", Some(2))
        .ok_or_else(|| invalid("malformed durable federation query"))?;
    if fields[0].as_string().as_deref() != Some(peer) {
        return Err(invalid("durable query peer mismatch"));
    }
    let tick = fields[1]
        .as_u64()
        .ok_or_else(|| invalid("invalid durable query tick"))?
        .map_err(|_| invalid("durable query tick overflow"))?;
    Ok(Some(tick))
}
fn save_query(scope: &crate::node_state::Scope, peer: &str, now: u64) -> crate::error::Result<()> {
    let value = crate::preserves_rail::record("federation-query-v1", vec![
        crate::preserves_rail::string(peer),
        crate::preserves_rail::u64_value(now),
    ]);
    scope.write(&query_path(peer)?, &crate::preserves_rail::canonical_bytes(&value)?)
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

pub(super) fn planned_session(
    manifest_ref: &str,
    generation: u64,
    plan: molten_core::federation::PullPlan,
    verified: &super::envelope::network::VerifiedRecord,
) -> crate::error::Result<molten_core::federation::Session> {
    crate::preserves_rail::validate_content_ref(manifest_ref)?;
    let value = crate::preserves_rail::record("federation-session-v1", vec![
        crate::preserves_rail::string(manifest_ref),
        crate::preserves_rail::string(&verified.record().signature.outcome_ref),
        crate::preserves_rail::sequence(
            plan.missing.iter().map(|entry| crate::preserves_rail::string(&entry.head_ref)).collect(),
        ),
    ]);
    let phase = if plan.missing.is_empty() {
        molten_core::federation::SessionPhase::Committed
    } else {
        molten_core::federation::SessionPhase::Planned
    };
    Ok(molten_core::federation::Session {
        session_ref: crate::preserves_rail::canonical_hash(&value)?,
        generation,
        plan,
        phase,
        verified: Vec::new(),
        admitted: Vec::new(),
        committed: Vec::new(),
        evidence_refs: vec![verified.verification().outcome_ref.clone()],
        attempts: 0,
    })
}
