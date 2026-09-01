use std::time::Duration;

use tokio::sync::broadcast;

/// A broadcast sender used to coordinate shutdown across tasks.
static SHUTDOWN_TX: std::sync::OnceLock<broadcast::Sender<()>> = std::sync::OnceLock::new();

fn shutdown_sender() -> broadcast::Sender<()> {
    SHUTDOWN_TX
        .get_or_init(|| {
            let (tx, _) = broadcast::channel(1);
            tx
        })
        .clone()
}

/// Wait for a shutdown signal (Ctrl+C on all platforms, SIGTERM on Unix).
///
/// This function listens for OS signals and broadcasts a shutdown notification
/// to all waiting tasks.
pub async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("failed to install SIGTERM handler: {e}");
                    ctrl_c.await.ok();
                    tracing::info!("received Ctrl+C signal");
                    let _ = shutdown_sender().send(());
                    return;
                }
            };

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("received Ctrl+C signal");
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM signal");
            }
        }
    }

    #[cfg(not(unix))]
    {
        match ctrl_c.await {
            Ok(()) => tracing::info!("received Ctrl+C signal"),
            Err(e) => {
                tracing::error!("failed to install Ctrl+C handler: {e}");
                return;
            }
        }
    }

    // Broadcast shutdown to all subscribers
    let _ = shutdown_sender().send(());
}

/// Subscribe to shutdown signals.
///
/// Returns a `broadcast::Receiver<()>` that will receive a value when
/// shutdown is triggered.
pub fn subscribe_shutdown() -> broadcast::Receiver<()> {
    shutdown_sender().subscribe()
}

/// Trigger a manual shutdown.
///
/// This broadcasts a shutdown signal to all subscribers.
pub fn trigger_shutdown() {
    let _ = shutdown_sender().send(());
}
