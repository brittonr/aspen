//! Verus specs for Nostr relay configuration defaults.
//!
//! Production `src/config.rs` owns serde/schemars integration and string-valued
//! bind/relay URLs. This module verifies the pure default-resource contract that
//! the runtime shell relies on: disabled-by-default service activation,
//! open write policy, no default relay URL, positive bounded resource constants,
//! and rate-limit burst defaults large enough for their per-second rates.

use vstd::prelude::*;

verus! {

pub const DEFAULT_NOSTR_PORT: u16 = 4869;
pub const MAX_NOSTR_CONNECTIONS: u32 = 256;
pub const MAX_SUBSCRIPTIONS_PER_CONNECTION: u32 = 16;
pub const MAX_FILTERS_PER_SUBSCRIPTION: u32 = 8;
pub const MAX_EVENT_SIZE: u32 = 64 * 1024;
pub const BROADCAST_CHANNEL_CAPACITY: u32 = 4096;
pub const MAX_STORED_EVENTS: u32 = 100_000;
pub const MAX_EVENTS_PER_SECOND_PER_IP: u32 = 10;
pub const MAX_EVENTS_BURST_PER_IP: u32 = 20;
pub const MAX_EVENTS_PER_SECOND_PER_PUBKEY: u32 = 5;
pub const MAX_EVENTS_BURST_PER_PUBKEY: u32 = 10;
pub const RATE_LIMIT_BUCKET_TTL_SECS: u64 = 300;
pub const RATE_LIMIT_CLEANUP_INTERVAL_SECS: u64 = 60;

pub enum WritePolicySpec {
    Open,
    AuthRequired,
    ReadOnly,
}

pub struct NostrRelayConfigSpec {
    pub enabled: bool,
    pub bind_port: u16,
    pub max_connections: u32,
    pub max_subscriptions_per_connection: u32,
    pub max_event_size_bytes: u32,
    pub write_policy: WritePolicySpec,
    pub relay_url_present: bool,
    pub events_per_second_per_ip: u32,
    pub events_burst_per_ip: u32,
    pub events_per_second_per_pubkey: u32,
    pub events_burst_per_pubkey: u32,
}

pub open spec fn default_write_policy() -> WritePolicySpec {
    WritePolicySpec::Open
}

pub open spec fn default_config_spec() -> NostrRelayConfigSpec {
    NostrRelayConfigSpec {
        enabled: false,
        bind_port: DEFAULT_NOSTR_PORT,
        max_connections: MAX_NOSTR_CONNECTIONS,
        max_subscriptions_per_connection: MAX_SUBSCRIPTIONS_PER_CONNECTION,
        max_event_size_bytes: MAX_EVENT_SIZE,
        write_policy: default_write_policy(),
        relay_url_present: false,
        events_per_second_per_ip: MAX_EVENTS_PER_SECOND_PER_IP,
        events_burst_per_ip: MAX_EVENTS_BURST_PER_IP,
        events_per_second_per_pubkey: MAX_EVENTS_PER_SECOND_PER_PUBKEY,
        events_burst_per_pubkey: MAX_EVENTS_BURST_PER_PUBKEY,
    }
}

pub open spec fn resource_defaults_positive(config: NostrRelayConfigSpec) -> bool {
    config.bind_port > 0
        && config.max_connections > 0
        && config.max_subscriptions_per_connection > 0
        && config.max_event_size_bytes > 0
        && BROADCAST_CHANNEL_CAPACITY > 0
        && MAX_FILTERS_PER_SUBSCRIPTION > 0
        && MAX_STORED_EVENTS > 0
}

pub open spec fn rate_defaults_consistent(config: NostrRelayConfigSpec) -> bool {
    config.events_per_second_per_ip > 0
        && config.events_burst_per_ip >= config.events_per_second_per_ip
        && config.events_per_second_per_pubkey > 0
        && config.events_burst_per_pubkey >= config.events_per_second_per_pubkey
        && RATE_LIMIT_BUCKET_TTL_SECS >= RATE_LIMIT_CLEANUP_INTERVAL_SECS
        && RATE_LIMIT_CLEANUP_INTERVAL_SECS > 0
}

pub open spec fn default_config_safe(config: NostrRelayConfigSpec) -> bool {
    !config.enabled
        && config.write_policy == WritePolicySpec::Open
        && !config.relay_url_present
        && resource_defaults_positive(config)
        && rate_defaults_consistent(config)
}

pub fn default_write_policy_exec() -> (policy: WritePolicySpec)
    ensures policy == default_write_policy()
{
    WritePolicySpec::Open
}

pub fn default_config_exec() -> (config: NostrRelayConfigSpec)
    ensures config == default_config_spec()
{
    NostrRelayConfigSpec {
        enabled: false,
        bind_port: DEFAULT_NOSTR_PORT,
        max_connections: MAX_NOSTR_CONNECTIONS,
        max_subscriptions_per_connection: MAX_SUBSCRIPTIONS_PER_CONNECTION,
        max_event_size_bytes: MAX_EVENT_SIZE,
        write_policy: default_write_policy_exec(),
        relay_url_present: false,
        events_per_second_per_ip: MAX_EVENTS_PER_SECOND_PER_IP,
        events_burst_per_ip: MAX_EVENTS_BURST_PER_IP,
        events_per_second_per_pubkey: MAX_EVENTS_PER_SECOND_PER_PUBKEY,
        events_burst_per_pubkey: MAX_EVENTS_BURST_PER_PUBKEY,
    }
}

pub proof fn default_policy_is_open()
    ensures default_write_policy() == WritePolicySpec::Open
{
}

pub proof fn default_config_is_disabled_open_and_local_unadvertised()
    ensures
        !default_config_spec().enabled,
        default_config_spec().write_policy == WritePolicySpec::Open,
        !default_config_spec().relay_url_present,
{
}

pub proof fn default_port_is_positive()
    ensures DEFAULT_NOSTR_PORT > 0
{
}

pub proof fn default_resource_limits_are_positive()
    ensures resource_defaults_positive(default_config_spec())
{
}

pub proof fn default_rate_limits_are_positive_and_bursty()
    ensures rate_defaults_consistent(default_config_spec())
{
}

pub proof fn default_config_is_safe()
    ensures default_config_safe(default_config_spec())
{
}

pub proof fn default_event_size_is_64kib()
    ensures MAX_EVENT_SIZE == 65536
{
}

pub proof fn default_storage_limit_exceeds_broadcast_capacity()
    ensures MAX_STORED_EVENTS > BROADCAST_CHANNEL_CAPACITY
{
}

pub proof fn default_subscription_limit_exceeds_filter_limit()
    ensures MAX_SUBSCRIPTIONS_PER_CONNECTION >= MAX_FILTERS_PER_SUBSCRIPTION
{
}

pub proof fn ip_burst_allows_at_least_one_second_of_events()
    ensures MAX_EVENTS_BURST_PER_IP >= MAX_EVENTS_PER_SECOND_PER_IP
{
}

pub proof fn pubkey_burst_allows_at_least_one_second_of_events()
    ensures MAX_EVENTS_BURST_PER_PUBKEY >= MAX_EVENTS_PER_SECOND_PER_PUBKEY
{
}

pub proof fn cleanup_interval_fits_within_bucket_ttl()
    ensures RATE_LIMIT_CLEANUP_INTERVAL_SECS <= RATE_LIMIT_BUCKET_TTL_SECS
{
}

} // verus!
