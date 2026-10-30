use std::io::Read;
use std::io::Write;

fn main() {
    if run().is_err() {
        eprintln!("collaboration-scope/native-callback-denied");
        std::process::exit(1);
    }
}

fn run() -> molten::error::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(molten::collaboration_scope::MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| molten::error::Failure::invalid_harness("collaboration native input unavailable"))?;
    if bytes.len() as u64 > molten::collaboration_scope::MAX_RECORD_BYTES {
        return Err(molten::error::Failure::invalid_harness("collaboration native input bound"));
    }
    let envelope = molten::system_extension::decode_callback_envelope(
        &bytes,
        molten::collaboration_scope::MAX_RECORD_BYTES,
        molten::collaboration_scope::MAX_RECORD_BYTES,
        128,
    )?;
    let outcome = molten::collaboration_scope::handle_callback(&envelope)?;
    let encoded = molten::system_extension::encode_callback_outcome(&outcome)?;
    std::io::stdout()
        .write_all(&encoded)
        .map_err(|_| molten::error::Failure::invalid_harness("collaboration native output unavailable"))?;
    std::io::stdout()
        .flush()
        .map_err(|_| molten::error::Failure::invalid_harness("collaboration native output unavailable"))
}
