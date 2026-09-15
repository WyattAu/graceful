//! Errors that occur during graceful shutdown.
///
/// Every variant documents the code path that produces it; no variant is
/// dead (checked by tests).
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    /// In-flight work did not drain within the configured
    /// [`ShutdownConfig::drain_timeout`](crate::ShutdownConfig::drain_timeout).
    /// Produced by
    /// [`wait_for_completion_with_deadline`](crate::ShutdownGuard::wait_for_completion_with_deadline)
    /// and [`run_shutdown`](crate::run_shutdown).
    #[error("drain timed out after {0:?}")]
    DrainTimeout(std::time::Duration),

    /// The overall shutdown — signal, drain, and finalization — exceeded
    /// [`ShutdownConfig::shutdown_timeout`](crate::ShutdownConfig::shutdown_timeout).
    /// Produced by [`run_shutdown`](crate::run_shutdown).
    #[error("shutdown timed out after {0:?}")]
    ShutdownTimeout(std::time::Duration),

    /// A caller-supplied finalization hook failed. Produced by
    /// [`run_shutdown`](crate::run_shutdown).
    #[error("task failed: {0}")]
    TaskFailed(String),
}
