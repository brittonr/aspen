//! NIP-42 client authentication — challenge generation and kind 22242
//! event verification.
//!
//! Each WebSocket connection gets an `AuthState` that tracks the issued
//! challenge and the authenticated pubkey (if any). Auth state is local
//! to the connection — it doesn't persist across reconnections.

use std::fmt;

use nostr::prelude::*;

use crate::constants;

/// Per-connection NIP-42 authentication state.
pub struct AuthState {
    /// Hex-encoded challenge sent to the client on connect.
    challenge_hex: String,
    /// Pubkey of the authenticated client, set after successful AUTH.
    authed_pubkey: Option<PublicKey>,
}

/// Errors from NIP-42 auth event verification.
#[derive(Debug)]
pub enum AuthError {
    /// Event is not kind 22242.
    WrongKind,
    /// The `challenge` tag does not match the issued challenge.
    InvalidChallenge,
    /// The `relay` tag does not match the configured relay URL.
    InvalidRelayUrl,
    /// `created_at` is outside the ±60s acceptance window.
    TimestampOutOfRange,
    /// Event signature verification failed.
    InvalidSignature(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongKind => write!(f, "must be kind 22242"),
            Self::InvalidChallenge => write!(f, "invalid challenge"),
            Self::InvalidRelayUrl => write!(f, "invalid relay url"),
            Self::TimestampOutOfRange => write!(f, "event too old or too new"),
            Self::InvalidSignature(msg) => write!(f, "invalid signature: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthState {
    /// Create a new auth state with a random challenge.
    ///
    /// Generates `AUTH_CHALLENGE_BYTES` (32) random bytes and hex-encodes
    /// them as the challenge string.
    pub fn new() -> Self {
        let mut bytes = [0u8; constants::AUTH_CHALLENGE_BYTES];
        ::rand::RngCore::fill_bytes(&mut ::rand::rng(), &mut bytes);
        Self {
            challenge_hex: hex::encode(bytes),
            authed_pubkey: None,
        }
    }

    /// The hex-encoded challenge string to send in `["AUTH", <challenge>]`.
    pub fn challenge_hex(&self) -> &str {
        &self.challenge_hex
    }

    /// The authenticated pubkey, if NIP-42 completed successfully.
    pub fn authed_pubkey(&self) -> Option<&PublicKey> {
        self.authed_pubkey.as_ref()
    }

    /// Whether the connection has completed NIP-42 authentication.
    pub fn is_authenticated(&self) -> bool {
        self.authed_pubkey.is_some()
    }

    /// Verify a kind 22242 auth event and, on success, mark the
    /// connection as authenticated.
    ///
    /// Returns the verified pubkey on success.
    pub fn verify_and_authenticate(
        &mut self,
        event: &Event,
        relay_url: Option<&str>,
        now_secs: u64,
    ) -> Result<PublicKey, AuthError> {
        let pubkey = verify_auth_event(event, &self.challenge_hex, relay_url, now_secs)?;
        self.authed_pubkey = Some(pubkey);
        Ok(pubkey)
    }
}

/// Verify a NIP-42 kind 22242 auth event.
///
/// Checks:
/// 1. `event.kind == 22242`
/// 2. `challenge` tag matches the issued challenge
/// 3. `relay` tag matches `relay_url` (skipped if `relay_url` is `None`)
/// 4. `created_at` within ±`AUTH_TIMESTAMP_WINDOW_SECS` of `now_secs`
/// 5. Event signature is valid
fn verify_auth_event(
    event: &Event,
    expected_challenge: &str,
    relay_url: Option<&str>,
    now_secs: u64,
) -> Result<PublicKey, AuthError> {
    // 1. Kind check
    if event.kind != Kind::Custom(constants::AUTH_EVENT_KIND) {
        return Err(AuthError::WrongKind);
    }

    // 2. Challenge tag
    let challenge_value = extract_tag(event, "challenge");
    match challenge_value {
        Some(c) if c == expected_challenge => {}
        _ => return Err(AuthError::InvalidChallenge),
    }

    // 3. Relay URL tag (skip if not configured)
    if let Some(expected_url) = relay_url {
        let relay_value = extract_tag(event, "relay");
        match relay_value {
            Some(r) if r == expected_url => {}
            _ => return Err(AuthError::InvalidRelayUrl),
        }
    }

    // 4. Timestamp window
    let created = event.created_at.as_secs();
    let delta = created.abs_diff(now_secs);
    if delta > constants::AUTH_TIMESTAMP_WINDOW_SECS {
        return Err(AuthError::TimestampOutOfRange);
    }

    // 5. Signature
    event.verify().map_err(|e| AuthError::InvalidSignature(e.to_string()))?;

    Ok(event.pubkey)
}

/// Extract the first value for a given tag name from an event.
fn extract_tag<'a>(event: &'a Event, tag_name: &str) -> Option<&'a str> {
    for tag in event.tags.iter() {
        let s = tag.as_slice();
        if s.len() >= 2 && s[0] == tag_name {
            return Some(&s[1]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid kind 22242 auth event with the given parameters.
    fn build_auth_event(keys: &Keys, challenge: &str, relay_url: &str, created_at: Timestamp) -> Event {
        let tags = vec![
            Tag::custom(TagKind::custom("relay"), [relay_url]),
            Tag::custom(TagKind::custom("challenge"), [challenge]),
        ];
        EventBuilder::new(Kind::Custom(22242), "")
            .tags(tags)
            .custom_created_at(created_at)
            .sign_with_keys(keys)
            .expect("signing failed")
    }

    fn now_timestamp() -> (u64, Timestamp) {
        let now = Timestamp::now().as_secs();
        (now, Timestamp::from_secs(now))
    }

    #[test]
    fn valid_auth_event_passes() {
        let keys = Keys::generate();
        let challenge = "a".repeat(64);
        let relay = "wss://relay.example.com";
        let (now, ts) = now_timestamp();

        let event = build_auth_event(&keys, &challenge, relay, ts);
        let result = verify_auth_event(&event, &challenge, Some(relay), now);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), keys.public_key());
    }

    #[test]
    fn wrong_kind_rejected() {
        let keys = Keys::generate();
        let challenge = "b".repeat(64);
        let (now, ts) = now_timestamp();

        // kind 1 instead of 22242
        let event = EventBuilder::new(Kind::TextNote, "hello").custom_created_at(ts).sign_with_keys(&keys).unwrap();

        let result = verify_auth_event(&event, &challenge, None, now);
        assert!(matches!(result, Err(AuthError::WrongKind)));
    }

    #[test]
    fn wrong_challenge_rejected() {
        let keys = Keys::generate();
        let relay = "wss://relay.example.com";
        let (now, ts) = now_timestamp();

        let event = build_auth_event(&keys, "wrong_challenge", relay, ts);
        let result = verify_auth_event(&event, "correct_challenge", Some(relay), now);
        assert!(matches!(result, Err(AuthError::InvalidChallenge)));
    }

    #[test]
    fn wrong_relay_url_rejected() {
        let keys = Keys::generate();
        let challenge = "c".repeat(64);
        let (now, ts) = now_timestamp();

        let event = build_auth_event(&keys, &challenge, "wss://wrong.example.com", ts);
        let result = verify_auth_event(&event, &challenge, Some("wss://correct.example.com"), now);
        assert!(matches!(result, Err(AuthError::InvalidRelayUrl)));
    }

    #[test]
    fn relay_url_check_skipped_when_none() {
        let keys = Keys::generate();
        let challenge = "d".repeat(64);
        let (now, ts) = now_timestamp();

        // Event has a relay tag but relay_url config is None → skip check
        let event = build_auth_event(&keys, &challenge, "wss://any.relay.com", ts);
        let result = verify_auth_event(&event, &challenge, None, now);
        assert!(result.is_ok());
    }

    #[test]
    fn expired_timestamp_rejected() {
        let keys = Keys::generate();
        let challenge = "e".repeat(64);
        let relay = "wss://relay.example.com";
        let now = Timestamp::now().as_secs();

        // 120 seconds in the past
        let old_ts = Timestamp::from_secs(now.saturating_sub(120));
        let event = build_auth_event(&keys, &challenge, relay, old_ts);
        let result = verify_auth_event(&event, &challenge, Some(relay), now);
        assert!(matches!(result, Err(AuthError::TimestampOutOfRange)));
    }

    #[test]
    fn future_timestamp_rejected() {
        let keys = Keys::generate();
        let challenge = "f".repeat(64);
        let relay = "wss://relay.example.com";
        let now = Timestamp::now().as_secs();

        // 120 seconds in the future
        let future_ts = Timestamp::from_secs(now.saturating_add(120));
        let event = build_auth_event(&keys, &challenge, relay, future_ts);
        let result = verify_auth_event(&event, &challenge, Some(relay), now);
        assert!(matches!(result, Err(AuthError::TimestampOutOfRange)));
    }

    #[test]
    fn auth_state_lifecycle() {
        let keys = Keys::generate();
        let mut state = AuthState::new();
        assert!(!state.is_authenticated());

        let relay = "wss://relay.example.com";
        let (now, ts) = now_timestamp();
        let event = build_auth_event(&keys, state.challenge_hex(), relay, ts);

        let result = state.verify_and_authenticate(&event, Some(relay), now);
        assert!(result.is_ok());
        assert!(state.is_authenticated());
        assert_eq!(state.authed_pubkey(), Some(&keys.public_key()));
    }

    #[test]
    fn each_auth_state_gets_unique_challenge() {
        let s1 = AuthState::new();
        let s2 = AuthState::new();
        assert_ne!(s1.challenge_hex(), s2.challenge_hex());
    }
}
