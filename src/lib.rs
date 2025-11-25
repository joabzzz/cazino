// Library exports for testing and HTTP API

#[cfg(any(feature = "server", feature = "wasm"))]
pub mod api;
#[cfg(feature = "server")]
pub mod cli;
pub mod db;
pub mod domain;
pub mod service;

// Re-export commonly used types
pub use service::CazinoService;
