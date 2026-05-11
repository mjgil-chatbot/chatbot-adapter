//! Core types for the agent adapter.
//!
//! This crate defines the canonical event model, request types, and adapter traits
//! used across all transports and providers.

mod adapter;
mod audit;
mod capabilities;
mod errors;
mod events;
mod request;
pub mod types;
