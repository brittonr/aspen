use anyhow::Result;

mod cargo_cmd;
mod cli;
mod command;
mod sysroot;
mod toolchain;

use cargo_cmd::CargoCmd;
use cli::Args;
pub use command::Command;

/// Constructs a new `Command` for launching cargo targeting
/// [hyperlight](https://github.com/hyperlight-dev/hyperlight) guest code.
///
/// The value of the `CARGO` environment variable is used if it is set; otherwise, the
/// default `cargo` from the system PATH is used.
/// If `RUSTUP_TOOLCHAIN` is set in the environment, it is also propagated to the
/// child process to ensure correct functioning of the rustup wrappers.
///
/// The default configuration is:
/// - No arguments to the program
/// - Inherits the current process's environment
/// - Inherits the current process's working directory
///
/// # Errors
///
/// This function will return an error if:
/// - If the `CARGO` environment variable is set but it specifies an invalid path
/// - If the `CARGO` environment variable is not set and the `cargo` program cannot be found in the system PATH
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use cargo_hyperlight::cargo;
///
/// let command = cargo().unwrap();
/// ```
pub fn cargo() -> Result<Command> {
    Command::new()
}

impl Args {
    pub fn sysroot_dir(&self) -> std::path::PathBuf {
        self.target_dir.join("sysroot")
    }

    pub fn triplet_dir(&self) -> std::path::PathBuf {
        self.sysroot_dir()
            .join("lib")
            .join("rustlib")
            .join(&self.target)
    }

    pub fn build_dir(&self) -> std::path::PathBuf {
        self.sysroot_dir().join("target")
    }

    pub fn libs_dir(&self) -> std::path::PathBuf {
        self.triplet_dir().join("lib")
    }

    pub fn includes_dir(&self) -> std::path::PathBuf {
        self.triplet_dir().join("include")
    }

    pub fn crate_dir(&self) -> std::path::PathBuf {
        self.sysroot_dir().join("crate")
    }

    // build_plan_dir removed: --build-plan was removed in Cargo 1.93.
    // Artifact discovery now uses --message-format=json during the build itself.
}

trait CargoCommandExt {
    fn populate_from_args(&mut self, args: &Args) -> &mut Self;
}

impl CargoCommandExt for std::process::Command {
    fn populate_from_args(&mut self, args: &Args) -> &mut Self {
        self.target(&args.target);
        self.sysroot(args.sysroot_dir());
        self.append_rustflags("--cfg=hyperlight");
        self.append_rustflags("--check-cfg=cfg(hyperlight)");
        self.entrypoint("entrypoint");
        // Custom target specs require -Zunstable-options for both Cargo and rustc.
        // RUSTC_BOOTSTRAP=1 alone is insufficient on nightly Cargo >= 1.93.
        // Pass to Cargo as a flag and to rustc via RUSTFLAGS so Cargo's
        // internal target probe also receives it.
        self.allow_unstable();
        self.arg("-Zunstable-options");
        self.append_rustflags("-Zunstable-options");
        if let Some(clang) = &args.clang {
            self.cc_env(&args.target, clang);
        } else {
            // If we couldn't find clang, use the default from the
            // system path. This will then error if we try to build
            // using cc-rs, but will succeed otherwise.
            self.cc_env(&args.target, "clang");
        }
        if let Some(ar) = &args.ar {
            self.ar_env(&args.target, ar);
        } else {
            // do nothing, let cc-rs find ar itself
        }
        self.append_cflags(&args.target, toolchain::cflags(args));

        self
    }
}

impl Args {
    pub fn prepare_sysroot(&self) -> Result<()> {
        // Build sysroot
        sysroot::build(self)?;

        // Build toolchain
        toolchain::prepare(self)?;

        Ok(())
    }
}
