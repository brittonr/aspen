pub(super) fn run(
    root: crate::node_state::DirectoryRoot,
    plan: super::AdmittedPlan,
    receiver: tokio::sync::mpsc::Receiver<super::Message>,
    ready: std::sync::mpsc::SyncSender<Result<std::time::Duration, String>>,
    stopped: std::sync::Arc<tokio::sync::Notify>,
) {
    let executor = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(executor) => executor,
        Err(_) => {
            let _failed = ready.send(Err("collaboration executor unavailable".to_string()));
            return;
        }
    };
    executor.block_on(async move {
        let parts = match super::config::start(&root, plan).await {
            Ok(parts) => parts,
            Err(_) => {
                let _failed = ready.send(Err("collaboration startup denied".to_string()));
                return;
            }
        };
        let mut runtime = super::Driver::new(parts);
        if ready.send(Ok(runtime.timeout.saturating_mul(3))).is_err() {
            return;
        }
        drive(&mut runtime, receiver, &stopped).await;
    });
}
enum Action {
    Message(Option<super::Message>),
    Event(crate::error::Result<crate::fabric_consistency::raft::ReplicaEvent>),
}
async fn drive(
    runtime: &mut super::Driver,
    mut receiver: tokio::sync::mpsc::Receiver<super::Message>,
    stopped: &tokio::sync::Notify,
) {
    loop {
        // Only receive futures are cancellable. An accepted effect finishes its
        // actual adapter operation before another message is serviced.
        let action = tokio::select! {
            biased;
            () = stopped.notified() => return,
            message = receiver.recv() => Action::Message(message),
            event = runtime.service.receive_event(), if !runtime.is_uncertain => Action::Event(event),
            incoming = runtime.ingress.next(), if !runtime.is_uncertain => Action::Event(incoming.map(|received| received.event)),
        };
        match action {
            Action::Message(None) => return,
            Action::Message(Some(message)) => handle(runtime, message).await,
            Action::Event(Ok(event)) => {
                if runtime.apply(event).await.is_err() {
                    runtime.is_uncertain = true;
                }
            }
            Action::Event(Err(_)) => runtime.is_uncertain = true,
        }
    }
}
async fn handle(runtime: &mut super::Driver, message: super::Message) {
    match message {
        super::Message::Route(binding, effect, reply) => {
            let result = runtime.commit(&binding, &effect).await.map_err(redacted_failure);
            let _delivered = reply.send(result);
        }
        super::Message::Observe(request, reply) => {
            let result = if runtime.is_uncertain {
                Err(super::super::denied(super::super::Denial::Uncertain))
            } else {
                runtime.observe_scope(request).await
            };
            let _delivered = reply.send(result.map_err(|error| {
                let message = redacted_failure(error);
                if message.starts_with("collaboration-scope/") {
                    message
                } else {
                    "collaboration-scope/uncertain".to_string()
                }
            }));
        }
    }
}

fn redacted_failure(error: crate::error::Failure) -> String {
    match error {
        crate::error::Failure::InvalidHarness(message)
            if matches!(
                message.as_str(),
                "collaboration-scope/bound"
                    | "collaboration-scope/domain"
                    | "collaboration-scope/missing"
                    | "collaboration-scope/stale"
                    | "collaboration-scope/expired"
                    | "collaboration-scope/membership"
                    | "collaboration-scope/authority"
                    | "collaboration-scope/conflict"
                    | "collaboration-scope/owner"
                    | "collaboration-scope/replay"
                    | "collaboration-scope/uncertain"
                    | "collaboration-scope/partition"
            ) =>
        {
            message
        }
        _ => "collaboration mutation denied or completion uncertain".to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn canonical_denial_remains_distinguishable_from_uncertainty() {
        let error = crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Stale);
        assert_eq!(super::redacted_failure(error), "collaboration-scope/stale");
    }

    #[test]
    fn untrusted_details_remain_redacted() {
        for message in [
            "backend secret: private-token",
            "collaboration-scope/stale/private-token",
        ] {
            let error = crate::error::Failure::invalid_harness(message);
            let disclosed = super::redacted_failure(error);
            assert!(!disclosed.contains("private-token"));
            assert_ne!(disclosed, "collaboration-scope/stale");
        }
    }
}
