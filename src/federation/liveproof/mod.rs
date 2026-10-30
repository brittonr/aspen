mod campaign;
mod provider;
mod receipt;
mod receiver;
mod setup;
mod terminal;

pub struct RunInput {
    pub run_directory: std::path::PathBuf,
    /// Independently reviewed current DAG conformance artifact; never synthesized by this fixture.
    pub dag_conformance_ref: String,
}

/// Runs a named malicious-wire case and requires the exact real rejection before registry mutation.
pub fn loopback_denial(input: &RunInput, fault: &str) -> crate::error::Result<preserves::IOValue> {
    campaign::loopback(input, fault)
}
pub fn distinct_process_denial(
    input: &RunInput,
    process_binary: &std::path::Path,
    fault: &str,
) -> crate::error::Result<preserves::IOValue> {
    campaign::distinct_process(input, process_binary, fault)
}

/// CLI bootstrap only: the selected directory must be fresh and owned by this run.
pub fn loopback(input: &RunInput) -> crate::error::Result<preserves::IOValue> {
    let run_directory = &input.run_directory;
    let exchange_root = crate::node_state::DirectoryRoot::open(run_directory)?;
    let exchange = exchange_root.receipts()?;
    require_fresh(&exchange)?;
    configure(&exchange, input)?;
    let provider_root = crate::node_state::DirectoryRoot::open(&run_directory.join("provider"))?;
    let receiver_root = crate::node_state::DirectoryRoot::open(&run_directory.join("receiver"))?;
    let provider_exchange = exchange_root.receipts()?;
    let child = std::thread::spawn(move || provider::run(&provider_root, &provider_exchange));
    let received = receiver::run(&receiver_root, &exchange);
    let provided = child.join().map_err(|_| invalid("live provider thread panicked"))?;
    provided?;
    let verified = received?;
    let readback = receipt::offline::readback(&receiver_root)?;
    if readback.bundle_ref != verified.bundle_ref {
        return Err(invalid("live bundle drifted after receiver readback"));
    }
    Ok(receipt::offline::public_result(&verified, "live-loopback"))
}

/// CLI role: `federation fixture-role --role ROLE --state-root ROOT --exchange-root ROOT`.
/// Both roles run the real production crypto/network/content/import path above.
pub fn role(
    role: &str,
    state_directory: &std::path::Path,
    exchange_directory: &std::path::Path,
) -> crate::error::Result<()> {
    let root = crate::node_state::DirectoryRoot::open(state_directory)?;
    let exchange = crate::node_state::DirectoryRoot::open_existing(exchange_directory)?.receipts()?;
    match role {
        "provider" => provider::run(&root, &exchange),
        "receiver" => receiver::run(&root, &exchange).map(|_| ()),
        _ => Err(invalid("federation fixture role must be provider or receiver")),
    }
}

/// Spawns the actual CLI binary twice; neither role is a test-process behavioral model.
pub fn distinct_process(
    input: &RunInput,
    process_binary: &std::path::Path,
) -> crate::error::Result<preserves::IOValue> {
    let run_directory = &input.run_directory;
    let exchange = crate::node_state::DirectoryRoot::open(run_directory)?.receipts()?;
    require_fresh(&exchange)?;
    configure(&exchange, input)?;
    let mut provider = spawn(process_binary, run_directory, "provider")?;
    let mut receiver = spawn(process_binary, run_directory, "receiver")?;
    if provider.child.id() == receiver.child.id()
        || provider.child.id() == std::process::id()
        || receiver.child.id() == std::process::id()
    {
        return Err(invalid("fixture roles did not acquire distinct OS process identities"));
    }
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(setup::transport::TIMEOUT * 3)?;
    let provider_status = provider.wait(&mut deadline)?;
    let receiver_status = receiver.wait(&mut deadline)?;
    if !provider_status.success() || !receiver_status.success() {
        return Err(invalid("actual federation role process failed; retained roots contain scoped evidence"));
    }
    if setup::exchange::handoff(&exchange, "provider")?.process_id != u64::from(provider.child.id())
        || setup::exchange::handoff(&exchange, "receiver")?.process_id != u64::from(receiver.child.id())
    {
        return Err(invalid("published role identities differ from the actual spawned OS processes"));
    }
    let root = crate::node_state::DirectoryRoot::open_existing(&run_directory.join("receiver"))?;
    let verified = receipt::offline::readback(&root)?;
    let value = crate::preserves_rail::record("federation-distinct-process-proof-v1", vec![
        receipt::offline::public_result(&verified, "distinct-os-process"),
        crate::preserves_rail::u64_value(u64::from(provider.child.id())),
        crate::preserves_rail::u64_value(u64::from(receiver.child.id())),
        crate::preserves_rail::u64_value(u64::from(std::process::id())),
    ]);
    setup::dagports::persist(&exchange, "distinct-process-proof.preserves", &value)?;
    Ok(value)
}

/// Offline verification requires an independently selected context hash; no networking occurs.
pub fn verify_detached(
    receiver_directory: &std::path::Path,
    trusted_context_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    let root = crate::node_state::DirectoryRoot::open_existing(receiver_directory)?;
    Ok(receipt::offline::public_result(
        &receipt::offline::verify(&root, trusted_context_ref)?,
        "offline-detached-verification",
    ))
}
fn require_fresh(scope: &crate::node_state::Scope) -> crate::error::Result<()> {
    for leaf in [
        "provider-handoff.preserves",
        "receiver-handoff.preserves",
        "receiver-session.preserves",
        "fixture-fault.preserves",
        "dag-conformance.preserves",
    ] {
        if scope.try_exists(&crate::node_state::RelativePath::parse(leaf)?)? {
            return Err(invalid("federation fixture refuses stale run handoffs; choose a fresh owned root"));
        }
    }
    Ok(())
}
fn configure(scope: &crate::node_state::Scope, input: &RunInput) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(&input.dag_conformance_ref)?;
    setup::dagports::persist(
        scope,
        "dag-conformance.preserves",
        &crate::preserves_rail::string(&input.dag_conformance_ref),
    )
    .map(|_| ())
}
fn spawn(binary: &std::path::Path, root: &std::path::Path, role: &str) -> crate::error::Result<Child> {
    let child = std::process::Command::new(binary)
        .args(["federation", "fixture-role", "--role", role])
        .arg("--state-root")
        .arg(root.join(role))
        .arg("--exchange-root")
        .arg(root)
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(crate::error::Failure::from)?;
    Ok(Child { child, reaped: false })
}
struct Child {
    child: std::process::Child,
    reaped: bool,
}
impl Child {
    fn wait(
        &mut self,
        deadline: &mut crate::fabric_time::SupervisionDeadline,
    ) -> crate::error::Result<std::process::ExitStatus> {
        loop {
            if let Some(status) = self.child.try_wait().map_err(crate::error::Failure::from)? {
                self.reaped = true;
                return Ok(status);
            }
            if deadline.is_expired()? {
                return Err(invalid("actual federation role process timed out"));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn invalid(message: &str) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}

#[cfg(test)]
mod bindingtests;
#[cfg(test)]
mod tests;
