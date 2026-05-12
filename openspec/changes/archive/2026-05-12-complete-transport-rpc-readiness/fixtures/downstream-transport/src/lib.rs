use aspen_transport::ConnectionManager;
use aspen_transport::StreamManager;
use aspen_transport::constants::CLIENT_ALPN;
use aspen_transport::constants::LOG_SUBSCRIBER_ALPN;
use aspen_transport::constants::NET_TUNNEL_ALPN;

pub const EXPECTED_PROTOCOL_COUNT: usize = 3;

#[must_use]
pub fn protocol_ids() -> [&'static [u8]; EXPECTED_PROTOCOL_COUNT] {
    [CLIENT_ALPN, LOG_SUBSCRIBER_ALPN, NET_TUNNEL_ALPN]
}

#[must_use]
pub fn build_connection_manager(max_connections: u32, max_streams_per_connection: u32) -> ConnectionManager {
    ConnectionManager::new(max_connections, max_streams_per_connection)
}

#[must_use]
pub fn build_stream_manager(max_streams_per_connection: u32) -> StreamManager {
    StreamManager::new(max_streams_per_connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_ids_are_non_empty() {
        for protocol_id in protocol_ids() {
            assert!(!protocol_id.is_empty(), "protocol identifiers must be non-empty");
        }
    }

    #[test]
    fn connection_manager_reports_configured_bounds() {
        let max_connections = 2;
        let max_streams_per_connection = 4;
        let manager = build_connection_manager(max_connections, max_streams_per_connection);

        assert_eq!(manager.max_connections(), max_connections);
        assert_eq!(manager.max_streams_per_connection(), max_streams_per_connection);
    }

    #[test]
    fn stream_manager_rejects_after_bound() {
        let max_streams_per_connection = 1;
        let manager = build_stream_manager(max_streams_per_connection);

        let first = manager.try_acquire_stream();
        let second = manager.try_acquire_stream();

        assert!(first.is_some(), "first stream permit should fit bound");
        assert!(second.is_none(), "second stream permit should exceed bound");
    }
}
