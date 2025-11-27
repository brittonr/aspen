use std::sync::Arc;

#[cfg(feature = "tofu-support")]
use crate::domain::TofuService;
#[cfg(feature = "vm-backend")]
use crate::domain::VmService;

/// Optional features state container
///
/// Contains services for feature-gated functionality. All services are
/// Option<Arc<T>> because they may not be available depending on:
/// - Compile-time feature flags
/// - Runtime configuration
/// - Infrastructure availability (e.g., VM manager failed to initialize)
///
/// # Usage
///
/// ```rust
/// #[cfg(feature = "vm-backend")]
/// pub async fn vm_handler(
///     State(features): State<FeaturesState>,
/// ) -> Result<Response> {
///     let vm_service = features.vm_service()
///         .ok_or(ServiceUnavailable)?;
///     // ...
/// }
/// ```
#[derive(Clone)]
pub struct FeaturesState {
    #[cfg(feature = "vm-backend")]
    vm_service: Option<Arc<VmService>>,

    #[cfg(feature = "tofu-support")]
    tofu_service: Option<Arc<TofuService>>,
}

impl FeaturesState {
    /// Create new features state
    pub fn new(
        #[cfg(feature = "vm-backend")] vm_service: Option<Arc<VmService>>,
        #[cfg(feature = "tofu-support")] tofu_service: Option<Arc<TofuService>>,
    ) -> Self {
        Self {
            #[cfg(feature = "vm-backend")]
            vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service,
        }
    }

    #[cfg(feature = "vm-backend")]
    /// Get VM service (if available)
    pub fn vm_service(&self) -> Option<Arc<VmService>> {
        self.vm_service.clone()
    }

    #[cfg(feature = "tofu-support")]
    /// Get Tofu service (if available)
    pub fn tofu_service(&self) -> Option<Arc<TofuService>> {
        self.tofu_service.clone()
    }
}

impl Default for FeaturesState {
    fn default() -> Self {
        Self {
            #[cfg(feature = "vm-backend")]
            vm_service: None,
            #[cfg(feature = "tofu-support")]
            tofu_service: None,
        }
    }
}
