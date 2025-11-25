//! Modular Schema System for Hiqlite
//!
//! This module provides a modular approach to database schema management,
//! allowing schemas to be organized by feature and conditionally loaded.

use anyhow::Result;
use async_trait::async_trait;
use hiqlite::Params;

pub mod workflow_schema;
pub mod worker_schema;
pub mod vm_schema;
pub mod tofu_schema;
pub mod execution_schema;

/// Trait for schema modules
#[async_trait]
pub trait SchemaModule: Send + Sync {
    /// Get the module name
    fn name(&self) -> &str;

    /// Get table creation statements
    fn table_definitions(&self) -> Vec<&'static str>;

    /// Get index creation statements
    fn index_definitions(&self) -> Vec<&'static str>;

    /// Get migration statements (ALTER TABLE, etc.)
    fn migrations(&self) -> Vec<&'static str>;

    /// Check if this schema module is enabled based on features
    fn is_enabled(&self) -> bool {
        true // By default, all modules are enabled
    }

    /// Apply the schema to the database
    async fn apply(&self, db: &crate::hiqlite_service::HiqliteService) -> Result<()> {
        if !self.is_enabled() {
            tracing::info!(module = self.name(), "Schema module disabled, skipping");
            return Ok(());
        }

        tracing::info!(module = self.name(), "Applying schema module");

        // Create tables
        for table_def in self.table_definitions() {
            db.execute(table_def, Params::default()).await?;
        }

        // Create indices
        for index_def in self.index_definitions() {
            let _ = db.execute(index_def, Params::default()).await; // Indices may already exist
        }

        // Apply migrations (these may fail if already applied)
        for migration in self.migrations() {
            let _ = db.execute(migration, Params::default()).await;
        }

        tracing::info!(module = self.name(), "Schema module applied successfully");
        Ok(())
    }
}

/// Schema registry that manages all schema modules
pub struct SchemaRegistry {
    modules: Vec<Box<dyn SchemaModule>>,
}

impl SchemaRegistry {
    /// Create a new schema registry with default modules
    pub fn new() -> Self {
        Self {
            modules: vec![
                Box::new(workflow_schema::WorkflowSchema),
                Box::new(worker_schema::WorkerSchema),
                Box::new(vm_schema::VmSchema),
                Box::new(tofu_schema::TofuSchema),
                Box::new(execution_schema::ExecutionSchema),
            ],
        }
    }

    /// Create a schema registry with only core modules
    pub fn core_only() -> Self {
        Self {
            modules: vec![
                Box::new(workflow_schema::WorkflowSchema),
                Box::new(worker_schema::WorkerSchema),
            ],
        }
    }

    /// Add a custom schema module
    pub fn add_module(mut self, module: Box<dyn SchemaModule>) -> Self {
        self.modules.push(module);
        self
    }

    /// Apply all enabled schema modules to the database
    pub async fn apply_all(&self, db: &crate::hiqlite_service::HiqliteService) -> Result<()> {
        tracing::info!("Applying {} schema modules", self.modules.len());

        for module in &self.modules {
            module.apply(db).await?;
        }

        tracing::info!("All schema modules applied successfully");
        Ok(())
    }

    /// Get list of enabled modules
    pub fn enabled_modules(&self) -> Vec<String> {
        self.modules
            .iter()
            .filter(|m| m.is_enabled())
            .map(|m| m.name().to_string())
            .collect()
    }

    /// Get total number of tables that will be created
    pub fn table_count(&self) -> usize {
        self.modules
            .iter()
            .filter(|m| m.is_enabled())
            .map(|m| m.table_definitions().len())
            .sum()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro to define a schema module
#[macro_export]
macro_rules! define_schema {
    (
        $name:ident,
        module_name: $module_name:expr,
        tables: [$($table:expr),* $(,)?],
        indices: [$($index:expr),* $(,)?],
        migrations: [$($migration:expr),* $(,)?]
    ) => {
        pub struct $name;

        #[async_trait::async_trait]
        impl $crate::storage::schemas::SchemaModule for $name {
            fn name(&self) -> &str {
                $module_name
            }

            fn table_definitions(&self) -> Vec<&'static str> {
                vec![$($table),*]
            }

            fn index_definitions(&self) -> Vec<&'static str> {
                vec![$($index),*]
            }

            fn migrations(&self) -> Vec<&'static str> {
                vec![$($migration),*]
            }
        }
    };
}