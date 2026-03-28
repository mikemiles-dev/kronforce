//! Remote agent communication layer.
//!
//! Defines the protocol messages exchanged between controller and agents,
//! the HTTP client used by the controller to dispatch jobs, and the HTTP
//! server that runs on each agent to receive and execute work.

pub mod client;
pub mod protocol;
pub mod server;

pub use client::AgentClient;
pub use server::{AgentState, router as agent_router};
