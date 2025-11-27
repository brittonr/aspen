use std::sync::Arc;

use flawless_utils::DeployedModule;

use crate::config::AuthConfig;

/// Configuration state container
///
/// Contains static configuration loaded at startup. This is immutable
/// after initialization, making it fundamentally different from services.
///
/// # Usage
///
/// ```rust
/// // In authentication middleware
/// pub async fn auth_middleware(
///     State(config): State<ConfigState>,
///     request: Request,
/// ) -> Result<Request> {
///     let auth = config.auth();
///     // ...
/// }
/// ```
#[derive(Clone)]
pub struct ConfigState {
    auth: Arc<AuthConfig>,
    module: Option<Arc<DeployedModule>>,
}

impl ConfigState {
    /// Create new configuration state
    pub fn new(auth: Arc<AuthConfig>, module: Option<DeployedModule>) -> Self {
        Self {
            auth,
            module: module.map(Arc::new),
        }
    }

    /// Get authentication configuration
    pub fn auth(&self) -> Arc<AuthConfig> {
        self.auth.clone()
    }

    /// Get deployed module (if available)
    pub fn module(&self) -> Option<&DeployedModule> {
        self.module.as_ref().map(|m| m.as_ref())
    }
}
