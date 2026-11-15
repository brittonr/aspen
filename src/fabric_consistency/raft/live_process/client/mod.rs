use super::*;

mod executor;
mod history;
mod tests;

const NATIVE_RECOVERY_TICKS: u64 = 16;

fn run(
    input: &crate::fabric_simulation::bounded_client::Input,
    directory: &std::path::Path,
) -> crate::error::Result<crate::fabric_simulation::bounded_client::History> {
    input.validate()?;
    write_value(
        &directory.join(crate::fabric_simulation::bounded_client::CLIENT_INPUT_FILE),
        &crate::fabric_simulation::bounded_client::input_value(input),
    )?;
    write_value(
        &directory.join("client-control-deployed.preserves"),
        &crate::preserves_rail::record("molten-client-control-deployed-v1", vec![
            crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(input)?),
            crate::preserves_rail::string(input.control.as_str()),
            crate::preserves_rail::string(crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY),
        ]),
    )?;
    let executable = running_executable()?;
    let mut binary = std::fs::File::open(&executable).map_err(crate::error::Failure::from)?;
    let mut binary_hasher = blake3::Hasher::new();
    binary_hasher.update_reader(&mut binary).map_err(crate::error::Failure::from)?;
    let binary_ref = format!("blake3:{}", binary_hasher.finalize().to_hex());
    let workload = input.differential_workload()?;
    let mut executor = executor::Driver::new(input, directory, &executable);
    let outcomes = crate::fabric_simulation::execute_workload(&mut executor, &workload);
    let outcome_value = crate::preserves_rail::sequence(
        outcomes
            .iter()
            .map(|outcome| {
                crate::preserves_rail::string(
                    outcome.map_or("unavailable", crate::fabric_simulation::OutcomeClass::as_str),
                )
            })
            .collect(),
    );
    write_value(
        &directory.join("client-execution.preserves"),
        &crate::preserves_rail::record("molten-client-native-execution-v1", vec![
            crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(input)?),
            outcome_value,
            crate::preserves_rail::string(executor.failure.as_deref().unwrap_or("")),
        ]),
    )?;
    persist_build_bindings(directory, input, &binary_ref)?;
    let executed = std::mem::take(&mut executor.executed);
    // A valid prefix may leave producers running. Reap those owned children
    // before reading their evidence, without executing omitted later phases.
    drop(executor);
    let result = history::collect(input, directory, executed);
    if result.is_err() {
        retain(directory, &format!("{}-{}-execution-error", input.control.as_str(), input.steps.len()))?;
    }
    result
}

fn persist_build_bindings(
    directory: &std::path::Path,
    input: &crate::fabric_simulation::bounded_client::Input,
    binary_ref: &str,
) -> crate::error::Result<()> {
    // Missing source identity is retained, not manufactured from a test fixture.
    // This record is native execution context, not the existing qualification format.
    write_value(
        &directory.join("client-build-bindings.preserves"),
        &crate::preserves_rail::record("molten-client-native-build-v1", vec![
            crate::preserves_rail::string(
                std::env::var("MOLTEN_CHAOSCONTROL_SOURCE_REVISION").unwrap_or_else(|_| "unavailable".to_string()),
            ),
            crate::preserves_rail::string(
                std::env::var("MOLTEN_CHAOSCONTROL_SOURCE_REF").unwrap_or_else(|_| "unavailable".to_string()),
            ),
            crate::preserves_rail::string(binary_ref),
            crate::preserves_rail::string(source_ref(include_str!("executor.rs"))?),
            crate::preserves_rail::string(source_ref(include_str!(
                "../../../../fabric_simulation/bounded_client/oracle.rs"
            ))?),
            crate::preserves_rail::string(source_ref(include_str!(
                "../../../../fabric_simulation/bounded_client/protocol/input.rs"
            ))?),
            crate::preserves_rail::string(source_ref(include_str!(
                "../../../../fabric_simulation/bounded_client/controls.rs"
            ))?),
            crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(input)?),
            crate::preserves_rail::string("native-process-crash-only; no guest or replay qualification"),
        ]),
    )
}

fn running_executable() -> crate::error::Result<std::path::PathBuf> {
    // current_exe() names a path Cargo may replace/unlink during a long run.
    // Repeats must hash and execute this image, not replacement bytes or an
    // absent pathname. exec resolves self before replacing the child's image.
    #[cfg(target_os = "linux")]
    {
        Ok(std::path::PathBuf::from("/proc/self/exe"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::current_exe().map_err(crate::error::Failure::from)
    }
}

fn source_ref(source: &str) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten-client-source-v1", vec![
        crate::preserves_rail::string(source),
    ]))
}

fn persist_check(
    directory: &std::path::Path,
    history: &crate::fabric_simulation::bounded_client::History,
    check: &crate::fabric_simulation::bounded_client::Check,
) -> crate::error::Result<()> {
    let (verdict, property, participant) = match &check.verdict {
        crate::fabric_simulation::bounded_client::Verdict::Pass => ("pass", "", ""),
        crate::fabric_simulation::bounded_client::Verdict::Fail { property, participant } => {
            ("fail", property.as_str(), participant.as_str())
        }
        crate::fabric_simulation::bounded_client::Verdict::NotEvaluated { reason } => {
            ("not-evaluated", reason.as_str(), "")
        }
        crate::fabric_simulation::bounded_client::Verdict::Incomplete { reason } => ("incomplete", reason.as_str(), ""),
        crate::fabric_simulation::bounded_client::Verdict::Invalid { reason } => ("invalid", reason.as_str(), ""),
        crate::fabric_simulation::bounded_client::Verdict::Unsupported => ("unsupported", "", ""),
    };
    write_value(
        &directory.join("client-oracle.preserves"),
        &crate::preserves_rail::record("molten-client-native-check-v1", vec![
            crate::preserves_rail::string(&history.input_ref),
            crate::preserves_rail::string(verdict),
            crate::preserves_rail::string(property),
            crate::preserves_rail::string(participant),
            crate::preserves_rail::string(history.deployed_control.as_str()),
            crate::preserves_rail::bool_value(history.is_control_reached),
            crate::preserves_rail::string(crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY),
            crate::preserves_rail::sequence(
                check
                    .liveness
                    .iter()
                    .map(|result| {
                        let (verdict, condition) = match result.verdict {
                            molten_core::protocol_oracle::LivenessVerdict::Pass => ("pass", ""),
                            molten_core::protocol_oracle::LivenessVerdict::Fail => ("fail", ""),
                            molten_core::protocol_oracle::LivenessVerdict::NotEvaluated(condition) => {
                                ("not-evaluated", condition.as_str())
                            }
                            molten_core::protocol_oracle::LivenessVerdict::Incomplete(condition) => {
                                ("incomplete", condition.as_str())
                            }
                        };
                        crate::preserves_rail::record("client-participant-liveness-v1", vec![
                            crate::preserves_rail::string(&result.participant),
                            crate::preserves_rail::string(verdict),
                            crate::preserves_rail::string(condition),
                        ])
                    })
                    .collect(),
            ),
        ]),
    )
}

/// Optional user-selected output copies only this run's bounded files. A
/// no-replace directory keeps independent original-build repeats separate.
fn retain(directory: &std::path::Path, label: &str) -> crate::error::Result<()> {
    let Some(root) = std::env::var_os("MOLTEN_CHAOSCONTROL_RETAIN") else {
        return Ok(());
    };
    let output = std::path::PathBuf::from(root).join(label);
    std::fs::create_dir_all(
        output
            .parent()
            .ok_or_else(|| crate::error::Failure::invalid_harness("retained output parent missing"))?,
    )
    .map_err(crate::error::Failure::from)?;
    std::fs::create_dir(&output).map_err(crate::error::Failure::from)?;
    for entry in std::fs::read_dir(directory).map_err(crate::error::Failure::from)? {
        let entry = entry.map_err(crate::error::Failure::from)?;
        if entry.file_type().map_err(crate::error::Failure::from)?.is_file() {
            std::fs::copy(entry.path(), output.join(entry.file_name())).map_err(crate::error::Failure::from)?;
        }
    }
    Ok(())
}
