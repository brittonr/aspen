//! This module implements a wrapper around snix-eval's [EvalIO] type,
//! adding functionality which is required by snix-cli:
//!
//! 1. Handling the C++ Nix `__corepkgs__`-hack for nixpkgs bootstrapping.
//!
//! All uses of [EvalIO] in snix-cli must make use of this wrapper,
//! otherwise nixpkgs bootstrapping will not work.

use snix_eval::{EvalIO, FileType};
use std::ffi::{OsStr, OsString};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

// TODO: Merge this together with SnixStoreIO?
pub struct SnixIO<T> {
    // Actual underlying [EvalIO] implementation.
    actual: T,
}

impl<T> SnixIO<T> {
    pub fn new(actual: T) -> Self {
        Self { actual }
    }
}

impl<T> EvalIO for SnixIO<T>
where
    T: AsRef<dyn EvalIO>,
{
    fn store_dir(&self) -> Option<String> {
        self.actual.as_ref().store_dir()
    }

    fn import_path(&self, path: &Path) -> io::Result<PathBuf> {
        self.actual.as_ref().import_path(path)
    }

    fn path_exists(&self, path: &Path) -> io::Result<bool> {
        if path.starts_with("/__corepkgs__") {
            return Ok(true);
        }

        self.actual.as_ref().path_exists(path)
    }

    fn open(&self, path: &Path) -> io::Result<Box<dyn io::Read>> {
        // Bundled version of corepkgs/fetchurl.nix. The counterpart
        // of this happens in [crate::configure_nix_path], where the `nix_path`
        // of the evaluation has `nix=/__corepkgs__` added to it.
        //
        // This workaround is similar to what cppnix does for passing
        // the path through.
        //
        // TODO: this comparison is bad we should use the sane path library.
        if path.starts_with("/__corepkgs__/fetchurl.nix") {
            return Ok(Box::new(Cursor::new(include_bytes!("fetchurl.nix"))));
        }

        self.actual.as_ref().open(path)
    }

    fn file_type(&self, path: &Path) -> io::Result<FileType> {
        self.actual.as_ref().file_type(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<(bytes::Bytes, FileType)>> {
        self.actual.as_ref().read_dir(path)
    }

    fn get_env(&self, key: &OsStr) -> Option<OsString> {
        self.actual.as_ref().get_env(key)
    }
}
