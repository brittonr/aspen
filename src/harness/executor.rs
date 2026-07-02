type ActorDecl = super::schema::ActorDecl;
type ActorExecutorConfig = super::schema::ActorExecutorConfig;
type ActorKind = super::schema::ActorKind;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorExecutorKind {
    Native,
    SteelReviewed,
    SteelPlaceholder,
    WasmReviewed,
    WasmPlaceholder,
    AdapterReviewed,
    AdapterPlaceholder,
    RemoteProxyReviewed,
    RemoteProxyPlaceholder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorExecutorDecl {
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub executor_kind: ActorExecutorKind,
    pub supported: bool,
}

pub fn actor_executor_registry(actors: &[ActorDecl]) -> Vec<ActorExecutorDecl> {
    actors
        .iter()
        .map(|actor| {
            let (executor_kind, supported) = executor_for_actor(actor);
            ActorExecutorDecl {
                actor_id: actor.id.clone(),
                actor_kind: actor.kind.clone(),
                executor_kind,
                supported,
            }
        })
        .collect()
}

pub fn ensure_supported_actor_executors(actors: &[ActorDecl]) -> Result<()> {
    for executor in actor_executor_registry(actors) {
        if !executor.supported {
            let message = match executor.actor_kind {
                ActorKind::Steel => {
                    format!("steel actor {} missing reviewed Steel executor preflight fixture", executor.actor_id)
                }
                ActorKind::Native => format!("native actor {} has unsupported executor fixture", executor.actor_id),
                ActorKind::Wasm => {
                    format!("wasm actor {} missing Wasm executor preflight fixture", executor.actor_id)
                }
                ActorKind::Adapter | ActorKind::RemoteProxy => {
                    format!(
                        "executor kind {} requires executor adapter preflight and remains disabled in local harness",
                        executor.actor_kind.as_str()
                    )
                }
            };
            return Err(MoltenError::invalid_harness(message));
        }
    }
    Ok(())
}

fn executor_for_actor(actor: &ActorDecl) -> (ActorExecutorKind, bool) {
    match (&actor.kind, &actor.executor) {
        (ActorKind::Native, None) => (ActorExecutorKind::Native, true),
        (ActorKind::Steel, Some(ActorExecutorConfig::Steel(_))) => (ActorExecutorKind::SteelReviewed, true),
        (ActorKind::Steel, _) => (ActorExecutorKind::SteelPlaceholder, false),
        (ActorKind::Wasm, Some(ActorExecutorConfig::Wasm(_))) => (ActorExecutorKind::WasmReviewed, true),
        (ActorKind::Wasm, _) => (ActorExecutorKind::WasmPlaceholder, false),
        (ActorKind::Adapter, Some(ActorExecutorConfig::Adapter(_))) => (ActorExecutorKind::AdapterReviewed, true),
        (ActorKind::Adapter, _) => (ActorExecutorKind::AdapterPlaceholder, false),
        (ActorKind::RemoteProxy, Some(ActorExecutorConfig::RemoteProxy(_))) => {
            (ActorExecutorKind::RemoteProxyReviewed, true)
        }
        (ActorKind::RemoteProxy, _) => (ActorExecutorKind::RemoteProxyPlaceholder, false),
        (ActorKind::Native, Some(_)) => (ActorExecutorKind::Native, false),
    }
}
