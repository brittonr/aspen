#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    namespace: release_channel_index::model::Namespace,
    provider_key: [u8; 32],
    contract: Contract,
    writer_secret: [u8; 32],
    transport: molten::world_witness::provider::process::Capability,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    provider_id: String,
    contract_id: String,
    source_revision: String,
    license: String,
}

impl Contract {
    fn into_trust(self, key: [u8; 32]) -> molten::world_witness::provider::Trust {
        molten::world_witness::provider::Trust {
            key,
            contract: molten_core::world_witness::ProviderContract {
                provider_id: self.provider_id,
                contract_id: self.contract_id,
                source_revision: self.source_revision,
                license: self.license,
            },
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Reserve {
    expected_revision: u64,
    claim: release_channel_index::model::Claim,
}

fn load<T: serde::de::DeserializeOwned>(path: &std::ffi::OsStr) -> Result<T, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(file, 65_537), &mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 65_536 {
        return Err("pilot configuration exceeds bound".into());
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

/// world-witness-provider-pilot CONFIG query|reserve|commit|orphan|resolve-orphan [INPUT.json]
#[derive(clap::Parser)]
#[command(name = "world-witness-provider-pilot")]
struct Arguments {
    config: std::path::PathBuf,
    operation: String,
    input: Option<std::path::PathBuf>,
}

fn execute(arguments: Arguments) -> Result<serde_json::Value, String> {
    let config: Config = load(arguments.config.as_os_str())?;
    let ports = molten::world_witness::LivePorts::new().map_err(|error| error.to_string())?;
    let mut index = molten::world_witness::provider::Index::new(
        config.namespace,
        config.contract.into_trust(config.provider_key),
        ed25519_dalek::SigningKey::from_bytes(&config.writer_secret),
        config.transport,
        ports,
    )
    .map_err(|error| error.to_string())?;
    let input = || arguments.input.as_deref().map(std::path::Path::as_os_str).ok_or("missing INPUT.json".to_string());
    match arguments.operation.as_str() {
        "query" => encode(index.query(0)),
        "reserve" => {
            let reserve: Reserve = load(input()?)?;
            encode(index.reserve(reserve.expected_revision, reserve.claim))
        }
        operation => {
            let reservation: molten::world_witness::provider::Reservation = load(input()?)?;
            match operation {
                "commit" => encode(index.commit(&reservation)),
                "orphan" => encode(index.orphan(&reservation)),
                "resolve-orphan" => encode(index.resolve_orphan(&reservation)),
                _ => Err("unknown operation".into()),
            }
        }
    }
}

fn encode<T: serde::Serialize>(
    result: Result<T, molten::world_witness::provider::Error>,
) -> Result<serde_json::Value, String> {
    match result {
        Ok(value) => Ok(serde_json::json!({ "outcome": "observed", "evidence": value })),
        Err(error) => {
            // Never translate a possible external mutation into an ordinary no-effect failure.
            Ok(serde_json::json!({ "outcome": format!("{:?}", error.kind),
                "detail": error.detail, "reservation": error.reservation }))
        }
    }
}

fn main() -> std::process::ExitCode {
    let outcome = <Arguments as clap::Parser>::try_parse().map_err(|error| error.to_string()).and_then(execute);
    match outcome {
        Ok(receipt) => {
            println!("{receipt}");
            if receipt["outcome"] == "observed" {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
