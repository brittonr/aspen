//! Storage Module
//!
//! This module provides abstractions and utilities for data storage,
//! including modular schema management for Hiqlite.

pub mod schemas;

// Re-export key types
pub use schemas::{SchemaModule, SchemaRegistry};