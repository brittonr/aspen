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

use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: git-remote-aspen <remote-name> <url>");
        eprintln!();
        eprintln!("This is a Git remote helper for Aspen Forge.");
        eprintln!("It should be invoked by Git, not directly.");
        std::process::exit(1);
    }

    let _remote_name = &args[1];
    let _url = &args[2];

    // Read commands from stdin
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            // Empty line signals end of command batch
            break;
        }

        match line {
            "capabilities" => {
                // Report supported capabilities
                writeln!(stdout, "fetch")?;
                writeln!(stdout, "push")?;
                writeln!(stdout, "option")?;
                writeln!(stdout)?; // Empty line terminates capabilities
            }
            "list" | "list for-push" => {
                // TODO: List refs from the Aspen repository
                // For now, return empty list
                writeln!(stdout)?;
            }
            cmd if cmd.starts_with("option ") => {
                // Handle options
                // Format: "option <name> <value>"
                writeln!(stdout, "unsupported")?;
            }
            cmd if cmd.starts_with("fetch ") => {
                // TODO: Implement fetch
                // Format: "fetch <sha1> <name>"
                eprintln!("git-remote-aspen: fetch not yet implemented");
                writeln!(stdout)?;
            }
            cmd if cmd.starts_with("push ") => {
                // TODO: Implement push
                // Format: "push <src>:<dst>"
                eprintln!("git-remote-aspen: push not yet implemented");
                writeln!(stdout)?;
            }
            _ => {
                eprintln!("git-remote-aspen: unknown command: {line}");
            }
        }

        stdout.flush()?;
    }

    Ok(())
}
