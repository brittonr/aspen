impl super::runtime::Host {
    pub(super) async fn exchange_at(
        &mut self,
        request: &super::super::protocol::Request,
        endpoint: std::net::SocketAddr,
    ) -> super::super::protocol::Response {
        let forwarded = request.clone();
        let mut task = tokio::task::spawn_blocking(move || super::super::client::execute_at(&forwarded, endpoint));
        loop {
            tokio::select! {
                result = &mut task => return match result {
                    Ok(response) => response,
                    Err(error) => super::super::protocol::uncertain(request, error.to_string()),
                },
                event = self.timers.recv() => {
                    let Some(event) = event else { return super::super::protocol::uncertain(request, "timer-closed".into()); };
                    if let Err(error) = self.apply(event).await { return super::super::protocol::uncertain(request, error.to_string()); }
                }
                event = self.ingress.next() => {
                    match event {
                        Ok(event) => if let Err(error) = self.apply(event.event).await { return super::super::protocol::uncertain(request, error.to_string()); },
                        Err(error) => return super::super::protocol::uncertain(request, error.to_string()),
                    }
                }
            }
            while self.observations.try_recv().is_ok() {}
        }
    }
}
