//! Git remote helper protocol implementation.
//!
//! Implements the stdin/stdout protocol described in git-remote-helpers(7).
//! Commands are read from stdin, responses written to stdout.

use std::io::BufRead;
use std::io::Write;

/// A parsed command from the git remote helper protocol.
#[derive(Debug, Clone)]
pub enum Command {
    /// Report supported capabilities.
    Capabilities,

    /// List refs (for fetch).
    List {
        /// True if listing for push (shows push capabilities).
        for_push: bool,
    },

    /// Fetch objects for a ref.
    Fetch {
        /// SHA-1 hash of object to fetch.
        sha1: String,
        /// Ref name.
        ref_name: String,
    },

    /// Push a ref.
    Push {
        /// Source ref (local).
        src: String,
        /// Destination ref (remote).
        dst: String,
        /// Force push.
        force: bool,
    },

    /// Set an option.
    Option {
        /// Option name.
        name: String,
        /// Option value.
        value: String,
    },

    /// Empty line (end of batch or command sequence).
    Empty,

    /// Unknown command.
    Unknown(String),
}

impl Command {
    /// Parse a command from a line of input.
    pub fn parse(line: &str) -> Self {
        let line = line.trim();

        if line.is_empty() {
            return Self::Empty;
        }

        if line == "capabilities" {
            return Self::Capabilities;
        }

        if line == "list" {
            return Self::List { for_push: false };
        }

        if line == "list for-push" {
            return Self::List { for_push: true };
        }

        if let Some(rest) = line.strip_prefix("option ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Self::Option {
                    name: parts[0].to_string(),
                    value: parts[1].to_string(),
                };
            } else if parts.len() == 1 {
                return Self::Option {
                    name: parts[0].to_string(),
                    value: String::new(),
                };
            }
        }

        if let Some(rest) = line.strip_prefix("fetch ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                return Self::Fetch {
                    sha1: parts[0].to_string(),
                    ref_name: parts[1].to_string(),
                };
            }
        }

        if let Some(rest) = line.strip_prefix("push ") {
            let force = rest.starts_with('+');
            let spec = if force { &rest[1..] } else { rest };

            let parts: Vec<&str> = spec.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Self::Push {
                    src: parts[0].to_string(),
                    dst: parts[1].to_string(),
                    force,
                };
            }
        }

        Self::Unknown(line.to_string())
    }
}

/// Capabilities supported by this remote helper.
pub const CAPABILITIES: &[&str] = &[
    "fetch",  // Can fetch refs and objects
    "push",   // Can push refs and objects
    "option", // Supports transport options
];

/// Response writer for the git remote helper protocol.
pub struct ProtocolWriter<W: Write> {
    writer: W,
}

impl<W: Write> ProtocolWriter<W> {
    /// Create a new protocol writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write capabilities response.
    pub fn write_capabilities(&mut self) -> std::io::Result<()> {
        for cap in CAPABILITIES {
            writeln!(self.writer, "{}", cap)?;
        }
        writeln!(self.writer)?; // Empty line terminates
        self.writer.flush()
    }

    /// Write a ref listing.
    #[allow(dead_code)]
    pub fn write_ref(&mut self, sha1: &str, ref_name: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{} {}", sha1, ref_name)
    }

    /// Write HEAD symref.
    #[allow(dead_code)]
    pub fn write_head_symref(&mut self, sha1: &str, target: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{} HEAD", sha1)?;
        writeln!(self.writer, "@{} HEAD", target)
    }

    /// Write empty line (end of list).
    pub fn write_end(&mut self) -> std::io::Result<()> {
        writeln!(self.writer)?;
        self.writer.flush()
    }

    /// Write option response.
    pub fn write_option_response(&mut self, supported: bool) -> std::io::Result<()> {
        if supported {
            writeln!(self.writer, "ok")?;
        } else {
            writeln!(self.writer, "unsupported")?;
        }
        self.writer.flush()
    }

    /// Write fetch response (after objects have been written).
    pub fn write_fetch_done(&mut self) -> std::io::Result<()> {
        writeln!(self.writer)?;
        self.writer.flush()
    }

    /// Write push response.
    #[allow(dead_code)]
    pub fn write_push_ok(&mut self, ref_name: &str) -> std::io::Result<()> {
        writeln!(self.writer, "ok {}", ref_name)?;
        self.writer.flush()
    }

    /// Write push error.
    pub fn write_push_error(&mut self, ref_name: &str, message: &str) -> std::io::Result<()> {
        writeln!(self.writer, "error {} {}", ref_name, message)?;
        self.writer.flush()
    }

    /// Write an error message to stderr.
    pub fn write_error(&mut self, message: &str) -> std::io::Result<()> {
        eprintln!("git-remote-aspen: {}", message);
        Ok(())
    }
}

/// Protocol reader for the git remote helper protocol.
pub struct ProtocolReader<R: BufRead> {
    reader: R,
    line_buffer: String,
}

impl<R: BufRead> ProtocolReader<R> {
    /// Create a new protocol reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buffer: String::new(),
        }
    }

    /// Read the next command.
    pub fn read_command(&mut self) -> std::io::Result<Command> {
        self.line_buffer.clear();
        let bytes_read = self.reader.read_line(&mut self.line_buffer)?;

        if bytes_read == 0 {
            // EOF
            return Ok(Command::Empty);
        }

        Ok(Command::parse(&self.line_buffer))
    }

    /// Read commands until an empty line.
    #[allow(dead_code)]
    pub fn read_batch(&mut self) -> std::io::Result<Vec<Command>> {
        let mut commands = Vec::new();

        loop {
            let cmd = self.read_command()?;
            match cmd {
                Command::Empty => break,
                _ => commands.push(cmd),
            }
        }

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capabilities() {
        assert!(matches!(Command::parse("capabilities"), Command::Capabilities));
    }

    #[test]
    fn test_parse_list() {
        match Command::parse("list") {
            Command::List { for_push } => assert!(!for_push),
            _ => panic!("expected List"),
        }

        match Command::parse("list for-push") {
            Command::List { for_push } => assert!(for_push),
            _ => panic!("expected List for-push"),
        }
    }

    #[test]
    fn test_parse_fetch() {
        match Command::parse("fetch abc123 refs/heads/main") {
            Command::Fetch { sha1, ref_name } => {
                assert_eq!(sha1, "abc123");
                assert_eq!(ref_name, "refs/heads/main");
            }
            _ => panic!("expected Fetch"),
        }
    }

    #[test]
    fn test_parse_push() {
        match Command::parse("push refs/heads/main:refs/heads/main") {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "refs/heads/main");
                assert_eq!(dst, "refs/heads/main");
                assert!(!force);
            }
            _ => panic!("expected Push"),
        }
    }

    #[test]
    fn test_parse_force_push() {
        match Command::parse("push +refs/heads/main:refs/heads/main") {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "refs/heads/main");
                assert_eq!(dst, "refs/heads/main");
                assert!(force);
            }
            _ => panic!("expected Push with force"),
        }
    }

    #[test]
    fn test_parse_option() {
        match Command::parse("option verbosity 1") {
            Command::Option { name, value } => {
                assert_eq!(name, "verbosity");
                assert_eq!(value, "1");
            }
            _ => panic!("expected Option"),
        }
    }

    #[test]
    fn test_parse_empty() {
        assert!(matches!(Command::parse(""), Command::Empty));
        assert!(matches!(Command::parse("  "), Command::Empty));
    }
}
