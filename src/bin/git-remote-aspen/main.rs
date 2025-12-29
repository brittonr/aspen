//! Git remote helper for Aspen Forge.
//!
//! This binary implements the Git remote helper protocol, enabling standard
//! git commands to work with Aspen Forge repositories.
//!
//! ## Usage
//!
//! Git invokes this helper automatically for URLs with the `aspen://` scheme:
//!
//! ```bash
//! git clone aspen://<ticket>/<repo_id> my-repo
//! git fetch aspen
//! git push aspen main
//! ```
//!
//! The helper reads commands from stdin and writes responses to stdout,
//! following the protocol described in `git-remote-helpers(7)`.
//!
//! ## URL Formats
//!
//! - `aspen://<ticket>/<repo_id>` - Connect via cluster ticket
//! - `aspen://<node_id>/<repo_id>` - Direct connection to a node
//!
//! ## Capabilities
//!
//! - `fetch` - Pull refs and objects from Forge
//! - `push` - Push refs and objects to Forge
//! - `option` - Configure transport options

mod protocol;
mod url;

use std::io::{self, Write};

use protocol::{Command, ProtocolReader, ProtocolWriter};
use url::AspenUrl;

/// Supported options and their current values.
#[allow(dead_code)]
struct Options {
    /// Verbosity level (0 = quiet, 1 = normal, 2+ = verbose).
    verbosity: u32,
    /// Progress reporting enabled.
    progress: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbosity: 1,
            progress: true,
        }
    }
}

/// Remote helper state.
#[allow(dead_code)]
struct RemoteHelper {
    /// Remote name (e.g., "origin").
    remote_name: String,
    /// Parsed URL.
    url: AspenUrl,
    /// Current options.
    options: Options,
}

impl RemoteHelper {
    /// Create a new remote helper.
    fn new(remote_name: String, url_str: &str) -> io::Result<Self> {
        let url = AspenUrl::parse(url_str).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("invalid URL: {e}"))
        })?;

        Ok(Self {
            remote_name,
            url,
            options: Options::default(),
        })
    }

    /// Run the remote helper protocol loop.
    fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();

        let mut reader = ProtocolReader::new(stdin.lock());
        let mut writer = ProtocolWriter::new(stdout.lock());

        loop {
            let cmd = reader.read_command()?;

            match cmd {
                Command::Capabilities => {
                    writer.write_capabilities()?;
                }

                Command::List { for_push } => {
                    self.handle_list(&mut writer, for_push)?;
                }

                Command::Fetch { sha1, ref_name } => {
                    self.handle_fetch(&mut writer, &sha1, &ref_name)?;
                }

                Command::Push { src, dst, force } => {
                    self.handle_push(&mut writer, &src, &dst, force)?;
                }

                Command::Option { name, value } => {
                    self.handle_option(&mut writer, &name, &value)?;
                }

                Command::Empty => {
                    // End of commands
                    break;
                }

                Command::Unknown(line) => {
                    writer.write_error(&format!("unknown command: {line}"))?;
                }
            }
        }

        Ok(())
    }

    /// Handle the "list" command.
    fn handle_list<W: Write>(
        &self,
        writer: &mut ProtocolWriter<W>,
        _for_push: bool,
    ) -> io::Result<()> {
        // TODO: Connect to Aspen cluster and list refs
        // For now, return empty list
        //
        // When implemented:
        // 1. Connect to cluster using self.url.target
        // 2. Call exporter.list_refs(&self.url.repo_id)
        // 3. Write each ref with writer.write_ref(sha1, name)

        if self.options.verbosity > 1 {
            eprintln!(
                "git-remote-aspen: listing refs for {}",
                self.url.repo_id().to_hex()
            );
        }

        writer.write_end()
    }

    /// Handle the "fetch" command.
    fn handle_fetch<W: Write>(
        &self,
        writer: &mut ProtocolWriter<W>,
        sha1: &str,
        ref_name: &str,
    ) -> io::Result<()> {
        // TODO: Implement fetch
        // 1. Connect to Aspen cluster
        // 2. Export objects from Forge to git format
        // 3. Write objects to git (via fast-import or direct pack)

        if self.options.verbosity > 0 {
            eprintln!(
                "git-remote-aspen: fetch {} {} (not yet implemented)",
                sha1, ref_name
            );
        }

        writer.write_fetch_done()
    }

    /// Handle the "push" command.
    fn handle_push<W: Write>(
        &self,
        writer: &mut ProtocolWriter<W>,
        src: &str,
        dst: &str,
        force: bool,
    ) -> io::Result<()> {
        // TODO: Implement push
        // 1. Read objects from local git
        // 2. Connect to Aspen cluster
        // 3. Import objects into Forge
        // 4. Update ref

        if self.options.verbosity > 0 {
            let force_str = if force { " (force)" } else { "" };
            eprintln!(
                "git-remote-aspen: push {}:{}{} (not yet implemented)",
                src, dst, force_str
            );
        }

        writer.write_push_error(dst, "push not yet implemented")
    }

    /// Handle the "option" command.
    fn handle_option<W: Write>(
        &self,
        writer: &mut ProtocolWriter<W>,
        name: &str,
        value: &str,
    ) -> io::Result<()> {
        let supported = match name {
            "verbosity" => {
                // We accept verbosity but store it (would need &mut self)
                value.parse::<u32>().is_ok()
            }
            "progress" => value == "true" || value == "false",
            _ => false,
        };

        writer.write_option_response(supported)
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: git-remote-aspen <remote-name> <url>");
        eprintln!();
        eprintln!("This is a Git remote helper for Aspen Forge.");
        eprintln!("It should be invoked by Git, not directly.");
        eprintln!();
        eprintln!("URL formats:");
        eprintln!("  aspen://<ticket>/<repo_id>  - Connect via cluster ticket");
        eprintln!("  aspen://<node_id>/<repo_id> - Direct node connection");
        std::process::exit(1);
    }

    let remote_name = &args[1];
    let url = &args[2];

    let mut helper = RemoteHelper::new(remote_name.clone(), url)?;
    helper.run()
}
