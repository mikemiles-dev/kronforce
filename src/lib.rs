//! Kronforce: a workload automation and job scheduling engine with distributed agents.
//!
//! This crate provides the core library for both the controller and agent binaries,
//! including job scheduling, execution dispatch, agent communication, and the REST API.

pub mod agent;
pub mod api;
pub mod config;
pub mod dag;
pub mod db;
pub mod error;
pub mod executor;
pub mod scheduler;

// Re-exports: these modules moved into their parent directories
// but are re-exported here so existing `crate::X` imports still work.
pub use agent::protocol;
pub use db::models;
pub use executor::notifications;
pub use executor::output_rules;
pub use executor::scripts;
pub use scheduler::cron_parser;
