//! VM Schema Module
//!
//! Schema for VM registry and events (only enabled when VM backend is used)

use crate::storage::schemas::SchemaModule;
use async_trait::async_trait;

pub struct VmSchema;

#[async_trait]
impl SchemaModule for VmSchema {
    fn name(&self) -> &str {
        "vms"
    }

    fn table_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE TABLE IF NOT EXISTS vms (
                id TEXT PRIMARY KEY,
                config TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                pid INTEGER,
                control_socket TEXT,
                job_dir TEXT,
                ip_address TEXT,
                metrics TEXT
            )",
            "CREATE TABLE IF NOT EXISTS vm_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vm_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT,
                timestamp INTEGER NOT NULL,
                node_id TEXT,
                FOREIGN KEY (vm_id) REFERENCES vms(id)
            )",
        ]
    }

    fn index_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_vms_state ON vms(state)",
            "CREATE INDEX IF NOT EXISTS idx_vms_node_id ON vms(node_id)",
            "CREATE INDEX IF NOT EXISTS idx_vm_events_vm_id ON vm_events(vm_id)",
            "CREATE INDEX IF NOT EXISTS idx_vm_events_timestamp ON vm_events(timestamp)",
        ]
    }

    fn migrations(&self) -> Vec<&'static str> {
        vec![]
    }

    fn is_enabled(&self) -> bool {
        // Check if VM backend feature is enabled
        #[cfg(feature = "vm-backend")]
        {
            true
        }
        #[cfg(not(feature = "vm-backend"))]
        {
            // Default to true for now until feature flags are implemented
            true
        }
    }
}