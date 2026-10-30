/// Execute only a previously pinned immutable verifier; callers own authority and argv.
#[cfg(target_os = "linux")]
pub(super) fn run(mut command: std::process::Command) -> Result<(), String> {
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = Child {
        process: command.spawn().map_err(|error| format!("verifier launch: {error}"))?,
        active: true,
    };
    let started = std::time::Instant::now();
    loop {
        match child.process.try_wait().map_err(|error| format!("verifier wait: {error}"))? {
            Some(status) => {
                child.active = false;
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("external verifier denied: {status}"))
                };
            }
            None if started.elapsed() >= std::time::Duration::from_secs(35) => {
                return Err("external verifier exceeded 35-second process bound".into());
            }
            None => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) fn run(_: std::process::Command) -> Result<(), String> {
    Err("capability verifier process groups require Linux".into())
}

#[cfg(target_os = "linux")]
struct Child {
    process: std::process::Child,
    active: bool,
}

#[cfg(target_os = "linux")]
impl Drop for Child {
    fn drop(&mut self) {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        // SAFETY: spawn created a dedicated process group whose positive pid is
        // retained by our unreaped Child. Linux SIGKILL is 9; a negative pid targets
        // that exact group, including a verifier's retained probe descendants.
        if self.active {
            if let Ok(pid) = i32::try_from(self.process.id()) {
                unsafe {
                    kill(-pid, 9);
                }
            }
            let _ = self.process.wait();
        }
    }
}
