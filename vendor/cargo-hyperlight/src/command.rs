use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::ffi::{OsStr, OsString, c_char};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::{env, iter};

use anyhow::{Context, Result};
use os_str_bytes::OsStrBytesExt;

use crate::CargoCommandExt;
use crate::cargo_cmd::{CargoBinary, CargoCmd as _, find_cargo, merge_env};
use crate::cli::{Args, Warning};

/// A process builder for cargo commands, providing a similar API to `std::process::Command`.
///
/// `Command` is a wrapper around `std::process::Command` specifically designed for
/// executing cargo commands targeting [hyperlight](https://github.com/hyperlight-dev/hyperlight)
/// guest code.
/// Before executing the desired command, `Command` takes care of setting up the
/// appropriate environment. It:
/// * creates a custom rust target for hyperlight guest code
/// * creates a sysroot with Rust's libs core and alloc
/// * finds the appropriate compiler and archiver for any C dependencies
/// * sets up necessary environment variables for `cc-rs` and `bindgen` to work correctly.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust,no_run
/// use cargo_hyperlight::cargo;
///
/// let mut command = cargo().unwrap();
/// command.arg("build").arg("--release");
/// command.exec(); // This will replace the current process
/// ```
///
/// Setting environment variables and working directory:
///
/// ```rust
/// use cargo_hyperlight::cargo;
///
/// let mut command = cargo().unwrap();
/// command
///     .current_dir("/path/to/project")
///     .env("CARGO_TARGET_DIR", "/custom/target")
///     .args(["build", "--release"]);
/// ```
#[derive(Clone)]
pub struct Command {
    cargo: CargoBinary,
    /// Arguments to pass to the cargo program
    args: Vec<OsString>,
    /// Environment variable mappings to set for the child process
    inherit_envs: bool,
    inherit_cargo_envs: bool,
    envs: BTreeMap<OsString, Option<OsString>>,
    // Working directory for the child process
    current_dir: Option<PathBuf>,
}

impl Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args = self.build_args_infallible();
        let mut cmd = self.command();
        cmd.populate_from_args(&args);

        write!(f, "env ")?;
        if let Some(current_dir) = &self.current_dir {
            write!(f, "-C {current_dir:?} ")?;
        }
        if !self.inherit_envs {
            write!(f, "-i ")?;
        }
        for (k, v) in cmd.get_envs() {
            if v.is_none() {
                write!(f, "-u {} ", k.to_string_lossy())?;
            }
        }
        for (k, v) in cmd.get_envs() {
            if let Some(v) = v {
                write!(f, "{}={:?} ", k.to_string_lossy(), v)?;
            }
        }
        write!(f, "{:?} ", self.get_program())?;
        for arg in &self.args {
            write!(f, "{arg:?} ")?;
        }
        writeln!(f)
    }
}

impl Command {
    /// Constructs a new `Command` for launching the cargo program.
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
    pub(crate) fn new() -> Result<Self> {
        let cargo = find_cargo()?;
        Ok(Self {
            cargo,
            args: Vec::new(),
            envs: BTreeMap::new(),
            inherit_envs: true,
            inherit_cargo_envs: true,
            current_dir: None,
        })
    }

    /// Adds an argument to pass to the cargo program.
    ///
    /// Only one argument can be passed per use. So instead of:
    ///
    /// ```no_run
    /// # let mut command = cargo_hyperlight::cargo().unwrap();
    /// command.arg("--features some_feature");
    /// ```
    ///
    /// usage would be:
    ///
    /// ```no_run
    /// # let mut command = cargo_hyperlight::cargo().unwrap();
    /// command.arg("--features").arg("some_feature");
    /// ```
    ///
    /// To pass multiple arguments see [`args`].
    ///
    /// [`args`]: Command::args
    ///
    /// Note that the argument is not shell-escaped, so if you pass an argument like
    /// `"hello world"`, it will be passed as a single argument with the literal
    /// `hello world`, not as two arguments `hello` and `world`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .arg("build")
    ///     .arg("--release")
    ///     .exec();
    /// ```
    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Adds multiple arguments to pass to the cargo program.
    ///
    /// To pass a single argument see [`arg`].
    ///
    /// [`arg`]: Command::arg
    ///
    /// Note that the arguments are not shell-escaped, so if you pass an argument
    /// like `"hello world"`, it will be passed as a single argument with the
    /// literal `hello world`, not as two arguments `hello` and `world`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .args(["build", "--release"])
    ///     .exec();
    /// ```
    pub fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    /// Sets the working directory for the child process.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .current_dir("path/to/project")
    ///     .arg("build")
    ///     .exec();
    /// ```
    ///
    /// [`canonicalize`]: std::fs::canonicalize
    pub fn current_dir(&mut self, dir: impl AsRef<Path>) -> &mut Self {
        self.current_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Inserts or updates an explicit environment variable mapping.
    ///
    /// This method allows you to add an environment variable mapping to the spawned process
    /// or overwrite a variable if it already exists.
    ///
    /// Child processes will inherit environment variables from their parent process by
    /// default. Environment variables explicitly set using [`env`] take precedence
    /// over inherited variables. You can disable environment variable inheritance entirely
    /// using [`env_clear`] or for a single key using [`env_remove`].
    ///
    /// Note that environment variable names are case-insensitive (but
    /// case-preserving) on Windows and case-sensitive on all other platforms.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .env("CARGO_TARGET_DIR", "/path/to/target")
    ///     .arg("build")
    ///     .exec();
    /// ```
    ///
    /// [`env`]: Command::env
    /// [`env_clear`]: Command::env_clear
    /// [`env_remove`]: Command::env_remove
    pub fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.envs
            .insert(key.as_ref().to_owned(), Some(value.as_ref().to_owned()));
        self
    }

    /// Clears all environment variables that will be set for the child process.
    ///
    /// This method will remove all environment variables from the child process,
    /// including those that would normally be inherited from the parent process.
    /// Environment variables can be added back individually using [`env`].
    ///
    /// If `RUSTUP_TOOLCHAIN` was set in the parent process, it will be preserved.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .env_clear()
    ///     .env("CARGO_TARGET_DIR", "/path/to/target")
    ///     .arg("build")
    ///     .exec();
    /// ```
    ///
    /// [`env`]: Command::env
    pub fn env_clear(&mut self) -> &mut Self {
        self.inherit_envs = false;
        self.envs.clear();
        self
    }

    /// Clears all `CARGO_` environment variables that will be set for the child process,
    /// except for `CARGO_HOME`.
    ///
    /// This method will remove all environment variables starting with `CARGO_`
    /// from the child process, including those that would normally be inherited
    /// from the parent process, except for `CARGO_HOME`. Other environment variables
    /// will remain unaffected. Environment variables can be added back individually
    /// using [`env`].
    ///
    /// This is particularly useful when using cargo-hyperlight from a build script
    /// or other cargo-invoked context where `CARGO_` variables may change the behavior
    /// of the cargo command being executed.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .env_clear_cargo()
    ///     .env("CARGO_TARGET_DIR", "/path/to/target")
    ///     .arg("build")
    ///     .exec();
    /// ```
    ///
    /// [`env`]: Command::env
    pub fn env_clear_cargo(&mut self) -> &mut Self {
        self.inherit_cargo_envs = false;
        self.envs.retain(|k, _| !is_cargo_env(k));
        self
    }

    #[doc(hidden)]
    #[deprecated(note = "use `env_clear_cargo` instead")]
    pub fn env_clear_cargo_vars(&mut self) -> &mut Self {
        self.env_clear_cargo()
    }

    /// Removes an explicitly set environment variable and prevents inheriting
    /// it from a parent process.
    ///
    /// This method will ensure that the specified environment variable is not
    /// present in the spawned process's environment, even if it was present
    /// in the parent process. This serves to "unset" environment variables.
    ///
    /// Note that environment variable names are case-insensitive (but
    /// case-preserving) on Windows and case-sensitive on all other platforms.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .env_remove("CARGO_TARGET_DIR")
    ///     .arg("build")
    ///     .exec();
    /// ```
    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.envs.insert(key.as_ref().to_owned(), None);
        self
    }

    /// Inserts or updates multiple explicit environment variable mappings.
    ///
    /// This method allows you to add multiple environment variable mappings
    /// to the spawned process or overwrite variables if they already exist.
    /// Environment variables can be passed as a `HashMap` or any other type
    /// implementing `IntoIterator` with the appropriate item type.
    ///
    /// Child processes will inherit environment variables from their parent process by
    /// default. Environment variables explicitly set using [`env`] take precedence
    /// over inherited variables. You can disable environment variable inheritance entirely
    /// using [`env_clear`] or for a single key using [`env_remove`].
    ///
    /// Note that environment variable names are case-insensitive (but
    /// case-preserving) on Windows and case-sensitive on all other platforms.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use std::collections::HashMap;
    /// use cargo_hyperlight::cargo;
    ///
    /// let mut envs = HashMap::new();
    /// envs.insert("CARGO_TARGET_DIR", "/path/to/target");
    /// envs.insert("CARGO_HOME", "/path/to/.cargo");
    ///
    /// cargo()
    ///     .unwrap()
    ///     .envs(&envs)
    ///     .arg("build")
    ///     .exec();
    /// ```
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .envs([
    ///         ("CARGO_TARGET_DIR", "/path/to/target"),
    ///         ("CARGO_HOME", "/path/to/.cargo"),
    ///     ])
    ///     .arg("build")
    ///     .exec();
    /// ```
    ///
    /// [`env`]: Command::env
    /// [`env_clear`]: Command::env_clear
    /// [`env_remove`]: Command::env_remove
    pub fn envs(
        &mut self,
        envs: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> &mut Self {
        for (k, v) in envs {
            self.env(k, v);
        }
        self
    }

    /// Returns an iterator over the arguments that will be passed to the cargo program.
    ///
    /// This does not include the program name itself (which can be retrieved with
    /// [`get_program`]).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// let mut command = cargo().unwrap();
    /// command.arg("build").arg("--release");
    ///
    /// let args: Vec<&std::ffi::OsStr> = command.get_args().collect();
    /// assert_eq!(args, &["build", "--release"]);
    /// ```
    ///
    /// [`get_program`]: Command::get_program
    pub fn get_args(&'_ self) -> impl Iterator<Item = &OsStr> {
        self.args.iter().map(|s| s.as_os_str())
    }

    /// Returns the working directory for the child process.
    ///
    /// This returns `None` if the working directory will not be changed from
    /// the current directory of the parent process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use cargo_hyperlight::cargo;
    ///
    /// let mut command = cargo().unwrap();
    /// assert_eq!(command.get_current_dir(), None);
    ///
    /// command.current_dir("/tmp");
    /// assert_eq!(command.get_current_dir(), Some(Path::new("/tmp")));
    /// ```
    pub fn get_current_dir(&self) -> Option<&Path> {
        self.current_dir.as_deref()
    }

    /// Returns an iterator over the environment mappings that will be set for the child process.
    ///
    /// Environment variables explicitly set or unset via [`env`], [`envs`], and
    /// [`env_remove`] can be retrieved with this method.
    ///
    /// Note that this output does not include environment variables inherited from the
    /// parent process.
    ///
    /// Each element is a tuple key/value where `None` means the variable is explicitly
    /// unset in the child process environment.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::ffi::OsStr;
    /// use cargo_hyperlight::cargo;
    ///
    /// let mut command = cargo().unwrap();
    /// command.env("CARGO_HOME", "/path/to/.cargo");
    /// command.env_remove("CARGO_TARGET_DIR");
    ///
    /// for (key, value) in command.get_envs() {
    ///     println!("{key:?} => {value:?}");
    /// }
    /// ```
    ///
    /// [`env`]: Command::env
    /// [`envs`]: Command::envs
    /// [`env_remove`]: Command::env_remove
    pub fn get_envs(&'_ self) -> impl Iterator<Item = (&OsStr, Option<&OsStr>)> {
        self.envs.iter().map(|(k, v)| (k.as_os_str(), v.as_deref()))
    }

    /// Returns the base environment variables for the command.
    ///
    /// This method returns the environment variables that will be inherited
    /// from the current process, taking into account whether [`env_clear`] has been called.
    ///
    /// [`env_clear`]: Command::env_clear
    fn base_env(&self) -> impl Iterator<Item = (OsString, OsString)> {
        env::vars_os().filter(|(k, _)| {
            if !self.inherit_envs {
                false
            } else if !self.inherit_cargo_envs {
                !is_cargo_env(k)
            } else {
                true
            }
        })
    }

    fn resolve_env(&self) -> HashMap<OsString, OsString> {
        merge_env(self.base_env(), self.get_envs())
    }

    fn command(&self) -> StdCommand {
        let mut command = self.cargo.command();
        command.args(self.get_args());
        if let Some(cwd) = &self.current_dir {
            command.current_dir(cwd);
        }
        if !self.inherit_envs {
            command.env_clear();
        }
        if !self.inherit_cargo_envs {
            for (k, _) in env::vars_os() {
                if is_cargo_env(&k) {
                    command.env_remove(k);
                }
            }
        }
        if let Some(rustup_toolchain) = &self.cargo.rustup_toolchain {
            command.env("RUSTUP_TOOLCHAIN", rustup_toolchain);
        }
        for (k, v) in self.get_envs() {
            match v {
                Some(v) => command.env(k, v),
                None => command.env_remove(k),
            };
        }
        command
    }

    /// Returns the path to the cargo program that will be executed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// let command = cargo().unwrap();
    /// println!("Program: {:?}", command.get_program());
    /// ```
    pub fn get_program(&self) -> &OsStr {
        self.cargo.path.as_os_str()
    }

    fn build_args(&self) -> Args {
        // parse the arguments and environment variables
        match Args::parse(
            self.get_args(),
            self.resolve_env(),
            self.get_current_dir(),
            Warning::WARN,
        ) {
            Ok(args) => args,
        }
    }

    fn build_args_infallible(&self) -> Args {
        match Args::parse(
            self.get_args(),
            self.resolve_env(),
            self.get_current_dir(),
            Warning::IGNORE,
        ) {
            Ok(args) => args,
            Err(err) => {
                eprintln!("Failed to parse arguments: {err}");
                std::process::exit(1);
            }
        }
    }

    /// Executes a cargo command as a child process, waiting for it to finish and
    /// collecting its exit status.
    ///
    /// The process stdin, stdout and stderr are inherited from the parent.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// let result = cargo()
    ///     .unwrap()
    ///     .arg("build")
    ///     .status();
    ///
    /// match result {
    ///     Ok(()) => println!("Cargo command succeeded"),
    ///     Err(e) => println!("Cargo command failed: {}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The sysroot preparation fails
    /// - The cargo process could not be spawned
    /// - The cargo process returned a non-zero exit status
    pub fn status(&self) -> anyhow::Result<()> {
        let args = self.build_args();

        args.prepare_sysroot()
            .context("Failed to prepare sysroot")?;

        self.command()
            .populate_from_args(&args)
            .checked_status()
            .context("Failed to execute cargo")?;
        Ok(())
    }

    /// Executes the cargo command, replacing the current process.
    ///
    /// This function will never return on success, as it replaces the current process
    /// with the cargo process. On error, it will print the error and exit with code 101.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use cargo_hyperlight::cargo;
    ///
    /// cargo()
    ///     .unwrap()
    ///     .arg("build")
    ///     .exec(); // This will never return
    /// ```
    ///
    /// # Errors
    ///
    /// This function will exit the process with code 101 if:
    /// - The sysroot preparation fails
    /// - The process replacement fails
    pub fn exec(&self) -> ! {
        match self.exec_impl() {
            Err(e) => {
                eprintln!("{e:?}");
                std::process::exit(101);
            }
        }
    }

    /// Internal implementation of process replacement.
    ///
    /// This method prepares the sysroot and then calls the low-level `exec` function
    /// to replace the current process.
    fn exec_impl(&self) -> anyhow::Result<Infallible> {
        let args = self.build_args();

        args.prepare_sysroot()
            .context("Failed to prepare sysroot")?;

        let mut command = self.command();
        command.populate_from_args(&args);

        if let Some(cwd) = self.get_current_dir() {
            env::set_current_dir(cwd).context("Failed to change current directory")?;
        }

        Ok(exec(
            command.get_program(),
            command.get_args(),
            command.resolve_env(self.base_env()),
        )?)
    }
}

/// Replaces the current process with the specified program using `execvpe`.
///
/// This function converts the provided arguments and environment variables into
/// the format expected by the `execvpe` system call and then replaces the current
/// process with the new program.
///
/// # Arguments
///
/// * `program` - The path to the program to execute
/// * `args` - The command-line arguments to pass to the program
/// * `envs` - The environment variables to set for the new process
///
/// # Returns
///
/// This function should never return on success. On failure, it returns an
/// `std::io::Error` describing what went wrong.
///
/// # Safety
///
/// This function uses unsafe code to call `libc::execvpe`. The implementation
/// carefully manages memory to ensure null-terminated strings are properly
/// constructed for the system call.
fn exec(
    program: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    envs: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
) -> std::io::Result<Infallible> {
    let mut env_bytes = vec![];
    let mut env_offsets = vec![];
    for (k, v) in envs.into_iter() {
        env_offsets.push(env_bytes.len());
        env_bytes.extend_from_slice(k.as_ref().as_encoded_bytes());
        env_bytes.push(b'=');
        env_bytes.extend_from_slice(v.as_ref().as_encoded_bytes());
        env_bytes.push(0);
    }
    let env_ptrs = env_offsets
        .into_iter()
        .map(|offset| env_bytes[offset..].as_ptr() as *const c_char)
        .chain(iter::once(std::ptr::null()))
        .collect::<Vec<_>>();

    let mut arg_bytes = vec![];
    let mut arg_offsets = vec![];

    arg_offsets.push(arg_bytes.len());
    arg_bytes.extend_from_slice(program.as_ref().as_encoded_bytes());
    arg_bytes.push(0);

    for arg in args {
        arg_offsets.push(arg_bytes.len());
        arg_bytes.extend_from_slice(arg.as_ref().as_encoded_bytes());
        arg_bytes.push(0);
    }
    let arg_ptrs = arg_offsets
        .into_iter()
        .map(|offset| arg_bytes[offset..].as_ptr() as *const c_char)
        .chain(iter::once(std::ptr::null()))
        .collect::<Vec<_>>();

    unsafe { libc::execvpe(arg_ptrs[0], arg_ptrs.as_ptr(), env_ptrs.as_ptr()) };

    Err(std::io::Error::last_os_error())
}

fn is_cargo_env(key: &OsStr) -> bool {
    key != "CARGO_HOME" && key.starts_with("CARGO_")
}
