//! Narrow retained process capabilities; no raw directory handle escapes.

pub const MAX_EXECUTABLE_BYTES: u64 = 256 * 1_024 * 1_024;

pub struct Executable {
    file: cap_std::fs::File,
    size: u64,
}

pub struct Workspace {
    directory: cap_std::fs::Dir,
}

impl super::authority::DirectoryRoot {
    pub fn open_executable(&self, path: &super::locator::RelativePath) -> crate::error::Result<Executable> {
        let directory = self.try_clone_dir()?;
        let observed = super::filesystem::observe_file(&directory, path.as_path())?;
        let super::authority::FileObservation::Regular(file) = observed else {
            return Err(super::invalid("process executable must be an owned regular file"));
        };
        if file.size > MAX_EXECUTABLE_BYTES || file.unix_mode.is_none_or(|mode| mode & 0o111 == 0) {
            return Err(super::invalid("process executable is oversized or lacks execute permission"));
        }
        Ok(Executable {
            file: file.file,
            size: file.size,
        })
    }
}

impl super::namespace::Scope {
    pub fn process_workspace(&self) -> crate::error::Result<Workspace> {
        Ok(Workspace {
            directory: self.try_clone_dir()?,
        })
    }
}

impl Executable {
    /// Measure bytes from the same retained inode the process will execute.
    pub fn read_bounded(&self, maximum: u64) -> crate::error::Result<Vec<u8>> {
        if maximum > MAX_EXECUTABLE_BYTES || self.size > maximum {
            return Err(super::invalid("executable read exceeds admitted byte bound"));
        }
        let mut file = self.file.try_clone().map_err(crate::error::Failure::from)?;
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0)).map_err(crate::error::Failure::from)?;
        let mut bounded = std::io::Read::take(file, maximum + 1);
        let capacity =
            usize::try_from(self.size).map_err(|_| super::invalid("executable size exceeds platform address range"))?;
        let mut bytes = Vec::with_capacity(capacity);
        std::io::Read::read_to_end(&mut bounded, &mut bytes).map_err(crate::error::Failure::from)?;
        if bytes.len() as u64 > maximum {
            return Err(super::invalid("executable grew beyond byte bound"));
        }
        Ok(bytes)
    }

    pub fn process_path(&self) -> crate::error::Result<std::path::PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let descriptor = std::os::fd::AsRawFd::as_raw_fd(&self.file);
            Ok(std::path::PathBuf::from(format!("/proc/{}/fd/{descriptor}", std::process::id())))
        }
        #[cfg(not(target_os = "linux"))]
        Err(super::invalid("retained executable process paths require the admitted Linux profile"))
    }
}

impl Workspace {
    pub fn process_path(&self) -> crate::error::Result<std::path::PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let descriptor = std::os::fd::AsRawFd::as_raw_fd(&self.directory);
            Ok(std::path::PathBuf::from(format!("/proc/{}/fd/{descriptor}", std::process::id())))
        }
        #[cfg(not(target_os = "linux"))]
        Err(super::invalid("retained workspace process paths require the admitted Linux profile"))
    }
}
