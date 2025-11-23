// Library exports for testing and HTTP API

// Core domain logic - always available
pub mod domain;

// Database abstraction - always available (trait + error types)
pub mod db;

// Optional: Axum API server (requires "server" feature)
#[cfg(feature = "server")]
pub mod api;

// Optional: CLI (requires "server" feature)
#[cfg(feature = "server")]
pub mod cli;

// Optional: Service layer (requires "server" feature)
#[cfg(feature = "server")]
pub mod service;

// Re-export commonly used types
#[cfg(feature = "server")]
pub use service::CazinoService;
