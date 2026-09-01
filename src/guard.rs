use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// RAII guard that tracks shutdown state.
///
/// When all clones of a `ShutdownGuard` are dropped, the associated
/// shutdown is considered complete. Cloning the guard and sending the
/// clone to background tasks allows the main task to wait for all
/// tasks to finish by dropping the original guard.
#[derive(Clone)]
pub struct ShutdownGuard {
    shutdown: Arc<AtomicBool>,
}

impl ShutdownGuard {
    /// Create a new shutdown guard.
    pub fn new() -> Self {
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal shutdown to all clones of this guard.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been signaled.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Wait for shutdown to be signaled.
    pub async fn wait_for_shutdown(&self) {
        loop {
            if self.is_shutdown() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}

impl Default for ShutdownGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // If this is the last clone, signal shutdown
        if Arc::strong_count(&self.shutdown) == 1 {
            self.shutdown();
        }
    }
}
