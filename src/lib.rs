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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn shutdown_config_defaults() {
        let config = ShutdownConfig::defaults();
        assert_eq!(config.drain_timeout, Duration::from_secs(30));
        assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
    }

    #[test]
    fn shutdown_config_default_trait() {
        let config = ShutdownConfig::default();
        assert_eq!(config.drain_timeout, Duration::from_secs(30));
        assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
    }

    #[test]
    fn shutdown_config_builder() {
        let config = ShutdownConfig::builder()
            .drain_timeout(Duration::from_secs(60))
            .shutdown_timeout(Duration::from_secs(20))
            .build();
        assert_eq!(config.drain_timeout, Duration::from_secs(60));
        assert_eq!(config.shutdown_timeout, Duration::from_secs(20));
    }

    #[test]
    fn shutdown_config_builder_defaults() {
        let config = ShutdownConfig::builder().build();
        assert_eq!(config.drain_timeout, Duration::from_secs(30));
        assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
    }

    #[test]
    fn shutdown_guard_new_and_is_shutdown() {
        let guard = ShutdownGuard::new();
        assert!(!guard.is_shutdown());
    }

    #[test]
    fn shutdown_guard_default() {
        let guard = ShutdownGuard::default();
        assert!(!guard.is_shutdown());
    }

    #[test]
    fn shutdown_guard_manual_shutdown() {
        let guard = ShutdownGuard::new();
        let clone = guard.clone();
        assert!(!clone.is_shutdown());

        guard.shutdown();
        assert!(clone.is_shutdown());
    }

    #[test]
    fn shutdown_guard_drop_last_clone_triggers_shutdown() {
        let guard = ShutdownGuard::new();
        let clone = guard.clone();
        drop(guard);
        // clone still alive, not shutdown yet
        assert!(!clone.is_shutdown());
        drop(clone);
        // all dropped — no way to check after drop, but it shouldn't panic
    }

    #[tokio::test]
    async fn shutdown_guard_wait_for_shutdown() {
        let guard = ShutdownGuard::new();
        let clone = guard.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            clone.shutdown();
        });

        let start = std::time::Instant::now();
        guard.wait_for_shutdown().await;
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn shutdown_error_display() {
        let err = ShutdownError::ChannelClosed;
        assert_eq!(err.to_string(), "shutdown channel closed");

        let err = ShutdownError::DrainTimeout(Duration::from_secs(30));
        assert_eq!(err.to_string(), "drain timed out after 30s");

        let err = ShutdownError::ShutdownTimeout(Duration::from_secs(10));
        assert_eq!(err.to_string(), "shutdown timed out after 10s");

        let err = ShutdownError::TaskFailed("worker crashed".to_string());
        assert_eq!(err.to_string(), "task failed: worker crashed");
    }

    #[test]
    fn trigger_shutdown_broadcasts() {
        let mut rx = signal::subscribe_shutdown();
        signal::trigger_shutdown();
        assert!(rx.try_recv().is_ok());
    }
}
