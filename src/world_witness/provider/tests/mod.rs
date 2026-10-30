struct Fake {
    snapshot: release_channel_index::model::Snapshot,
    server: ed25519_dalek::SigningKey,
    writer: [u8; 32],
    lose_reserve: bool,
    replay: Option<release_channel_index::model::SignedReply>,
}

impl super::Transport for Fake {
    fn exchange(
        &mut self,
        request: &release_channel_index::model::SignedRequest,
    ) -> Result<release_channel_index::model::SignedReply, String> {
        release_channel_index::crypto::verify(
            &self.writer,
            b"release-channel-index/request/v1\0",
            &request.request,
            &request.signature,
        )?;
        if let Some(reply) = &self.replay {
            return Ok(reply.clone());
        }
        release_channel_index::transition::apply(&mut self.snapshot, request)?;
        if self.lose_reserve
            && matches!(request.request.operation, release_channel_index::model::Operation::Reserve { .. })
        {
            self.lose_reserve = false;
            return Err("reply lost after durable reservation".into());
        }
        let reply = release_channel_index::model::Reply {
            namespace: request.request.namespace.clone(),
            challenge: request.request.challenge,
            request_digest: release_channel_index::crypto::request_digest(&request.request)?,
            snapshot: self.snapshot.clone(),
        };
        let signature = release_channel_index::crypto::sign(&self.server, b"release-channel-index/reply/v1\0", &reply)?;
        Ok(release_channel_index::model::SignedReply { reply, signature })
    }
}

fn fixture() -> super::Index<Fake, crate::world_witness::LivePorts> {
    let writer = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let server = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
    let pin = server.verifying_key().to_bytes();
    let fake = Fake {
        snapshot: release_channel_index::model::Snapshot {
            revision: 1,
            highest: None,
            committed: None,
            pending: None,
        },
        server,
        writer: writer.verifying_key().to_bytes(),
        lose_reserve: false,
        replay: None,
    };
    super::Index::new(
        release_channel_index::model::Namespace {
            channel: "world".into(),
            platform: "test".into(),
            track: "main".into(),
        },
        super::Trust {
            key: pin,
            contract: super::contract::reviewed(&pin),
        },
        writer,
        fake,
        crate::world_witness::LivePorts::new().expect("live ports"),
    )
    .expect("provider")
}

fn claim(generation: u64, predecessor: Option<[u8; 32]>) -> release_channel_index::model::Claim {
    release_channel_index::model::Claim {
        generation,
        digest: [u8::try_from(generation).expect("fixture generation fits one byte"); 32],
        predecessor,
    }
}

#[test]
fn staged_claim_is_not_current_and_whole_consumer_rollback_is_rejected() {
    let mut provider = fixture();
    let first = claim(1, None);
    let reserved = provider.reserve(1, first.clone()).expect("reserve first");
    assert!(provider.observe_current(&first, 1).is_err());
    provider.commit(&reserved).expect("commit first");
    let rolled_back_consumer_claim = first.clone();
    let revision = provider.query(0).expect("query").snapshot.revision;
    let second = claim(2, Some(first.digest));
    let next = provider.reserve(revision, second.clone()).expect("reserve second");
    provider.commit(&next).expect("commit second");
    assert!(provider.observe_current(&rolled_back_consumer_claim, 0).is_err());
    provider.observe_current(&second, 0).expect("actual external highest current");
}

#[test]
fn lost_append_retains_exact_operation_and_fresh_query_recovers_without_duplicate() {
    let mut provider = fixture();
    provider.transport.lose_reserve = true;
    let candidate = claim(1, None);
    let error = provider.reserve(1, candidate.clone()).expect_err("lost response");
    assert_eq!(error.kind, super::ErrorKind::Uncertain);
    let original = error.reservation.expect("reservation retained before dispatch");
    let recovered = provider.recover_reservation(&candidate, 1).expect("fresh observation").expect("recover pending");
    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.revision, 2);
    let committed = provider.commit(&recovered).expect("finish original operation");
    assert_eq!(committed.snapshot.committed, Some(candidate));
    assert_eq!(committed.snapshot.revision, 3);
}

#[test]
fn orphan_resolution_preserves_highwater_and_requires_strict_supersession() {
    let mut provider = fixture();
    let first = claim(1, None);
    let abandoned = provider.reserve(1, first.clone()).expect("reserve");
    provider.orphan(&abandoned).expect("known failed CAS");
    assert!(provider.observe_reserved(&abandoned).is_err());
    let resolved = provider.resolve_orphan(&abandoned).expect("explicit recovery");
    assert_eq!(resolved.snapshot.highest, Some(first.clone()));
    assert_eq!(resolved.snapshot.committed, None);
    assert!(provider.observe_current(&first, 0).is_err());
    assert!(provider.reserve(resolved.snapshot.revision, first).is_err());
    let next = claim(2, None);
    let replacement = provider.reserve(resolved.snapshot.revision, next.clone()).expect("strictly higher claim");
    provider.commit(&replacement).expect("recovery commit");
    provider.observe_current(&next, 0).expect("recovered current");
}

#[test]
fn cached_signed_reply_cannot_satisfy_a_new_query_challenge() {
    let mut provider = fixture();
    let observation = provider.query(0).expect("first fresh query");
    provider.transport.replay = Some(observation.reply);
    assert!(provider.query(0).is_err());
}

#[test]
fn namespace_and_minimum_revision_are_fenced() {
    let mut provider = fixture();
    let mut reserved = provider.reserve(1, claim(1, None)).expect("reserve");
    reserved.namespace.track = "other".into();
    assert!(provider.commit(&reserved).is_err());
    assert!(provider.query(100).is_err());
    provider.provider_key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes();
    assert!(provider.query(0).is_err());
}

#[test]
fn checkpoint_identity_ignores_query_nonce_but_binds_exact_state() {
    let mut provider = fixture();
    let first = provider.query(0).expect("first query");
    let second = provider.query(0).expect("second query");
    let first_id = super::projection::checkpoint_id(&provider.provider_key, &provider.namespace, &first.snapshot)
        .expect("checkpoint identity");
    let second_id = super::projection::checkpoint_id(&provider.provider_key, &provider.namespace, &second.snapshot)
        .expect("repeated identity");
    assert_eq!(first_id, second_id);
    let reserved = provider.reserve(1, claim(1, None)).expect("reserve");
    let pending = provider.observe_reserved(&reserved).expect("pending");
    let mut fork = pending.snapshot.clone();
    fork.committed = fork.highest.clone();
    fork.pending = None;
    fork.validate().expect("alternate valid state at the same revision");
    let pending_id = super::projection::checkpoint_id(&provider.provider_key, &provider.namespace, &pending.snapshot)
        .expect("pending identity");
    let fork_id =
        super::projection::checkpoint_id(&provider.provider_key, &provider.namespace, &fork).expect("fork identity");
    assert_ne!(pending_id, fork_id);
}
