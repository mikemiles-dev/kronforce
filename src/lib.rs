//! Kronforce: a workload automation and job scheduling engine with distributed agents.
//!
//! This crate provides the core library for both the controller and agent binaries,
//! including job scheduling, execution dispatch, agent communication, and the REST API.

pub mod agent;
pub mod api;
pub mod config;
pub mod crypto;
pub mod dag;
pub mod db;
pub mod error;
pub mod executor;
pub mod mcp_server;
pub mod scheduler;
pub mod tls;
