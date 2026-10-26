// SPDX-License-Identifier: AGPL-3.0-or-later
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use cap_fs_ext::FollowSymlinks;
use cap_fs_ext::OpenOptionsFollowExt;
use cap_std::fs::Dir;
use cap_std::fs::OpenOptions;
use molten_batch_application::Error;

#[derive(Clone)]
pub struct CapabilityRoot {
    dir: Arc<Dir>,
}
impl CapabilityRoot {
    /// The only ambient filesystem acquisition in runtime adapters. The caller
    /// explicitly supplies this root; everything beneath uses a directory handle.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let dir = Dir::open_ambient_dir(path, cap_std::ambient_authority()).map_err(super::invalid)?;
        Ok(Self { dir: Arc::new(dir) })
    }
    pub fn from_dir(dir: Dir) -> Self {
        Self { dir: Arc::new(dir) }
    }
    fn name(name: &str) -> Result<(), Error> {
        if name.is_empty()
            || name.len() > 128
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            || name == "."
            || name == ".."
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub fn file(&self, name: &str, create: bool) -> Result<std::fs::File, Error> {
        Self::name(name)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(create).follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_fs_ext::OpenOptionsExt;
            options.mode(0o600);
        }
        let f = self.dir.open_with(name, &options).map_err(super::invalid)?.into_std();
        if !f.metadata().map_err(super::invalid)?.is_file() {
            return Err(Error::Denied);
        }
        if create {
            self.sync()?;
        }
        Ok(f)
    }
    pub fn read(&self, name: &str, max: usize) -> Result<Vec<u8>, Error> {
        Self::name(name)?;
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = self.dir.open_with(name, &options).map_err(super::invalid)?;
        if !file.metadata().map_err(super::invalid)?.is_file() {
            return Err(Error::Denied);
        }
        let mut bytes = Vec::new();
        file.take(max as u64 + 1).read_to_end(&mut bytes).map_err(super::invalid)?;
        if bytes.len() > max {
            return Err(Error::Bound);
        }
        Ok(bytes)
    }
    pub fn create_new(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        Self::name(name)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true).follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_fs_ext::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = self.dir.open_with(name, &options).map_err(super::invalid)?.into_std();
        file.write_all(bytes).map_err(super::invalid)?;
        file.sync_all().map_err(super::invalid)?;
        self.sync()
    }
    pub fn replace(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        Self::name(name)?;
        let temporary = format!("{name}.new");
        // A leftover staging file is a blocker, never silently followed/deleted.
        self.create_new(&temporary, bytes)?;
        self.dir.rename(&temporary, &self.dir, name).map_err(super::invalid)?;
        self.sync()
    }
    // r[impl molten.batch_orchestration.repairs]
    fn sync(&self) -> Result<(), Error> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        self.dir
            .open_with(".", &options)
            .map_err(super::invalid)?
            .into_std()
            .sync_all()
            .map_err(super::invalid)
    }
    pub fn read_json<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T, Error> {
        serde_json::from_slice(&self.read(name, 1024 * 1024)?).map_err(super::invalid)
    }
}
