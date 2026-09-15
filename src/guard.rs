use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Notify, watch};

use crate::error::ShutdownError;

struct GuardInner {
    /// `true` once shutdown has been signaled. Shared with every clone.
    /// The paired receiver is retained alongside the sender: dropping it
    /// makes `Sender::send` refuse to store the value, which would leave
    /// `is_shutdown()` reading a stale flag.
    flag_tx: watch::Sender<bool>,
    _flag_rx: watch::Receiver<bool>,
    /// Notified when the last task clone drops (in-flight work complete).
    done: Notify,
}

/// RAII guard that tracks shutdown state and in-flight work.
///
/// Clone the guard and hand clones to background tasks; each clone marks
/// one unit of in-flight work. Two waits are supported:
///
/// - [`wait_for_shutdown`](Self::wait_for_shutdown) resolves when
///   [`shutdown`](Self::shutdown) is signaled — used by tasks to notice
///   they should stop.
/// - [`wait_for_completion`](Self::wait_for_completion) resolves when
///   every task clone has been dropped — used by the coordinator to know
///   draining finished. Dropping a clone is therefore what *completes*
///   the drain; it does not by itself signal shutdown.
///
/// When the last clone anywhere is dropped the shutdown flag is signaled
/// as a safety net (nothing is left to observe it, but subscribers of
/// [`crate::subscribe_shutdown`] are woken rather than left hanging).
#[derive(Clone)]
pub struct ShutdownGuard {
    inner: Arc<GuardInner>,
}

impl ShutdownGuard {
    /// Create a new shutdown guard.
    pub fn new() -> Self {
        let (flag_tx, flag_rx) = watch::channel(false);
        Self {
            inner: Arc::new(GuardInner {
                flag_tx,
                _flag_rx: flag_rx,
                done: Notify::new(),
            }),
        }
    }

    /// Signal shutdown to all clones. Wakes every
    /// [`wait_for_shutdown`](Self::wait_for_shutdown) waiter immediately.
    pub fn shutdown(&self) {
        let _ = self.inner.flag_tx.send(true);
    }

    /// Check if shutdown has been signaled.
    pub fn is_shutdown(&self) -> bool {
        *self.inner.flag_tx.borrow()
    }

    /// Subscribe a new receiver on the shutdown flag, for integrations
    /// that prefer raw `watch` access (e.g. `tokio::select!` arms).
    pub fn watch_receiver(&self) -> watch::Receiver<bool> {
        self.inner.flag_tx.subscribe()
    }

    /// Wait for shutdown to be signaled.
    ///
    /// Event-driven: returns immediately once signaled, with no polling
    /// latency. Infallible: dropping the last guard clone signals shutdown
    /// as a safety net, so the channel value always converges to `true`.
    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.inner.flag_tx.subscribe();
        let _ = rx.wait_for(|signaled| *signaled).await;
    }

    /// Wait until every clone except this one has been dropped — i.e.
    /// this clone is the sole remaining owner and the drain is complete.
    /// Resolves immediately if no other clones are outstanding.
    pub async fn wait_for_completion(&self) {
        loop {
            // Register interest before checking the count so a concurrent
            // drop cannot slip a notify between check and await. Spurious
            // wakeups are fine: the count is rechecked each iteration.
            let notified = self.inner.done.notified();
            if Arc::strong_count(&self.inner) <= 1 {
                return;
            }
            notified.await;
        }
    }

    /// Like [`wait_for_completion`](Self::wait_for_completion), bounded by
    /// `deadline`. On expiry the in-flight work has not drained and
    /// [`ShutdownError::DrainTimeout`] is returned (the process should
    /// proceed to exit).
    pub async fn wait_for_completion_with_deadline(
        &self,
        deadline: Duration,
    ) -> Result<(), ShutdownError> {
        match tokio::time::timeout(deadline, self.wait_for_completion()).await {
            Ok(()) => Ok(()),
            Err(_) => Err(ShutdownError::DrainTimeout(deadline)),
        }
    }

    /// Number of outstanding clones (including this one). Intended for
    /// tests and diagnostics.
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }
}

impl Default for ShutdownGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // `self` is being dropped, so the count includes it until the end
        // of drop: 1 means this was the last clone anywhere.
        if Arc::strong_count(&self.inner) == 1 {
            // Last clone anywhere: nothing can signal shutdown anymore.
            // Signal it so any `wait_for_shutdown` waiter and broadcast
            // subscribers are not left hanging.
            let _ = self.inner.flag_tx.send(true);
        }
        // Wake completion waiters unconditionally; they recheck the count
        // and re-arm, so spurious wakeups are harmless.
        self.inner.done.notify_waiters();
    }
}
