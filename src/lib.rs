#![forbid(unsafe_code)]

//! # graceful
//!
//! Graceful shutdown for Rust services — signal handling, connection draining,
//! and RAII cleanup guards.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use graceful::{ShutdownGuard, shutdown_signal};
//!
//! #[tokio::main]
//! async fn main() {
//!     let guard = ShutdownGuard::new();
//!
//!     // Spawn a task that respects shutdown
//!     let g = guard.clone();
//!     tokio::spawn(async move {
//!         loop {
//!             if g.is_shutdown() {
//!                 break;
//!             }
//!             // Do work...
//!             tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//!         }
//!     });
//!
//!     // Wait for shutdown signal
//!     shutdown_signal().await;
//!     drop(guard); // Triggers cleanup for all associated guards
//! }
//! ```
//!
//! ## Axum Integration
//!
//! ```rust,no_run
//! use axum::Router;
//! use graceful::{ShutdownGuard, shutdown_signal, ShutdownConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let guard = ShutdownGuard::new();
//!     let config = ShutdownConfig::builder()
//!         .drain_timeout(std::time::Duration::from_secs(30))
//!         .build();
//!
//!     let app = Router::new();
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!
//!     let server = axum::serve(listener, app)
//!         .with_graceful_shutdown(shutdown_signal());
//!
//!     tokio::select! {
//!         result = server => {
//!             if let Err(e) = result {
//!                 eprintln!("server error: {}", e);
//!             }
//!         }
//!         _ = shutdown_signal() => {
//!             tracing::info!("shutting down...");
//!         }
//!     }
//!
//!     // Wait for drain to complete
//!     drop(guard);
//! }
//! ```

mod config;
mod error;
mod guard;
mod signal;

pub use config::ShutdownConfig;
pub use error::ShutdownError;
pub use guard::ShutdownGuard;
pub use signal::shutdown_signal;
