pub(super) struct DurableTransport(pub release_channel_index::provider::Provider);

impl crate::world_witness::provider::Transport for DurableTransport {
    fn exchange(
        &mut self,
        request: &release_channel_index::model::SignedRequest,
    ) -> Result<release_channel_index::model::SignedReply, String> {
        self.0.handle(request)
    }
}

fn namespace() -> release_channel_index::model::Namespace {
    release_channel_index::model::Namespace {
        channel: "world".into(),
        platform: "local-test".into(),
        track: "main".into(),
    }
}

pub(super) fn initialize(
    path: &std::path::Path,
) -> crate::world_witness::provider::Index<DurableTransport, crate::world_witness::LivePorts> {
    let admin = ed25519_dalek::SigningKey::from_bytes(&[10; 32]);
    let writer = ed25519_dalek::SigningKey::from_bytes(&[11; 32]);
    let server = ed25519_dalek::SigningKey::from_bytes(&[12; 32]);
    let pin = server.verifying_key().to_bytes();
    let provider =
        release_channel_index::provider::Provider::initialize(path, admin.verifying_key().to_bytes(), server).unwrap();
    let enrollment = release_channel_index::crypto::authenticate(&admin, release_channel_index::model::Request {
        namespace: namespace(),
        challenge: [13; 32],
        operation: release_channel_index::model::Operation::Enroll {
            writer: writer.verifying_key().to_bytes(),
        },
    })
    .unwrap();
    provider.handle(&enrollment).unwrap();
    crate::world_witness::provider::Index::new(
        namespace(),
        crate::world_witness::provider::Trust {
            key: pin,
            contract: crate::world_witness::provider::contract::reviewed(&pin),
        },
        writer,
        DurableTransport(provider),
        crate::world_witness::LivePorts::new().unwrap(),
    )
    .unwrap()
}

pub(super) fn reopen(
    path: &std::path::Path,
) -> crate::world_witness::provider::Index<DurableTransport, crate::world_witness::LivePorts> {
    let admin = ed25519_dalek::SigningKey::from_bytes(&[10; 32]);
    let writer = ed25519_dalek::SigningKey::from_bytes(&[11; 32]);
    let server = ed25519_dalek::SigningKey::from_bytes(&[12; 32]);
    let pin = server.verifying_key().to_bytes();
    let provider =
        release_channel_index::provider::Provider::open(path, admin.verifying_key().to_bytes(), server).unwrap();
    crate::world_witness::provider::Index::new(
        namespace(),
        crate::world_witness::provider::Trust {
            key: pin,
            contract: crate::world_witness::provider::contract::reviewed(&pin),
        },
        writer,
        DurableTransport(provider),
        crate::world_witness::LivePorts::new().unwrap(),
    )
    .unwrap()
}

pub(super) fn claim(generation: u64, predecessor: Option<[u8; 32]>) -> release_channel_index::model::Claim {
    release_channel_index::model::Claim {
        generation,
        digest: [u8::try_from(generation).expect("fixture generation fits one byte"); 32],
        predecessor,
    }
}

pub(super) fn commit(
    provider: &mut crate::world_witness::provider::Index<DurableTransport, crate::world_witness::LivePorts>,
    claim: release_channel_index::model::Claim,
) -> u64 {
    let revision = provider.query(0).unwrap().snapshot.revision;
    let reservation = provider.reserve(revision, claim).unwrap();
    provider.commit(&reservation).unwrap().snapshot.revision
}
