use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::Notify;

/// A shutdown flag that can be checked, set, and awaited.
///
/// Thread-safe and clone-friendly. Each clone shares the same underlying flag.
/// Uses `Acquire`/`Release` ordering for correct visibility across threads.
///
/// # Example
///
/// ```rust,no_run
/// use shutdown_kit::flag::ShutdownFlag;
///
/// let flag = ShutdownFlag::new();
/// let flag2 = flag.clone();
///
/// // In background task
/// tokio::spawn(async move {
///     flag2.wait().await;
///     println!("shutdown requested");
/// });
///
/// // Later, request shutdown
/// flag.set();
/// ```
#[derive(Clone)]
pub struct ShutdownFlag {
    flag: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl ShutdownFlag {
    /// Create a new shutdown flag (not set).
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Check if shutdown has been requested.
    #[inline]
    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Request shutdown. Wakes all waiting tasks.
    pub fn set(&self) {
        self.flag.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    /// Wait until shutdown is requested. Returns immediately if already set.
    pub async fn wait(&self) {
        if self.is_set() {
            return;
        }
        self.notify.notified().await;
    }

    /// Create a future that resolves when shutdown is set.
    ///
    /// Useful with `tokio::select!` for cancellation.
    pub fn cancelled(&self) -> ShutdownFuture<'_> {
        ShutdownFuture { flag: self }
    }

    /// Reset the flag to not-set. Primarily for testing.
    pub fn reset(&self) {
        self.flag.store(false, Ordering::Release);
    }

    /// Convert to `Arc<AtomicBool>` for backward compatibility with
    /// code that already uses raw atomic bools.
    pub fn as_bool(&self) -> Arc<AtomicBool> {
        self.flag.clone()
    }
}

impl Default for ShutdownFlag {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ShutdownFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownFlag")
            .field("is_set", &self.is_set())
            .finish()
    }
}

/// Future that resolves when the shutdown flag is set.
///
/// Created by [`ShutdownFlag::cancelled`]. Resolves immediately if the flag
/// is already set, otherwise waits for [`ShutdownFlag::set`] to be called.
pub struct ShutdownFuture<'a> {
    flag: &'a ShutdownFlag,
}

impl<'a> Future for ShutdownFuture<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.flag.is_set() {
            return Poll::Ready(());
        }
        // Register waker via the Notify, then double-check
        let mut notified = Box::pin(self.flag.notify.notified());
        match notified.as_mut().poll(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => {
                // Double-check after registering waker
                if self.flag.is_set() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

/// Macro for selecting with shutdown cancellation.
///
/// # Example
///
/// ```rust,no_run
/// use shutdown_kit::{select_with_shutdown, flag::ShutdownFlag};
///
/// let flag = ShutdownFlag::new();
/// select_with_shutdown!(flag, {
///     result = some_async_op() => {
///         println!("got result: {:?}", result);
///     }
/// });
/// ```
#[macro_export]
macro_rules! select_with_shutdown {
    ($flag:expr, { $($body:tt)* }) => {
        tokio::select! {
            $($body)*
            _ = $flag.wait() => {
                tracing::debug!("shutdown flag set, cancelling");
            }
        }
    };
}
