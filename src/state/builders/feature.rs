// Feature Builder - Creates optional/feature-gated services
//
// Responsibilities:
// - Create VmService (if vm-backend feature enabled)
// - Create TofuService (if tofu-support feature enabled)

use std::sync::Arc;

use crate::adapters::ExecutionRegistry;
use crate::config::AppConfig;
use crate::hiqlite::HiqliteService;
use crate::infrastructure::vm::VmManagement;

#[cfg(feature = "vm-backend")]
use crate::domain::VmService;
#[cfg(feature = "tofu-support")]
use crate::domain::TofuService;

use super::components::FeatureComponents;

pub struct FeatureBuilder<'a> {
    config: &'a AppConfig,
    vm_manager: Option<Arc<dyn VmManagement>>,
    execution_registry: Arc<ExecutionRegistry>,
    hiqlite: Arc<HiqliteService>,
}

impl<'a> FeatureBuilder<'a> {
    pub fn new(
        config: &'a AppConfig,
        vm_manager: Option<Arc<dyn VmManagement>>,
        execution_registry: Arc<ExecutionRegistry>,
        hiqlite: Arc<HiqliteService>,
    ) -> Self {
        Self {
            config,
            vm_manager,
            execution_registry,
            hiqlite,
        }
    }

    /// Build feature-gated components
    pub fn build(self) -> FeatureComponents {
        #[cfg(feature = "vm-backend")]
        let vm_service = self
            .vm_manager
            .map(|vm_manager| Arc::new(VmService::new(vm_manager)));

        #[cfg(feature = "tofu-support")]
        let tofu_service = {
            use crate::tofu::{TofuStateBackend, TofuPlanExecutor, CliTofuExecutor};

            let state_backend = Arc::new(TofuStateBackend::new(self.hiqlite.clone()));
            let executor = Arc::new(CliTofuExecutor::new());
            let plan_executor = Arc::new(TofuPlanExecutor::new(
                self.hiqlite.clone(),
                executor,
                self.config.storage.work_dir.clone(),
            ));

            Some(Arc::new(TofuService::new(state_backend, plan_executor)))
        };

        FeatureComponents {
            #[cfg(feature = "vm-backend")]
            vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service,
        }
    }
}
