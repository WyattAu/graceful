/// Errors that can occur during graceful shutdown.
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    /// The shutdown signal channel was closed.
    #[error("shutdown channel closed")]
    ChannelClosed,

    /// The drain timed out.
    #[error("drain timed out after {0:?}")]
    DrainTimeout(std::time::Duration),

    /// The overall shutdown timed out.
    #[error("shutdown timed out after {0:?}")]
    ShutdownTimeout(std::time::Duration),

    /// A task failed during shutdown.
    #[error("task failed: {0}")]
    TaskFailed(String),
}
