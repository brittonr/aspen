//! Concrete capability-local intent/terminal persistence, ordered before live effects.
use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

pub struct Journal {
    root: cap_std::fs::Dir,
    role: &'static str,
}
impl Journal {
    pub fn new(root: &cap_std::fs::Dir, role: &'static str) -> super::Result<Self> {
        if role != "client" && role != "server" {
            return Err("unsupported journal participant".into());
        }
        Ok(Self {
            root: root.try_clone().map_err(super::failure)?,
            role,
        })
    }
    pub fn begin(&self, lease: &super::authority::Lease) -> super::Result<String> {
        let value = crate::codec::record("component-rpc-intent-v1", vec![
            lease.scope().value(),
            crate::codec::string(lease.receipt()),
            crate::codec::string(super::bridge::invocation(lease.scope())?),
            crate::codec::string("single-attempt-no-automatic-retry"),
        ]);
        self.persist("intent", &value)?;
        super::hash(&value)
    }
    pub fn finish(&self, intent: &str, result: Option<&super::transcript::Transcript>) -> super::Result<()> {
        let (class, reference) = match result {
            Some(transcript) => {
                self.persist("live", &transcript.value()?)?;
                ("returned", transcript.reference.as_str())
            }
            None => ("uncertain-reconcile-before-retry", intent),
        };
        let value = crate::codec::record("component-rpc-terminal-observation-v1", vec![
            crate::codec::string(intent),
            crate::codec::string(class),
            crate::codec::string(reference),
            crate::codec::string("not-exactly-once-effects"),
        ]);
        self.persist("terminal", &value)
    }
    fn persist(&self, phase: &str, value: &preserves::IOValue) -> super::Result<()> {
        let bytes = crate::codec::canonical_bytes(value).map_err(super::failure)?;
        let leaf = format!("{}-{phase}.preserves", self.role);
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
        let mut file = self.root.open_with(&leaf, &options).map_err(super::failure)?;
        file.write_all(&bytes).map_err(super::failure)?;
        file.sync_all().map_err(super::failure)?;
        self.root.open(".").and_then(|directory| directory.sync_all()).map_err(super::failure)
    }
}
