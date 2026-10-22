//! A loopback UDP forwarder between the client and the listener.
//!
//! The client dials the forwarder instead of the listener, so every Iroh QUIC
//! datagram of the real transport passes through it. While partitioned, the
//! forwarder drops datagrams in both directions; healing resumes forwarding.
//! The endpoint identity still authenticates the peer, so the forwarder only
//! decides whether datagrams arrive.

const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(20);
const DATAGRAM_BYTES: usize = 65_536;

type Flag = std::sync::Arc<std::sync::atomic::AtomicBool>;
type Socket = std::sync::Arc<std::net::UdpSocket>;
type ClientAddress = std::sync::Arc<std::sync::Mutex<Option<std::net::SocketAddr>>>;

fn is_set(flag: &Flag) -> bool {
    flag.load(std::sync::atomic::Ordering::Acquire)
}

fn set(flag: &Flag, value: bool) {
    flag.store(value, std::sync::atomic::Ordering::Release);
}

/// The shared state of both forwarding directions.
#[derive(Clone)]
struct Route {
    front: Socket,
    back: Socket,
    client: ClientAddress,
    is_partitioned: Flag,
    is_stopped: Flag,
}

impl Route {
    /// Client to listener. The latest client source address is the return path.
    fn outbound(self) {
        let mut datagram = vec![0_u8; DATAGRAM_BYTES];
        while !is_set(&self.is_stopped) {
            let Ok((read, source)) = self.front.recv_from(&mut datagram) else {
                continue;
            };
            if let Ok(mut client) = self.client.lock() {
                *client = Some(source);
            }
            if !is_set(&self.is_partitioned) {
                let _sent = self.back.send(&datagram[..read]);
            }
        }
    }

    /// Listener to client.
    fn inbound(self) {
        let mut datagram = vec![0_u8; DATAGRAM_BYTES];
        while !is_set(&self.is_stopped) {
            let Ok(read) = self.back.recv(&mut datagram) else {
                continue;
            };
            let destination = self.client.lock().ok().and_then(|client| *client);
            if let Some(destination) = destination
                && !is_set(&self.is_partitioned)
            {
                let _sent = self.front.send_to(&datagram[..read], destination);
            }
        }
    }
}

pub(crate) struct Forwarder {
    front_addr: std::net::SocketAddr,
    route: Route,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Forwarder {
    /// Forward datagrams between a fresh loopback front address and the listener.
    pub(crate) fn start(target: std::net::SocketAddr) -> std::io::Result<Self> {
        let front = std::net::UdpSocket::bind(("127.0.0.1", 0))?;
        let back = std::net::UdpSocket::bind(("127.0.0.1", 0))?;
        back.connect(target)?;
        front.set_read_timeout(Some(READ_TIMEOUT))?;
        back.set_read_timeout(Some(READ_TIMEOUT))?;
        let front_addr = front.local_addr()?;
        let route = Route {
            front: std::sync::Arc::new(front),
            back: std::sync::Arc::new(back),
            client: std::sync::Arc::new(std::sync::Mutex::new(None)),
            is_partitioned: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            is_stopped: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        let outbound = route.clone();
        let inbound = route.clone();
        Ok(Self {
            front_addr,
            route,
            threads: vec![
                std::thread::spawn(move || outbound.outbound()),
                std::thread::spawn(move || inbound.inbound()),
            ],
        })
    }

    pub(crate) fn front_addr(&self) -> std::net::SocketAddr {
        self.front_addr
    }

    pub(crate) fn partition(&self) {
        set(&self.route.is_partitioned, true);
    }

    pub(crate) fn heal(&self) {
        set(&self.route.is_partitioned, false);
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        set(&self.route.is_stopped, true);
        for thread in self.threads.drain(..) {
            let _join = thread.join();
        }
    }
}
