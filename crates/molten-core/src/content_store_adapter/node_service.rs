//! Bounded node content intent. No filesystem, network, identity discovery, or clocks.
use super::*;

pub const NODE_CONTENT_SCHEMA: &str = "molten.node-content.v1";
pub const NODE_CONTENT_MAX_BYTES: u64 = 1_048_576;
pub const NODE_CONTENT_CHUNK_BYTES: u64 = 65_536;
pub const NODE_CONTENT_READ_SECONDS: u64 = 10;
const NODE_CONTENT_MAX_DURATION_MS: u64 = 300_000;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeContentConfig {
    pub schema: String,
    pub manifest_ref: String,
    pub readers: Vec<String>,
    pub bind_addr: std::net::SocketAddr,
    pub tick_ms: u64,
}

#[derive(Debug, Clone)]
pub struct NodeContentPlan {
    grant: ContentReadGrant,
    bind_addr: std::net::SocketAddr,
    tick_ms: u64,
}

impl NodeContentPlan {
    // r[impl molten.node_content.lifecycle]
    pub fn admit(config: NodeContentConfig, policy_ref: String, ticks: u64) -> Result<Self, &'static str> {
        if config.schema != NODE_CONTENT_SCHEMA {
            return Err("node content schema denied");
        }
        let bind_ip = config.bind_addr.ip();
        if bind_ip.is_unspecified() || bind_ip.is_multicast() || config.bind_addr.port() == 0 {
            return Err("node content address denied");
        }
        if !(100..=1_000).contains(&config.tick_ms) || !(1..=4_096).contains(&ticks) {
            return Err("node content duration denied");
        }
        if ticks.checked_mul(config.tick_ms).is_none_or(|duration_ms| duration_ms > NODE_CONTENT_MAX_DURATION_MS) {
            return Err("node content duration denied");
        }
        let grant = ContentReadGrant::from_operator_policy(policy_ref, config.manifest_ref, config.readers)
            .map_err(|_| "node content read grant denied")?;
        Ok(Self {
            grant,
            bind_addr: config.bind_addr,
            tick_ms: config.tick_ms,
        })
    }
    pub fn grant(&self) -> &ContentReadGrant {
        &self.grant
    }
    pub fn bind_addr(&self) -> std::net::SocketAddr {
        self.bind_addr
    }
    pub fn tick_ms(&self) -> u64 {
        self.tick_ms
    }
}

pub fn node_content_bounds() -> ContentResourceBounds {
    ContentResourceBounds {
        max_total_bytes: NODE_CONTENT_MAX_BYTES,
        max_chunk_count: 16,
        max_chunk_bytes: NODE_CONTENT_CHUNK_BYTES,
        max_range_bytes: NODE_CONTENT_MAX_BYTES,
        max_concurrent_operations: 2,
        max_queued_bytes: NODE_CONTENT_MAX_BYTES,
        max_memory_bytes: NODE_CONTENT_MAX_BYTES.saturating_mul(2),
        max_deadline_ticks: 60,
        max_retries: 1,
        max_events: 64,
        max_status_entries: 16,
    }
}

pub fn admit_node_archive(bytes: &[u8], expected: &str) -> Result<(), &'static str> {
    if bytes.is_empty() || u64::try_from(bytes.len()).map_or(true, |length_bytes| length_bytes > NODE_CONTENT_MAX_BYTES) {
        return Err("node content archive digest or bounds denied");
    }
    if !valid_reader_key(expected) {
        return Err("node content archive digest or bounds denied");
    }
    if blake3::hash(bytes).to_hex().as_str() != expected {
        return Err("node content archive digest or bounds denied");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> NodeContentConfig {
        NodeContentConfig {
            schema: NODE_CONTENT_SCHEMA.into(),
            manifest_ref: format!("blake3:{}", "a".repeat(64)),
            readers: vec!["b".repeat(64)],
            bind_addr: "192.0.2.1:17888".parse().unwrap(),
            tick_ms: 250,
        }
    }
    fn admit(c: NodeContentConfig, ticks: u64) -> Result<NodeContentPlan, &'static str> {
        NodeContentPlan::admit(c, format!("blake3:{}", "c".repeat(64)), ticks)
    }
    #[test]
    fn bounded_specific_grant_admits() {
        let c = config();
        let p = admit(c.clone(), 1200).unwrap();
        assert!(p.grant().allows(&c.manifest_ref, &c.readers[0]));
        assert!(!p.grant().allows(&c.manifest_ref, &"d".repeat(64)));
        assert_eq!(p.tick_ms(), 250);
    }
    #[test]
    fn malformed_empty_duplicate_and_overbound_requests_deny() {
        for ticks in [0, 1201, u64::MAX] {
            assert!(admit(config(), ticks).is_err());
        }
        for address in ["0.0.0.0:17888", "192.0.2.1:0", "224.0.0.1:17888"] {
            let mut c = config();
            c.bind_addr = address.parse().unwrap();
            assert!(admit(c, 1).is_err());
        }
        for readers in [vec![], vec!["b".repeat(64); 2], vec!["invalid".into()]] {
            let mut c = config();
            c.readers = readers;
            assert!(admit(c, 1).is_err());
        }
        let mut c = config();
        c.schema = "unknown".into();
        assert!(admit(c, 1).is_err());
        let mut c = config();
        c.tick_ms = 0;
        assert!(admit(c, 1).is_err());
    }
    #[test]
    fn archive_identity_fails_closed() {
        let bytes = b"bounded archive";
        assert!(admit_node_archive(bytes, blake3::hash(bytes).to_hex().as_str()).is_ok());
        assert!(admit_node_archive(bytes, &"a".repeat(64)).is_err());
        assert!(admit_node_archive(&[], &"a".repeat(64)).is_err());

        let maximum = vec![b'a'; NODE_CONTENT_MAX_BYTES as usize];
        assert!(admit_node_archive(&maximum, blake3::hash(&maximum).to_hex().as_str()).is_ok());
        let too_large = vec![b'a'; NODE_CONTENT_MAX_BYTES.saturating_add(1) as usize];
        assert!(admit_node_archive(&too_large, blake3::hash(&too_large).to_hex().as_str()).is_err());
    }
}
