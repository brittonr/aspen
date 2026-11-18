#[path = "cancellation/service.rs"]
mod service;

#[path = "cancellation/abandoned.rs"]
mod abandoned;
#[path = "cancellation/open.rs"]
mod open;
#[path = "cancellation/transfer.rs"]
mod transfer;

struct Pair {
    listener: molten::fabric_transport::direct::Acceptor,
    outbound: molten::fabric_transport::direct::Outbound,
    listener_service: service::Service,
    client_service: service::Service,
    listener_peer: tokio::net::UnixStream,
    client_peer: tokio::net::UnixStream,
}

fn configuration() -> crate::config::RoleConfig {
    crate::config::RoleConfig {
        role: "listener".to_owned(),
        case: "positive".to_owned(),
        root: std::path::PathBuf::new(),
        generation: 1,
        client_generation: 1,
        bind_ip: std::net::Ipv4Addr::LOCALHOST.into(),
        bind_port: 0,
    }
}

fn outbound(
    listener: &molten::fabric_transport::direct::Acceptor,
) -> crate::TestResult<molten::fabric_transport::direct::Outbound> {
    Ok(molten::fabric_transport::direct::Outbound {
        input: crate::config::client_input(&configuration(), listener.handoff().clone())?,
        peer: crate::config::peer(true, 1)?,
        bounds: crate::config::bounds(),
        request: crate::config::request(1)?,
        clock: std::time::Instant::now,
    })
}

async fn pair(action: service::OpenAction) -> crate::TestResult<Pair> {
    let listener = molten::fabric_transport::direct::Acceptor::bind(molten::fabric_transport::direct::Config {
        listener: crate::config::listener_input(&configuration())?,
        peer: crate::config::peer(false, 1)?,
        bounds: crate::config::bounds(),
        // Leave one source token available so an abandoned-reservation retry
        // cannot be mistaken for an unrelated rate-limit denial.
        receive_budget: molten::fabric_transport::ReceiveBudget {
            burst: 2,
            refill_ticks: 100,
            max_sources: 1,
        },
        clock: std::time::Instant::now,
    })
    .await?;
    let outbound = outbound(&listener)?;
    let (listener_socket, listener_peer) = tokio::net::UnixStream::pair()?;
    let (client_socket, client_peer) = tokio::net::UnixStream::pair()?;
    Ok(Pair {
        listener,
        outbound,
        listener_service: service::Service::new(listener_socket, false, action)?,
        client_service: service::Service::new(client_socket, true, service::OpenAction::Ready)?,
        listener_peer,
        client_peer,
    })
}
