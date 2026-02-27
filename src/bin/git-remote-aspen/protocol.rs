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
    pub fn write_ref(&mut self, sha1: &str, ref_name: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{} {}", sha1, ref_name)
    }

    /// Write HEAD symref.
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

    // ========================================================================
    // Batch protocol tests — verify multi-ref fetch/push batching
    // ========================================================================

    #[test]
    fn test_read_fetch_batch() {
        let input = b"fetch aaa111 refs/heads/main\nfetch bbb222 refs/heads/dev\n\n";
        let mut reader = ProtocolReader::new(&input[..]);
        let batch = reader.read_batch().unwrap();

        assert_eq!(batch.len(), 2);
        match &batch[0] {
            Command::Fetch { sha1, ref_name } => {
                assert_eq!(sha1, "aaa111");
                assert_eq!(ref_name, "refs/heads/main");
            }
            _ => panic!("expected Fetch"),
        }
        match &batch[1] {
            Command::Fetch { sha1, ref_name } => {
                assert_eq!(sha1, "bbb222");
                assert_eq!(ref_name, "refs/heads/dev");
            }
            _ => panic!("expected Fetch"),
        }
    }

    #[test]
    fn test_read_push_batch() {
        let input = b"push refs/heads/main:refs/heads/main\npush refs/heads/dev:refs/heads/dev\n\n";
        let mut reader = ProtocolReader::new(&input[..]);
        let batch = reader.read_batch().unwrap();

        assert_eq!(batch.len(), 2);
        match &batch[0] {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "refs/heads/main");
                assert_eq!(dst, "refs/heads/main");
                assert!(!force);
            }
            _ => panic!("expected Push"),
        }
        match &batch[1] {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "refs/heads/dev");
                assert_eq!(dst, "refs/heads/dev");
                assert!(!force);
            }
            _ => panic!("expected Push"),
        }
    }

    #[test]
    fn test_parse_push_delete_ref() {
        // "push :refs/heads/old-branch" deletes the remote ref
        match Command::parse("push :refs/heads/old-branch") {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "");
                assert_eq!(dst, "refs/heads/old-branch");
                assert!(!force);
            }
            _ => panic!("expected Push with empty src (delete)"),
        }
    }

    #[test]
    fn test_parse_push_force_delete_ref() {
        // "push +:refs/heads/old-branch" force-deletes the remote ref
        match Command::parse("push +:refs/heads/old-branch") {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "");
                assert_eq!(dst, "refs/heads/old-branch");
                assert!(force);
            }
            _ => panic!("expected Push with force + empty src (force delete)"),
        }
    }

    #[test]
    fn test_read_mixed_batch_stops_at_empty() {
        // Fetch batch followed by more commands — batch stops at first blank line
        let input = b"fetch aaa111 refs/heads/main\n\ncapabilities\n";
        let mut reader = ProtocolReader::new(&input[..]);
        let batch = reader.read_batch().unwrap();

        assert_eq!(batch.len(), 1);
        // Next command after batch should be "capabilities"
        let next = reader.read_command().unwrap();
        assert!(matches!(next, Command::Capabilities));
    }

    #[test]
    fn test_read_single_fetch_batch() {
        // Single fetch followed by blank line is still a valid batch
        let input = b"fetch abc123 refs/heads/main\n\n";
        let mut reader = ProtocolReader::new(&input[..]);
        let batch = reader.read_batch().unwrap();

        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn test_read_empty_batch() {
        // Blank line immediately = empty batch
        let input = b"\n";
        let mut reader = ProtocolReader::new(&input[..]);
        let batch = reader.read_batch().unwrap();

        assert!(batch.is_empty());
    }

    // ========================================================================
    // Writer output format tests
    // ========================================================================

    #[test]
    fn test_write_capabilities_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_capabilities().unwrap();
        }
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("fetch\n"), "should list fetch capability");
        assert!(text.contains("push\n"), "should list push capability");
        assert!(text.contains("option\n"), "should list option capability");
        assert!(text.ends_with("\n\n"), "capabilities should end with blank line");
    }

    #[test]
    fn test_write_ref_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_ref("abc123", "refs/heads/main").unwrap();
        }
        let text = String::from_utf8(output).unwrap();
        assert_eq!(text, "abc123 refs/heads/main\n");
    }

    #[test]
    fn test_write_head_symref_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_head_symref("abc123", "refs/heads/main").unwrap();
        }
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "abc123 HEAD");
        assert_eq!(lines[1], "@refs/heads/main HEAD");
    }

    #[test]
    fn test_write_push_ok_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_push_ok("refs/heads/main").unwrap();
        }
        let text = String::from_utf8(output).unwrap();
        assert_eq!(text, "ok refs/heads/main\n");
    }

    #[test]
    fn test_write_push_error_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_push_error("refs/heads/main", "non-fast-forward").unwrap();
        }
        let text = String::from_utf8(output).unwrap();
        assert_eq!(text, "error refs/heads/main non-fast-forward\n");
    }

    #[test]
    fn test_write_option_supported() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_option_response(true).unwrap();
        }
        assert_eq!(String::from_utf8(output).unwrap(), "ok\n");
    }

    #[test]
    fn test_write_option_unsupported() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_option_response(false).unwrap();
        }
        assert_eq!(String::from_utf8(output).unwrap(), "unsupported\n");
    }

    #[test]
    fn test_write_end_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_end().unwrap();
        }
        assert_eq!(String::from_utf8(output).unwrap(), "\n");
    }

    #[test]
    fn test_write_fetch_done_format() {
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_fetch_done().unwrap();
        }
        assert_eq!(String::from_utf8(output).unwrap(), "\n");
    }

    // ========================================================================
    // Full session simulation tests
    // ========================================================================

    #[test]
    fn test_full_list_session() {
        // Simulates: capabilities → list → empty (done)
        let input = b"capabilities\nlist\n\n";
        let mut reader = ProtocolReader::new(&input[..]);
        let mut output = Vec::new();

        // capabilities
        let cmd = reader.read_command().unwrap();
        assert!(matches!(cmd, Command::Capabilities));
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_capabilities().unwrap();
        }

        // list
        let cmd = reader.read_command().unwrap();
        assert!(matches!(cmd, Command::List { for_push: false }));
        {
            let mut writer = ProtocolWriter::new(&mut output);
            writer.write_ref("aaa111", "refs/heads/main").unwrap();
            writer.write_ref("bbb222", "refs/heads/dev").unwrap();
            writer.write_end().unwrap();
        }

        // empty = done
        let cmd = reader.read_command().unwrap();
        assert!(matches!(cmd, Command::Empty));

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("aaa111 refs/heads/main"));
        assert!(text.contains("bbb222 refs/heads/dev"));
    }

    #[test]
    fn test_full_push_session_multi_ref() {
        // Simulates: capabilities → list for-push → push batch → empty (done)
        let input = b"capabilities\nlist for-push\npush refs/heads/main:refs/heads/main\npush +refs/heads/dev:refs/heads/dev\n\n\n";
        let mut reader = ProtocolReader::new(&input[..]);

        let cmd = reader.read_command().unwrap();
        assert!(matches!(cmd, Command::Capabilities));

        let cmd = reader.read_command().unwrap();
        assert!(matches!(cmd, Command::List { for_push: true }));

        // Read push batch
        let batch = reader.read_batch().unwrap();
        assert_eq!(batch.len(), 2);
        match &batch[0] {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "refs/heads/main");
                assert_eq!(dst, "refs/heads/main");
                assert!(!force);
            }
            _ => panic!("expected Push"),
        }
        match &batch[1] {
            Command::Push { src, dst, force } => {
                assert_eq!(src, "refs/heads/dev");
                assert_eq!(dst, "refs/heads/dev");
                assert!(*force);
            }
            _ => panic!("expected force Push"),
        }
    }

    #[test]
    fn test_parse_option_with_no_value() {
        match Command::parse("option progress") {
            Command::Option { name, value } => {
                assert_eq!(name, "progress");
                assert_eq!(value, "");
            }
            _ => panic!("expected Option with empty value"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        match Command::parse("import abc123") {
            Command::Unknown(line) => assert_eq!(line, "import abc123"),
            _ => panic!("expected Unknown"),
        }
    }

    #[test]
    fn test_reader_eof_returns_empty() {
        let input = b"";
        let mut reader = ProtocolReader::new(&input[..]);
        let cmd = reader.read_command().unwrap();
        assert!(matches!(cmd, Command::Empty));
    }

    #[test]
    fn test_write_ref_list_with_head() {
        // Simulate a full ref listing with HEAD symref
        let mut output = Vec::new();
        {
            let mut writer = ProtocolWriter::new(&mut output);
            let sha1 = "a".repeat(40);
            writer.write_head_symref(&sha1, "refs/heads/main").unwrap();
            writer.write_ref(&sha1, "refs/heads/main").unwrap();
            writer.write_ref(&"b".repeat(40), "refs/heads/dev").unwrap();
            writer.write_end().unwrap();
        }
        let text = String::from_utf8(output).unwrap();
        // Should contain HEAD, symref, two refs, then blank line terminator
        assert!(text.contains(&format!("{} HEAD", "a".repeat(40))));
        assert!(text.contains("@refs/heads/main HEAD"));
        assert!(text.contains(&format!("{} refs/heads/main", "a".repeat(40))));
        assert!(text.contains(&format!("{} refs/heads/dev", "b".repeat(40))));
        assert!(text.ends_with('\n'), "listing should end with newline");
    }
}
