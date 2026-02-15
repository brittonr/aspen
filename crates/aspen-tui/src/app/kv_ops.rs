//! Key-value store operations.

use super::state::App;

impl App {
    /// Execute key-value operation based on input.
    pub(crate) async fn execute_kv_operation(&mut self) {
        let input = self.input_buffer.trim().to_string();

        if let Some(key) = input.strip_prefix("get ") {
            self.read_key(key).await;
        } else if let Some(rest) = input.strip_prefix("set ") {
            if let Some((key, value)) = rest.split_once(' ') {
                self.write_key(key, value.as_bytes().to_vec()).await;
            } else {
                self.set_status("Usage: set <key> <value>");
            }
        } else {
            self.set_status("Commands: get <key> | set <key> <value>");
        }

        self.input_buffer.clear();
    }

    /// Read a key from the cluster.
    async fn read_key(&mut self, key: &str) {
        match self.client.read(key.to_string()).await {
            Ok(value) => {
                self.last_read_result = Some((key.to_string(), value));
                self.set_status(&format!("Read key '{}'", key));
            }
            Err(e) => {
                self.set_status(&format!("Read failed: {}", e));
            }
        }
    }

    /// Write a key-value pair to the cluster.
    async fn write_key(&mut self, key: &str, value: Vec<u8>) {
        match self.client.write(key.to_string(), value).await {
            Ok(_) => {
                self.set_status(&format!("Written key '{}'", key));
            }
            Err(e) => {
                self.set_status(&format!("Write failed: {}", e));
            }
        }
    }
}
