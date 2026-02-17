use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Result, bail};

pub trait CargoCmd {
    fn manifest_path(&mut self, path: &Option<impl AsRef<Path>>) -> &mut Self;
    fn target_dir(&mut self, path: impl AsRef<Path>) -> &mut Self;
    fn target(&mut self, triplet: impl AsRef<str>) -> &mut Self;
    fn cc_env(&mut self, triplet: impl AsRef<str>, cc: impl AsRef<Path>) -> &mut Self;
    fn ar_env(&mut self, triplet: impl AsRef<str>, ar: impl AsRef<Path>) -> &mut Self;
    fn sysroot(&mut self, path: impl AsRef<Path>) -> &mut Self;
    fn entrypoint(&mut self, entry: impl AsRef<str>) -> &mut Self;
    fn append_rustflags(&mut self, flags: impl AsRef<OsStr>) -> &mut Self;
    fn append_cflags(&mut self, triplet: impl AsRef<str>, flags: impl AsRef<OsStr>) -> &mut Self;
    fn append_bindgen_cflags(
        &mut self,
        triplet: impl AsRef<str>,
        flags: impl AsRef<OsStr>,
    ) -> &mut Self;
    fn allow_unstable(&mut self) -> &mut Self;
    fn resolve_env(
        &self,
        base: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> HashMap<OsString, OsString>;
    fn checked_output(&mut self) -> Result<CheckedOutput>;
    fn checked_status(&mut self) -> Result<()>;
}

#[derive(Clone, Hash)]
pub struct CargoBinary {
    pub path: PathBuf,
    pub rustup_toolchain: Option<OsString>,
}

impl CargoBinary {
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(&self.path);
        if let Some(rustup_toolchain) = &self.rustup_toolchain {
            cmd.env("RUSTUP_TOOLCHAIN", rustup_toolchain);
        }
        cmd
    }
}

pub fn find_cargo() -> Result<CargoBinary> {
    let cargo = match env::var_os("CARGO") {
        Some(cargo) => Path::new(&cargo).canonicalize()?,
        None => which::which("cargo")?.canonicalize()?,
    };
    let rustup_toolchain = env::var_os("RUSTUP_TOOLCHAIN");
    Ok(CargoBinary {
        path: cargo,
        rustup_toolchain,
    })
}

pub fn cargo_cmd() -> Result<Command> {
    Ok(find_cargo()?.command())
}

pub struct CheckedOutput {
    pub stdout: Vec<u8>,
    #[allow(dead_code)]
    pub stderr: Vec<u8>,
}

impl CargoCmd for Command {
    fn manifest_path(&mut self, path: &Option<impl AsRef<Path>>) -> &mut Self {
        if let Some(path) = path {
            self.arg("--manifest-path").arg(path.as_ref());
        }
        self
    }

    fn target_dir(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.env("CARGO_BUILD_TARGET_DIR", path.as_ref());
        self.env("CARGO_TARGET_DIR", path.as_ref());
        self
    }

    fn target(&mut self, triplet: impl AsRef<str>) -> &mut Self {
        self.env("CARGO_BUILD_TARGET", triplet.as_ref());
        self
    }

    fn cc_env(&mut self, triplet: impl AsRef<str>, cc: impl AsRef<Path>) -> &mut Self {
        // set both CC_<triplet> and CLANG_PATH so that cc-rs and bindgen can pick it up
        // use CC_<triplet> as this is the highest priority for cc-rs
        // see https://docs.rs/cc/latest/cc/#external-configuration-via-environment-variables
        self.env(format!("CC_{}", triplet.as_ref()), cc.as_ref());

        // bindgen uses clang-sys, which looks for clang in the CLANG_PATH env var
        // see https://github.com/KyleMayes/clang-sys?tab=readme-ov-file#environment-variables
        // TODO(jprendes): can we make this env var target-specific as well?
        self.env("CLANG_PATH", cc.as_ref());

        // In windows, set LIBCLANG_PATH
        if cfg!(windows)
            && let Some(parent) = cc.as_ref().parent()
        {
            // TODO(jprendes): is this considering all possible situations?
            // * why is CLANG_PATH not enough on windows?
            // see https://rust-lang.github.io/rust-bindgen/requirements.html#windows
            // see https://github.com/KyleMayes/clang-sys?tab=readme-ov-file#environment-variables
            // TODO(jprendes): can we make this env var target-specific as well?
            if parent.join("libclang.dll").exists() {
                self.env("LIBCLANG_PATH", parent.join("libclang.dll"));
            } else if parent.join("clang.dll").exists() {
                self.env("LIBCLANG_PATH", parent.join("clang.dll"));
            }
        }
        self
    }

    fn ar_env(&mut self, triplet: impl AsRef<str>, ar: impl AsRef<Path>) -> &mut Self {
        // set AR_<triplet> so that cc-rs can pick it up
        self.env(format!("AR_{}", triplet.as_ref()), ar.as_ref());
        self
    }

    fn sysroot(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.append_rustflags("--sysroot")
            .append_rustflags(path.as_ref())
    }

    fn entrypoint(&mut self, entry: impl AsRef<str>) -> &mut Self {
        let entry = entry.as_ref();
        self.append_rustflags(format!("-Clink-args=-e{entry}"))
    }

    fn append_rustflags(&mut self, flags: impl AsRef<OsStr>) -> &mut Self {
        if flags.as_ref().is_empty() {
            return self;
        }

        let mut new_flags = get_env(self, "RUSTFLAGS").unwrap_or_default();
        if !new_flags.is_empty() {
            new_flags.push(" ");
        }
        new_flags.push(flags.as_ref());
        self.env("RUSTFLAGS", new_flags);
        self
    }

    fn append_cflags(&mut self, triplet: impl AsRef<str>, flags: impl AsRef<OsStr>) -> &mut Self {
        if flags.as_ref().is_empty() {
            return self;
        }

        let triplet = triplet.as_ref();

        let mut new_flags = find_cflags(self, triplet);

        if !new_flags.is_empty() {
            new_flags.push(" ");
        }
        new_flags.push(flags.as_ref());
        self.env(format!("CFLAGS_{triplet}"), new_flags);

        self.append_bindgen_cflags(triplet, flags);

        self
    }

    fn append_bindgen_cflags(
        &mut self,
        triplet: impl AsRef<str>,
        flags: impl AsRef<OsStr>,
    ) -> &mut Self {
        if flags.as_ref().is_empty() {
            return self;
        }

        // For some reason we need to escape backslashes on Windows
        // TODO(jprendes): check if we need to do any better escaping for other special characters
        let flags = flags.as_ref().to_string_lossy().replace("\\", "\\\\");

        // TODO(jprendes): account and use the target specific variants of BINDGEN_EXTRA_CLANG_ARGS
        // see https://github.com/rust-lang/rust-bindgen/tree/main?tab=readme-ov-file#environment-variables
        let mut new_flags =
            get_env(self, "BINDGEN_EXTRA_CLANG_ARGS").unwrap_or_else(|| find_cflags(self, triplet));
        if !new_flags.is_empty() {
            new_flags.push(" ");
        }
        new_flags.push(flags);
        self.env("BINDGEN_EXTRA_CLANG_ARGS", new_flags);
        self
    }

    fn allow_unstable(&mut self) -> &mut Self {
        self.env("RUSTC_BOOTSTRAP", "1")
    }

    fn resolve_env(
        &self,
        base: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> HashMap<OsString, OsString> {
        merge_env(base, self.get_envs())
    }

    fn checked_output(&mut self) -> Result<CheckedOutput> {
        let output = self.output();

        let Ok(output) = output else {
            bail!("Failed to execute command:\n{self:?}");
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if let Some(code) = output.status.code() {
                bail!("Command exited with code {code}:\n{self:?}\n{stderr}");
            } else {
                bail!("Command terminated by signal:\n{self:?}\n{stderr}");
            }
        }

        Ok(CheckedOutput {
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn checked_status(&mut self) -> Result<()> {
        self.stderr(Stdio::inherit());
        self.stdout(Stdio::inherit());
        let _ = self.checked_output()?;
        Ok(())
    }
}

fn get_env(cmd: &Command, key: &str) -> Option<OsString> {
    let mut envs = cmd.get_envs();
    match envs.find(|(k, _)| *k == key) {
        Some((_, v)) => v.map(ToOwned::to_owned),
        None => std::env::var_os(key),
    }
}

fn find_cflags(cmd: &Command, triplet: impl AsRef<str>) -> OsString {
    let triplet = triplet.as_ref();
    let triplet_snake_case = triplet.replace('-', "_");
    let triplet_snake_case_upper = triplet_snake_case.to_uppercase();

    let search_keys = [
        format!("CFLAGS_{triplet}"),
        format!("CFLAGS_{triplet_snake_case}"),
        format!("CFLAGS_{triplet_snake_case_upper}"),
        "CFLAGS_hyperlight".to_string(),
        "CFLAGS_HYPERLIGHT".to_string(),
        "HYPERLIGHT_CFLAGS".to_string(),
        "TARGET_CFLAGS".to_string(),
        "CFLAGS".to_string(),
    ];

    search_keys
        .iter()
        .find_map(|key| get_env(cmd, key))
        .unwrap_or_default()
}

pub fn merge_env(
    base: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    envs: impl IntoIterator<Item = (impl AsRef<OsStr>, Option<impl AsRef<OsStr>>)>,
) -> HashMap<OsString, OsString> {
    let mut base = base
        .into_iter()
        .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned()))
        .collect::<HashMap<_, _>>();

    for (k, v) in envs {
        if let Some(v) = v {
            base.insert(k.as_ref().to_owned(), v.as_ref().to_owned());
        } else {
            base.remove(k.as_ref());
        }
    }

    base
}
