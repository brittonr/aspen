/// Explicit process/SSH capability. Arguments are passed directly, never through a shell.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub program: std::path::PathBuf,
    pub args: Vec<String>,
    pub timeout_ms: u64,
}

impl super::Transport for Capability {
    fn exchange(
        &mut self,
        request: &release_channel_index::model::SignedRequest,
    ) -> Result<release_channel_index::model::SignedReply, String> {
        let mut command = std::process::Command::new(&self.program);
        command.args(&self.args);
        release_channel_index::client::exchange(
            &mut command,
            request,
            std::time::Duration::from_millis(self.timeout_ms),
        )
    }
}
