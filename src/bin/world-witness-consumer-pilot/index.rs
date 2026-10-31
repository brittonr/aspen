const ADMIN_SEED: [u8; 32] = [81; 32];
const WRITER_SEED: [u8; 32] = [83; 32];
const SERVER_SEED: [u8; 32] = [85; 32];
/// Absolute provider executable selected at provisioning; absent selects the built-in child.
const PROGRAM_FILE: &str = "provider-program";
const CONFIG_FILE: &str = "provider-config.json";
const TIMEOUT_MS: u64 = 10_000;

fn namespace() -> release_channel_index::model::Namespace {
    release_channel_index::model::Namespace {
        channel: "private-world-pilot".into(),
        platform: "same-host".into(),
        track: "main".into(),
    }
}

fn admin() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&ADMIN_SEED)
}

fn pin() -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(&SERVER_SEED).verifying_key().to_bytes()
}

/// Built-in provider initialization child; enrollment always crosses the selected transport.
pub fn initialize(path: &std::path::Path) -> super::Result<()> {
    release_channel_index::provider::Provider::initialize(
        path,
        admin().verifying_key().to_bytes(),
        ed25519_dalek::SigningKey::from_bytes(&SERVER_SEED),
    )?;
    Ok(())
}

fn utf8(path: &std::path::Path) -> super::Result<String> {
    Ok(path.to_str().ok_or("pilot path must be UTF-8")?.to_owned())
}

fn capability(root: &std::path::Path) -> super::Result<molten::world_witness::provider::process::Capability> {
    let selected = root.join(PROGRAM_FILE);
    if !selected.exists() {
        return Ok(molten::world_witness::provider::process::Capability {
            program: std::env::current_exe()?,
            args: vec!["index-serve".into(), utf8(root)?],
            timeout_ms: TIMEOUT_MS,
        });
    }
    Ok(molten::world_witness::provider::process::Capability {
        program: std::fs::read_to_string(selected)?.into(),
        args: vec![
            "serve".into(),
            utf8(&root.join("index"))?,
            utf8(&root.join(CONFIG_FILE))?,
        ],
        timeout_ms: TIMEOUT_MS,
    })
}

pub fn open(
    root: &std::path::Path,
) -> super::Result<
    molten::world_witness::provider::Index<
        molten::world_witness::provider::process::Capability,
        molten::world_witness::LivePorts,
    >,
> {
    let contract = policy(std::fs::read_to_string(root.join("provider-revision"))?)?.provider;
    molten::world_witness::provider::Index::new(
        namespace(),
        molten::world_witness::provider::Trust { key: pin(), contract },
        ed25519_dalek::SigningKey::from_bytes(&WRITER_SEED),
        capability(root)?,
        molten::world_witness::LivePorts::new()?,
    )
    .map_err(|error| error.to_string().into())
}

pub fn policy(revision: String) -> super::Result<molten_core::world_witness::WitnessPolicy> {
    let contract = molten::world_witness::provider::contract::reviewed(&pin());
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

/// This command runs only in the separate built-in provider capability process.
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
    let service = release_channel_index::provider::Provider::open(
        &root.join("index"),
        admin().verifying_key().to_bytes(),
        ed25519_dalek::SigningKey::from_bytes(&SERVER_SEED),
    )?;
    let reply = service.handle(&request)?;
    serde_json::to_writer(std::io::stdout(), &reply)?;
    Ok(())
}

/// Initializes the selected provider, then enrolls the writer through the actual transport.
pub fn provision(root: &std::path::Path, program: Option<&std::path::Path>) -> super::Result<()> {
    let status = match program {
        Some(program) => {
            let config =
                serde_json::json!({ "admin": admin().verifying_key().to_bytes(), "server_secret": SERVER_SEED });
            let mut options = std::fs::OpenOptions::new();
            std::os::unix::fs::OpenOptionsExt::mode(options.write(true).create_new(true), 0o600);
            std::io::Write::write_all(&mut options.open(root.join(CONFIG_FILE))?, &serde_json::to_vec(&config)?)?;
            std::fs::write(root.join(PROGRAM_FILE), utf8(&std::path::absolute(program)?)?)?;
            std::process::Command::new(program)
                .arg("init")
                .arg(root.join("index"))
                .arg(root.join(CONFIG_FILE))
                .status()?
        }
        None => std::process::Command::new(std::env::current_exe()?).arg("index-initialize").arg(root).status()?,
    };
    if !status.success() {
        return Err("provider initialization process failed".into());
    }
    let request = release_channel_index::crypto::authenticate(&admin(), release_channel_index::model::Request {
        namespace: namespace(),
        challenge: [87; 32],
        operation: release_channel_index::model::Operation::Enroll {
            writer: ed25519_dalek::SigningKey::from_bytes(&WRITER_SEED).verifying_key().to_bytes(),
        },
    })?;
    let mut transport = capability(root)?;
    let reply = molten::world_witness::provider::Transport::exchange(&mut transport, &request)?;
    release_channel_index::client::verify_reply(&pin(), &request.request, &reply, 0)?;
    Ok(())
}
