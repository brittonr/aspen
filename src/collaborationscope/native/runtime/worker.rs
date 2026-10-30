pub(super) fn run(
    root: crate::node_state::DirectoryRoot,
    plan: super::AdmittedPlan,
    receiver: tokio::sync::mpsc::Receiver<super::Message>,
    ready: std::sync::mpsc::SyncSender<Result<std::time::Duration, String>>,
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
        drive(&mut runtime, receiver).await;
    });
}
enum Action {
    Message(Option<super::Message>),
    Event(crate::error::Result<crate::fabric_consistency::raft::ReplicaEvent>),
}
async fn drive(runtime: &mut super::Driver, mut receiver: tokio::sync::mpsc::Receiver<super::Message>) {
    loop {
        // Only receive futures are cancellable. An accepted effect finishes its
        // actual adapter operation before another message is serviced.
        let action = tokio::select! {
            biased;
            message = receiver.recv() => Action::Message(message),
            event = runtime.service.receive_event(), if !runtime.is_uncertain => Action::Event(event),
            incoming = runtime.ingress.next(), if !runtime.is_uncertain => Action::Event(incoming.map(|received| received.event)),
        };
        match action {
            Action::Message(None | Some(super::Message::Stop)) => return,
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
            let result = runtime
                .commit(&binding, &effect)
                .await
                .map_err(|_| "collaboration mutation denied or completion uncertain".to_string());
            let _delivered = reply.send(result);
        }
        super::Message::Read(request, reply) => {
            let result = if runtime.is_uncertain {
                Err(super::super::denied(super::super::Denial::Uncertain))
            } else {
                runtime.observe_read(&request).await
            };
            let _delivered = reply.send(result.map_err(|_| "collaboration currentness challenge denied".to_string()));
        }
        super::Message::Stop => {}
    }
}
