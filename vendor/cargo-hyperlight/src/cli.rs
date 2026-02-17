use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::env::consts::ARCH;
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::{Context, Result};
use const_format::formatcp;
use os_str_bytes::OsStrBytesExt as _;

use crate::cargo_cmd::{CargoCmd as _, cargo_cmd};
use crate::toolchain;

pub struct Args {
    pub manifest_path: Option<PathBuf>,
    pub target_dir: PathBuf,
    pub target: String,
    pub env: HashMap<OsString, OsString>,
    pub current_dir: PathBuf,
    pub clang: Option<PathBuf>,
    pub ar: Option<PathBuf>,
}

pub trait WarningLevel {
    type Error;
    fn warning<T: Debug>(
        &self,
        msg: &str,
        err: impl Into<anyhow::Error>,
        default: T,
    ) -> Result<T, Self::Error>;
}

pub struct Warning;

#[doc(hidden)]
pub mod warning {
    pub struct WarningIgnore;
    pub struct WarningWarn;
    #[allow(dead_code)]
    pub struct WarningError;
}

impl Warning {
    pub const IGNORE: warning::WarningIgnore = warning::WarningIgnore;
    pub const WARN: warning::WarningWarn = warning::WarningWarn;
    #[allow(dead_code)]
    pub const ERROR: warning::WarningError = warning::WarningError;
}

impl WarningLevel for warning::WarningIgnore {
    type Error = Infallible;
    fn warning<T: Debug>(
        &self,
        _msg: &str,
        _err: impl Into<anyhow::Error>,
        default: T,
    ) -> Result<T, Self::Error> {
        Ok(default)
    }
}

impl WarningLevel for warning::WarningWarn {
    type Error = Infallible;
    fn warning<T: Debug>(
        &self,
        msg: &str,
        err: impl Into<anyhow::Error>,
        default: T,
    ) -> Result<T, Self::Error> {
        warning(msg);
        warning(format!("{:?}", err.into()));
        warning(format!("using {default:?}"));
        Ok(default)
    }
}

impl WarningLevel for warning::WarningError {
    type Error = anyhow::Error;
    fn warning<T: Debug>(
        &self,
        msg: &str,
        err: impl Into<anyhow::Error>,
        _default: T,
    ) -> Result<T, Self::Error> {
        Err(err.into()).context(msg.to_string())
    }
}

impl Args {
    pub fn parse<W: WarningLevel>(
        args: impl IntoIterator<Item = impl Into<OsString> + Clone>,
        env: impl IntoIterator<Item = (impl Into<OsString>, impl Into<OsString>)>,
        cwd: Option<impl Into<PathBuf>>,
        warn: W,
    ) -> Result<Args, W::Error> {
        let mut args = ArgsImpl::parse_args(args);
        args.env = env.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        let cwd = match cwd {
            Some(cwd) => cwd.into(),
            None => match env::current_dir() {
                Ok(cwd) => cwd,
                Err(err) => {
                    warn.warning("Could not get current directory", err, PathBuf::from("."))?
                }
            },
        };
        args.current_dir = cwd.clone();
        Args::try_from_with_defaults(warn, args)
    }
}

fn warning(msg: impl AsRef<str>) {
    eprintln!(
        "{}{}{}",
        console::style("warning").yellow().bold(),
        console::style(": ").bold(),
        console::style(msg.as_ref()).bold(),
    );
}

impl TryFrom<ArgsImpl> for Args {
    type Error = anyhow::Error;

    fn try_from(value: ArgsImpl) -> Result<Self> {
        Args::try_from_with_defaults(Warning::ERROR, value)
    }
}

impl Args {
    fn try_from_with_defaults<W: WarningLevel>(warn: W, value: ArgsImpl) -> Result<Self, W::Error> {
        let manifest_path = value.manifest_path;

        let target_dir = match value.target_dir {
            Some(dir) => dir,
            None => match resolve_target_dir(&manifest_path, &value.env, &value.current_dir) {
                Ok(dir) => dir,
                Err(err) => warn.warning(
                    "could not resolve target directory",
                    err,
                    value.current_dir.join("target"),
                )?,
            },
        };

        let target = match value.target {
            Some(triplet) => triplet,
            None => match resolve_target(&value.env, &value.current_dir) {
                Ok(triplet) => triplet,
                Err(err) => warn.warning(
                    "could not resolve target triple",
                    err,
                    DEFAULT_TARGET.to_string(),
                )?,
            },
        };

        let target = if target.ends_with("-hyperlight-none") {
            target
        } else {
            let (arch, _) = target.split_once('-').unwrap_or((&target, ""));
            warn.warning(
                "requested target is not a hyperlight target",
                anyhow::anyhow!("invalid hyperlight target: {target}"),
                format!("{arch}-hyperlight-none"),
            )?
        };

        let target_dir = value.current_dir.join(target_dir);

        Ok(Args {
            manifest_path,
            target_dir,
            target,
            env: value.env,
            current_dir: value.current_dir,
            clang: toolchain::find_cc().ok(),
            ar: toolchain::find_ar().ok(),
        })
    }
}

const DEFAULT_TARGET: &str = const { formatcp!("{ARCH}-hyperlight-none") };

#[derive(Default)]
//#[command(disable_help_subcommand = true)]
struct ArgsImpl {
    /// Path to Cargo.toml
    manifest_path: Option<PathBuf>,

    /// Directory for all generated artifacts
    target_dir: Option<PathBuf>,

    /// Target triple to build for
    target: Option<String>,

    /// Environment variables to set
    env: HashMap<OsString, OsString>,

    /// Current working directory
    pub current_dir: PathBuf,
}

fn parse_arg(
    flag: &str,
    arg: &OsStr,
    args: &mut impl Iterator<Item = OsString>,
) -> Option<OsString> {
    let value = arg.strip_prefix(flag)?;
    if value.is_empty() {
        args.next()
    } else {
        value.strip_prefix("=").map(OsStr::to_os_string)
    }
}

impl ArgsImpl {
    pub fn parse_args(args: impl IntoIterator<Item = impl Into<OsString> + Clone>) -> Self {
        let mut this = Self::default();
        let mut args = args.into_iter().map(Into::into);

        while let Some(arg) = args.next() {
            if arg == "--" {
                break;
            }
            if let Some(path) = parse_arg("--manifest-path", &arg, &mut args) {
                this.manifest_path = Some(PathBuf::from(path));
                continue;
            }
            if let Some(dir) = parse_arg("--target-dir", &arg, &mut args) {
                this.target_dir = Some(PathBuf::from(dir));
                continue;
            }
            if let Some(triplet) = parse_arg("--target", &arg, &mut args) {
                this.target = Some(triplet.to_string_lossy().to_string());
                continue;
            }
        }
        this
    }
}

#[derive(serde::Deserialize)]
struct CargoMetadata {
    target_directory: PathBuf,
}

fn resolve_target_dir(
    manifest_path: &Option<PathBuf>,
    env: &HashMap<OsString, OsString>,
    cwd: &PathBuf,
) -> Result<PathBuf> {
    let output = cargo_cmd()?
        .env_clear()
        .envs(env.iter())
        .current_dir(cwd)
        .arg("metadata")
        .manifest_path(manifest_path)
        .arg("--format-version=1")
        .arg("--no-deps")
        .checked_output()
        .context("Failed to get cargo metadata")?;

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("Failed to parse cargo metadata")?;

    Ok(metadata.target_directory)
}

fn resolve_target(env: &HashMap<OsString, OsString>, cwd: &PathBuf) -> Result<String> {
    let output = cargo_cmd()?
        .env_clear()
        .envs(env.iter())
        .current_dir(cwd)
        .arg("config")
        .arg("get")
        .arg("--quiet")
        .arg("--format=json-value")
        .arg("-Zunstable-options")
        .arg("build.target")
        // cargo config is an unstable feature
        .allow_unstable()
        // use output instead of checked_output
        // as cargo will error if build.target is not set
        .output()
        .context("Failed to get cargo config")?;

    let target = String::from_utf8_lossy(&output.stdout);
    let target = target.trim();
    let target = target.trim_matches(|c| c == '"' || c == '\'');

    if target.is_empty() {
        Ok(DEFAULT_TARGET.into())
    } else {
        Ok(target.into())
    }
}
