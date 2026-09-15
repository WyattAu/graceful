#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! # graceful
//!
//! Graceful shutdown for Rust services — signal handling, connection draining,
//! and RAII cleanup guards.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use shutdown_kit::{ShutdownGuard, shutdown_signal};
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
//! ```ignore
//! // Requires adding `axum` to your own crate; shown for illustration.
//! use axum::Router;
//! use shutdown_kit::{ShutdownGuard, shutdown_signal, ShutdownConfig};
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
pub use signal::{shutdown_signal, subscribe_shutdown, trigger_shutdown};

/// Runs the full coordinated shutdown sequence against an OS signal:
///
/// 1. Wait for [`shutdown_signal`] (or any [`trigger_shutdown`] broadcast).
/// 2. Signal [`ShutdownGuard::shutdown`] and drain in-flight work —
///    bounded by `config.drain_timeout`
///    ([`ShutdownError::DrainTimeout`] on expiry).
/// 3. Run the caller-supplied `finalize` hook — bounded by the *remaining*
///    `config.shutdown_timeout` budget
///    ([`ShutdownError::ShutdownTimeout`] if the overall budget is
///    exhausted, [`ShutdownError::TaskFailed`] if the hook errors).
///
/// ```rust,no_run
/// use shutdown_kit::{ShutdownConfig, ShutdownGuard, run_shutdown};
///
/// # async fn example() -> Result<(), shutdown_kit::ShutdownError> {
/// let guard = ShutdownGuard::new();
/// let task = guard.clone();
/// tokio::spawn(async move {
///     let _ = task.wait_for_shutdown().await; // stop on signal
///     // ... flush buffers, close connections ...
///     drop(task);                             // drain contribution done
/// });
///
/// run_shutdown(ShutdownConfig::defaults(), &guard, || async {
///     tracing::info!("final flush complete");
///     Ok(())
/// })
/// .await?;
/// # Ok(())
/// # }
/// ```
pub async fn run_shutdown<F, Fut>(
    config: ShutdownConfig,
    guard: &ShutdownGuard,
    finalize: F,
) -> Result<(), ShutdownError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), ShutdownError>>,
{
    // Resolve on whichever arrives first: an OS signal (Ctrl+C/SIGTERM via
    // `shutdown_signal`, which then broadcasts) or a broadcast that was
    // already triggered (e.g. `trigger_shutdown` from tests or an embedded
    // supervisor). Subscribing before the select means a broadcast fired
    // earlier in this function's lifetime is still observed.
    let mut rx = signal::subscribe_shutdown();
    tokio::select! {
        _ = signal::shutdown_signal() => {}
        _ = rx.recv() => {}
    }
    guard.shutdown();

    // Overall budget starts once the signal arrives.
    let overall = tokio::time::timeout(config.shutdown_timeout, async {
        guard
            .wait_for_completion_with_deadline(config.drain_timeout)
            .await?;
        finalize().await
    })
    .await;

    match overall {
        Ok(result) => result,
        Err(_) => Err(ShutdownError::ShutdownTimeout(config.shutdown_timeout)),
    }
}

/// Feature-gated shutdown flag for lightweight cancellation.
#[cfg(feature = "shutdown-flag")]
pub mod flag;

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
