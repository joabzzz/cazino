pub mod models;

#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "server")]
pub mod websocket;

#[cfg(feature = "server")]
pub use server::run_server;
