use super::*;

impl Sealed<'_> {
    pub(super) fn require_tool_identity(&self) -> Result<(), Tpm2IdentityFailure> {
        let bytes = self
            .executable
            .read_bounded(crate::node_state::MAX_EXECUTABLE_BYTES)
            .map_err(|_| Tpm2IdentityFailure::CapabilityUnavailable)?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != self.policy.executable_ref {
            return Err(Tpm2IdentityFailure::ToolIdentityMismatch);
        }
        Ok(())
    }

    pub(super) fn run_tool(&self, mut args: Vec<String>, maximum: usize) -> Result<Vec<u8>, Tpm2IdentityFailure> {
        self.require_tool_identity()?;
        let workspace = self.workspace.process_path().map_err(|_| Tpm2IdentityFailure::CapabilityUnavailable)?;
        let tcti = match self.policy.provider_class {
            Tpm2ProviderClass::LinuxResourceManager => "device:/dev/tpmrm0".to_string(),
            Tpm2ProviderClass::SwtpmWorkspace => format!("swtpm:path={}/swtpm.sock", workspace.display()),
        };
        args.extend(["-T".to_string(), tcti]);
        run(self.executable, &self.workspace, args, bounded_exec::Input::Null, maximum)
    }
}

/// Secret output is never handed to the execution fabric's public stream publisher.
pub(super) fn run(
    executable: &crate::node_state::Executable,
    workspace: &crate::node_state::Workspace,
    arguments: Vec<String>,
    input: bounded_exec::Input,
    maximum: usize,
) -> Result<Vec<u8>, Tpm2IdentityFailure> {
    let policy =
        bounded_exec::OutcomePolicy::new(vec![0], true, true).map_err(|_| Tpm2IdentityFailure::InvalidPolicy)?;
    let request = bounded_exec::RunRequest {
        command: bounded_exec::CommandSpec {
            program: executable.process_path().map_err(|_| Tpm2IdentityFailure::CapabilityUnavailable)?,
            args: arguments.into_iter().map(std::ffi::OsString::from).collect(),
            current_dir: workspace.process_path().map_err(|_| Tpm2IdentityFailure::CapabilityUnavailable)?,
            environment_mode: bounded_exec::EnvironmentMode::Clear,
            environment: vec![("TSS2_LOG".into(), "all+NONE".into())],
            input,
        },
        limits: bounded_exec::ExecutionLimits {
            timeout_ms: 5_000,
            stdin_max_bytes: 128,
            stdout_max_bytes: maximum,
            stderr_max_bytes: 4_096,
            poll_interval_ms: 10,
            teardown_timeout_ms: 1_000,
        },
        termination_scope: bounded_exec::TerminationScope::ProcessGroup,
        outcome_policy: policy,
    };
    let mut output = bounded_exec::run(request).map_err(|_| Tpm2IdentityFailure::ProcessUnavailable)?;
    // Never render raw errors, stream bytes, arguments, key material or TPM locators.
    output.stderr.bytes.fill(0);
    match output.disposition {
        bounded_exec::Disposition::Succeeded => Ok(output.stdout.bytes),
        bounded_exec::Disposition::TimedOut | bounded_exec::Disposition::Cancelled => {
            output.stdout.bytes.fill(0);
            Err(Tpm2IdentityFailure::TimedOut)
        }
        bounded_exec::Disposition::OutputLimitExceeded(_) => {
            output.stdout.bytes.fill(0);
            Err(Tpm2IdentityFailure::OutputLimit)
        }
        bounded_exec::Disposition::ExitFailed => {
            output.stdout.bytes.fill(0);
            // tpm2-tools 5.7 distinguishes TCTI initialization (4) from TPM
            // command/auth failures (1/3). POLICY_FAIL itself is flattened to 1;
            // UnsealDenied does not assert a more specific hardware diagnosis.
            const TCTI_UNAVAILABLE_EXIT: i32 = 4;
            if output.exit_code == Some(TCTI_UNAVAILABLE_EXIT) || output.exit_code.is_none() {
                Err(Tpm2IdentityFailure::ProcessUnavailable)
            } else {
                Err(Tpm2IdentityFailure::UnsealDenied)
            }
        }
    }
}
