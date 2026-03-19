//! NIP-34 bridge: publishes Forge events as Nostr events to the embedded relay.
//!
//! Converts ForgePushCompleted hook events into NIP-34 events and publishes
//! them to the relay. The bridge is fire-and-forget — relay failures are
//! logged but don't affect forge operations.
//!
//! NIP-34 event kinds:
//! - 30617: Repository announcement (parameterized replaceable, d-tag = repo ID)
//! - 1617:  Patch (not yet implemented)
//! - 1621:  Issue (not yet implemented)

use std::sync::Arc;

use aspen_hooks_types::event::ForgePushCompletedPayload;
use aspen_hooks_types::event::HookEvent;
use aspen_hooks_types::event::HookEventType;
use nostr::prelude::*;
use tracing::debug;
use tracing::warn;

/// NIP-34 event kind for repository announcements.
const KIND_REPO_ANNOUNCEMENT: u16 = 30617;

/// Build a NIP-34 kind 30617 repo announcement event.
pub fn build_repo_announcement(repo_id: &str, repo_name: &str, description: Option<&str>) -> EventBuilder {
    let mut tags = vec![
        Tag::custom(TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::D)), [repo_id]),
        Tag::custom(TagKind::Custom("name".into()), [repo_name]),
    ];
    if let Some(desc) = description {
        tags.push(Tag::custom(TagKind::Custom("description".into()), [desc]));
    }
    EventBuilder::new(Kind::Custom(KIND_REPO_ANNOUNCEMENT), "").tags(tags)
}

/// Build a NIP-34 event for a push (ref update).
///
/// Published as a regular event (not replaceable) with repo, ref, and commit tags.
pub fn build_push_event(repo_id: &str, ref_name: &str, commit_hash: &str) -> EventBuilder {
    let tags = vec![
        Tag::custom(TagKind::Custom("repo".into()), [repo_id]),
        Tag::custom(TagKind::Custom("ref".into()), [ref_name]),
        Tag::custom(TagKind::Custom("commit".into()), [commit_hash]),
    ];
    EventBuilder::new(Kind::TextNote, format!("push to {ref_name}")).tags(tags)
}

/// Create a hook handler closure that bridges forge events to the Nostr relay.
///
/// The returned closure can be registered with `HookService::register_handler`.
/// Relay publish failures are logged but swallowed — forge operations must not
/// fail because of the Nostr bridge.
pub fn create_bridge_handler<S: aspen_core::KeyValueStore + ?Sized + 'static>(
    relay: Arc<aspen_nostr_relay::NostrRelayService<S>>,
) -> impl Fn(&HookEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>>
+ Send
+ Sync
+ 'static {
    move |event: &HookEvent| {
        let relay = Arc::clone(&relay);
        let event_type = event.event_type;
        let payload_json = event.payload.clone();
        Box::pin(async move {
            if event_type == HookEventType::ForgePushCompleted
                && let Ok(payload) = serde_json::from_value::<ForgePushCompletedPayload>(payload_json)
            {
                let builder = build_push_event(&payload.repo_id, &payload.ref_name, &payload.new_hash);
                match relay.identity().sign_event(builder) {
                    Ok(signed) => match relay.publish(&signed).await {
                        Ok(is_new) => {
                            debug!(
                                repo = %payload.repo_id,
                                ref_name = %payload.ref_name,
                                is_new,
                                "NIP-34 push event published"
                            );
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to publish NIP-34 push event");
                        }
                    },
                    Err(e) => {
                        warn!(error = %e, "failed to sign NIP-34 push event");
                    }
                }
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_announcement_has_correct_kind_and_tags() {
        let builder = build_repo_announcement("abc123", "my-repo", Some("A test repo"));
        let keys = nostr::Keys::generate();
        let event = builder.sign_with_keys(&keys).unwrap();

        assert_eq!(event.kind, nostr::Kind::Custom(30617));
        // Check d-tag is present
        let tags_str = format!("{:?}", event.tags);
        assert!(tags_str.contains("abc123"), "should contain repo ID in tags");
        assert!(tags_str.contains("my-repo"), "should contain repo name in tags");
    }

    #[test]
    fn push_event_has_repo_and_ref_tags() {
        let builder = build_push_event("repo123", "refs/heads/main", "commit456");
        let keys = nostr::Keys::generate();
        let event = builder.sign_with_keys(&keys).unwrap();

        let tags_str = format!("{:?}", event.tags);
        assert!(tags_str.contains("repo123"), "should contain repo ID");
        assert!(tags_str.contains("refs/heads/main"), "should contain ref");
        assert!(tags_str.contains("commit456"), "should contain commit");
    }
}
