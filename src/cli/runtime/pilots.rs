#[cfg(feature = "spacewasm-pilot")]
#[path = "pilots/spacewasm.rs"]
pub(crate) mod spacewasm;
#[cfg(feature = "component-rpc")]
#[path = "pilots/componentrpc.rs"]
pub(crate) mod componentrpc;

type Outcome<T> = molten::error::Result<T>;

fn directory(path: &std::path::Path) -> Outcome<cap_std::fs::Dir> {
    cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority()).map_err(molten::error::Failure::from)
}

fn failure(error: String) -> molten::error::Failure {
    molten::error::Failure::invalid_harness(error)
}

fn emit(value: &preserves::IOValue) -> Outcome<()> {
    println!("{}", molten::preserves_rail::to_text(value)?);
    Ok(())
}

#[cfg(feature = "component-rpc")]
fn run_async<T>(future: impl std::future::Future<Output = Result<T, String>>) -> Outcome<T> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
        .map_err(molten::error::Failure::from)?;
    runtime.block_on(future).map_err(failure)
}
