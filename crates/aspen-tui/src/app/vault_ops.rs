//! Vault browser operations.

use tracing::warn;

use super::state::App;

impl App {
    /// Enter a vault (drill down).
    pub(crate) async fn enter_vault(&mut self) {
        if self.active_vault.is_none() && !self.vaults.is_empty() {
            if let Some(vault) = self.vaults.get(self.selected_vault) {
                let vault_name = vault.name.clone();
                self.active_vault = Some(vault_name.clone());
                self.selected_vault_key = 0;
                self.refresh_vault_keys(&vault_name).await;
            }
        }
    }

    /// Exit vault (go back to vault list).
    pub(crate) fn exit_vault(&mut self) {
        if self.active_vault.is_some() {
            self.active_vault = None;
            self.vault_keys.clear();
            self.selected_vault_key = 0;
        } else {
            // If not in a vault, Esc quits
            self.should_quit = true;
        }
    }

    /// Refresh the list of vaults.
    pub(crate) async fn refresh_vaults(&mut self) {
        match self.client.list_vaults().await {
            Ok(vaults) => {
                self.vaults = vaults;
                // Reset selection if out of bounds
                if self.selected_vault >= self.vaults.len() && !self.vaults.is_empty() {
                    self.selected_vault = self.vaults.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to list vaults");
                self.set_status(&format!("Failed to list vaults: {}", e));
            }
        }
    }

    /// Refresh keys for a specific vault.
    async fn refresh_vault_keys(&mut self, vault: &str) {
        match self.client.list_vault_keys(vault).await {
            Ok(keys) => {
                self.vault_keys = keys;
                // Reset selection if out of bounds
                if self.selected_vault_key >= self.vault_keys.len() && !self.vault_keys.is_empty() {
                    self.selected_vault_key = self.vault_keys.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, vault = vault, "failed to list vault keys");
                self.set_status(&format!("Failed to list vault keys: {}", e));
            }
        }
    }
}
