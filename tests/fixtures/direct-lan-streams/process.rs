struct OwnedChild {
    child: std::process::Child,
    log: std::path::PathBuf,
}

impl OwnedChild {
    fn spawn(config: &std::path::Path, log: std::path::PathBuf, cwd: &std::path::Path) -> TestResult<Self> {
        let output = std::fs::File::create(&log)?;
        let child = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", CHILD_ROLE, "--ignored", "--nocapture", "--test-threads=1"])
            .env_clear()
            .env(CONFIG_ENV, config)
            .current_dir(cwd)
            .stdin(std::process::Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .spawn()?;
        Ok(Self { child, log })
    }

    fn await_file(&mut self, artifact: &std::path::Path) -> TestResult<Vec<u8>> {
        let mut deadline = molten::fabric_time::SupervisionDeadline::after(CHILD_BUDGET)?;
        loop {
            match std::fs::metadata(artifact) {
                Ok(metadata) => {
                    if !metadata.is_file() || metadata.len() > MAX_ARTIFACT_BYTES {
                        return Err("invalid role artifact type or bound".into());
                    }
                    return Ok(std::fs::read(artifact)?);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            if let Some(status) = self.child.try_wait()? {
                return Err(format!("role exited before artifact: {status}; {}", self.diagnostic()?).into());
            }
            if deadline.is_expired()? {
                return Err(format!("role artifact deadline exceeded: {}", self.diagnostic()?).into());
            }
            std::thread::sleep(POLL);
        }
    }

    fn finish(&mut self) -> TestResult {
        let mut deadline = molten::fabric_time::SupervisionDeadline::after(CHILD_BUDGET)?;
        loop {
            if let Some(status) = self.child.try_wait()? {
                if !status.success() {
                    return Err(format!("role failed: {status}; {}", self.diagnostic()?).into());
                }
                return Ok(());
            }
            if deadline.is_expired()? {
                return Err(format!("role completion deadline exceeded: {}", self.diagnostic()?).into());
            }
            std::thread::sleep(POLL);
        }
    }

    fn diagnostic(&self) -> TestResult<String> {
        let file = std::fs::File::open(&self.log)?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(file, MAX_ARTIFACT_BYTES), &mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn kill(&mut self) -> TestResult {
        if self.child.try_wait()?.is_some() {
            return Err("role already exited before injected process loss".into());
        }
        self.child.kill()?;
        let status = self.child.wait()?;
        if status.success() {
            return Err("injected process loss reported success".into());
        }
        Ok(())
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _terminated = self.child.kill();
        let _reaped = self.child.wait();
    }
}

fn private_directory(path: &std::path::Path) -> TestResult {
    std::fs::create_dir(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn protected_socket(path: &std::path::Path) -> TestResult<tokio::net::UnixListener> {
    let listener = tokio::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    if std::fs::metadata(path)?.permissions().mode() & 0o777 != 0o600 {
        return Err("protected consumer socket mode mismatch".into());
    }
    Ok(listener)
}

fn restart_listener(
    workspace: &std::path::Path,
    handoff_path: &std::path::Path,
    config: &mut config::RoleConfig,
    listener: &mut OwnedChild,
) -> TestResult {
    let old_handoff = read_endpoint(handoff_path)?;
    let ip = old_handoff.descriptor.locators.iter().find_map(|locator| {
        locator.value.strip_prefix("ip:").and_then(|value| value.parse::<std::net::SocketAddr>().ok())
    }).ok_or("direct handoff has no admitted IP locator")?;
    listener.kill()?;
    std::fs::rename(handoff_path, workspace.join("old-handoff.preserves"))?;
    config.generation = 2;
    config.bind_port = ip.port();
    *listener = OwnedChild::spawn(&write_config(config)?, workspace.join("restarted-listener.log"), workspace)?;
    listener.await_file(handoff_path)?;
    let replacement = read_endpoint(handoff_path)?;
    assert_eq!(old_handoff.descriptor.public_endpoint_identity, replacement.descriptor.public_endpoint_identity);
    assert_ne!(old_handoff.descriptor_ref, replacement.descriptor_ref);
    assert_eq!(replacement.descriptor.generation, 2);
    Ok(())
}
