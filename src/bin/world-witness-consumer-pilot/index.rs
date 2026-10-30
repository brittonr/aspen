fn namespace() -> release_channel_index::model::Namespace {
    release_channel_index::model::Namespace {
        channel: "private-world-pilot".into(),
        platform: "same-host".into(),
        track: "main".into(),
    }
}

pub fn initialize(path: &std::path::Path) -> super::Result<()> {
    let admin = ed25519_dalek::SigningKey::from_bytes(&[81; 32]);
    let writer = ed25519_dalek::SigningKey::from_bytes(&[83; 32]);
    let server = ed25519_dalek::SigningKey::from_bytes(&[85; 32]);
    let provider =
        release_channel_index::provider::Provider::initialize(path, admin.verifying_key().to_bytes(), server)?;
    let enrollment = release_channel_index::crypto::authenticate(&admin, release_channel_index::model::Request {
        namespace: namespace(),
        challenge: [87; 32],
        operation: release_channel_index::model::Operation::Enroll {
            writer: writer.verifying_key().to_bytes(),
        },
    })?;
    provider.handle(&enrollment)?;
    Ok(())
}

pub fn open(
    root: &std::path::Path,
) -> super::Result<
    molten::world_witness::provider::Index<
        molten::world_witness::provider::process::Capability,
        molten::world_witness::LivePorts,
    >,
> {
    let writer = ed25519_dalek::SigningKey::from_bytes(&[83; 32]);
    let pin = ed25519_dalek::SigningKey::from_bytes(&[85; 32]).verifying_key().to_bytes();
    let contract = policy(std::fs::read_to_string(root.join("provider-revision"))?)?.provider;
    let transport = molten::world_witness::provider::process::Capability {
        program: std::env::current_exe()?,
        args: vec![
            "index-serve".into(),
            root.to_str().ok_or("pilot root must be UTF-8")?.into(),
        ],
        timeout_ms: 10_000,
    };
    molten::world_witness::provider::Index::new(
        namespace(),
        molten::world_witness::provider::Trust { key: pin, contract },
        writer,
        transport,
        molten::world_witness::LivePorts::new()?,
    )
    .map_err(|error| error.to_string().into())
}

pub fn policy(revision: String) -> super::Result<molten_core::world_witness::WitnessPolicy> {
    let contract = molten::world_witness::provider::contract::reviewed(
        &ed25519_dalek::SigningKey::from_bytes(&[85; 32]).verifying_key().to_bytes(),
    );
    if revision != contract.source_revision {
        return Err("operator provider source pin differs from compiled reviewed contract".into());
    }
    let provider_id = contract.provider_id.clone();
    Ok(molten_core::world_witness::WitnessPolicy {
        required_profile: molten_core::world_witness::AssuranceProfile::IndependentWitnessV1,
        provider: contract,
        allowed_witnesses: std::collections::BTreeSet::from([provider_id]),
        minimum_quorum: 1,
        maximum_age: 60,
    })
}

/// This command runs only in the separate provider capability process.
pub fn serve(root: &std::path::Path) -> super::Result<()> {
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(std::io::stdin(), (release_channel_index::model::MAX_FRAME + 1) as u64),
        &mut bytes,
    )?;
    if bytes.len() > release_channel_index::model::MAX_FRAME {
        return Err("provider request exceeds frame bound".into());
    }
    let request: release_channel_index::model::SignedRequest = serde_json::from_slice(&bytes)?;
    let admin = ed25519_dalek::SigningKey::from_bytes(&[81; 32]);
    let server = ed25519_dalek::SigningKey::from_bytes(&[85; 32]);
    let service =
        release_channel_index::provider::Provider::open(&root.join("index"), admin.verifying_key().to_bytes(), server)?;
    let reply = service.handle(&request)?;
    serde_json::to_writer(std::io::stdout(), &reply)?;
    Ok(())
}

pub fn provision(root: &std::path::Path) -> super::Result<()> {
    let status = std::process::Command::new(std::env::current_exe()?).arg("index-initialize").arg(root).status()?;
    if !status.success() {
        return Err("provider initialization process failed".into());
    }
    Ok(())
}
