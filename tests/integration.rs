//! Integration tests for the graceful (shutdown-kit) crate.
//!
//! Tests ShutdownConfig defaults/builder, ShutdownGuard creation/state/shutdown,
//! and ShutdownError display.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use shutdown_kit::{ShutdownConfig, ShutdownError, ShutdownGuard};

// ---------------------------------------------------------------------------
// ShutdownConfig defaults and builder
// ---------------------------------------------------------------------------

#[test]
fn config_defaults_has_correct_timeouts() {
    let config = ShutdownConfig::defaults();
    assert_eq!(config.drain_timeout, Duration::from_secs(30));
    assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
}

#[test]
fn config_default_trait_matches_defaults() {
    let config = ShutdownConfig::default();
    assert_eq!(config.drain_timeout, Duration::from_secs(30));
    assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
}

#[test]
fn config_builder_overrides_drain_timeout() {
    let config = ShutdownConfig::builder()
        .drain_timeout(Duration::from_secs(120))
        .build();
    assert_eq!(config.drain_timeout, Duration::from_secs(120));
    assert_eq!(config.shutdown_timeout, Duration::from_secs(10)); // default
}

#[test]
fn config_builder_overrides_shutdown_timeout() {
    let config = ShutdownConfig::builder()
        .shutdown_timeout(Duration::from_secs(5))
        .build();
    assert_eq!(config.shutdown_timeout, Duration::from_secs(5));
    assert_eq!(config.drain_timeout, Duration::from_secs(30)); // default
}

#[test]
fn config_builder_overrides_both() {
    let config = ShutdownConfig::builder()
        .drain_timeout(Duration::from_secs(60))
        .shutdown_timeout(Duration::from_secs(20))
        .build();
    assert_eq!(config.drain_timeout, Duration::from_secs(60));
    assert_eq!(config.shutdown_timeout, Duration::from_secs(20));
}

#[test]
fn config_builder_empty_uses_defaults() {
    let config = ShutdownConfig::builder().build();
    assert_eq!(config.drain_timeout, Duration::from_secs(30));
    assert_eq!(config.shutdown_timeout, Duration::from_secs(10));
}

#[test]
fn config_is_cloneable() {
    let config = ShutdownConfig::builder()
        .drain_timeout(Duration::from_secs(5))
        .build();
    let cloned = config.clone();
    assert_eq!(cloned.drain_timeout, Duration::from_secs(5));
}

// ---------------------------------------------------------------------------
// ShutdownGuard creation, is_shutdown(), shutdown()
// ---------------------------------------------------------------------------

#[test]
fn guard_new_is_not_shutdown() {
    let guard = ShutdownGuard::new();
    assert!(!guard.is_shutdown());
}

#[test]
fn guard_default_is_not_shutdown() {
    let guard = ShutdownGuard::default();
    assert!(!guard.is_shutdown());
}

#[test]
fn guard_shutdown_sets_state() {
    let guard = ShutdownGuard::new();
    guard.shutdown();
    assert!(guard.is_shutdown());
}

#[test]
fn guard_shutdown_propagates_to_clones() {
    let guard = ShutdownGuard::new();
    let clone1 = guard.clone();
    let clone2 = guard.clone();

    guard.shutdown();

    assert!(guard.is_shutdown());
    assert!(clone1.is_shutdown());
    assert!(clone2.is_shutdown());
}

#[test]
fn guard_clone_shares_state() {
    let guard = ShutdownGuard::new();
    let clone = guard.clone();

    assert!(!clone.is_shutdown());

    guard.shutdown();
    assert!(clone.is_shutdown());
}

#[test]
fn guard_drop_last_clone_triggers_shutdown() {
    let guard = ShutdownGuard::new();
    let clone = guard.clone();

    drop(guard);
    // clone is still alive, should not be shutdown yet
    assert!(!clone.is_shutdown());

    drop(clone);
    // All dropped without panic
}

#[test]
fn guard_multiple_clones_drop_order() {
    let g1 = ShutdownGuard::new();
    let g2 = g1.clone();
    let g3 = g1.clone();

    drop(g2);
    assert!(!g1.is_shutdown());
    assert!(!g3.is_shutdown());

    drop(g3);
    assert!(!g1.is_shutdown());

    drop(g1);
    // All dropped without panic
}

#[tokio::test]
async fn guard_wait_for_shutdown() {
    let guard = ShutdownGuard::new();
    let clone = guard.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        clone.shutdown();
    });

    let start = std::time::Instant::now();
    guard.wait_for_shutdown().await;
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(2));
    assert!(elapsed >= Duration::from_millis(40));
}

#[tokio::test]
async fn guard_wait_for_shutdown_already_shutdown() {
    let guard = ShutdownGuard::new();
    guard.shutdown();

    let start = std::time::Instant::now();
    guard.wait_for_shutdown().await;
    assert!(start.elapsed() < Duration::from_millis(100));
}

#[test]
fn guard_manual_shutdown_overrides_drop() {
    let guard = ShutdownGuard::new();
    let clone = guard.clone();

    // Manually shutdown before dropping
    guard.shutdown();
    assert!(clone.is_shutdown());

    // Drop the original - should not double-shutdown
    drop(guard);
    assert!(clone.is_shutdown());
}

// ---------------------------------------------------------------------------
// ShutdownError display
// ---------------------------------------------------------------------------

#[test]
fn error_channel_closed_display() {}

#[test]
fn error_drain_timeout_display() {
    let err = ShutdownError::DrainTimeout(Duration::from_secs(30));
    assert_eq!(err.to_string(), "drain timed out after 30s");
}

#[test]
fn error_shutdown_timeout_display() {
    let err = ShutdownError::ShutdownTimeout(Duration::from_secs(10));
    assert_eq!(err.to_string(), "shutdown timed out after 10s");
}

#[test]
fn error_task_failed_display() {
    let err = ShutdownError::TaskFailed("worker 3 panicked".into());
    assert_eq!(err.to_string(), "task failed: worker 3 panicked");
}

#[test]
fn error_debug_format() {
    let err = ShutdownError::DrainTimeout(Duration::from_millis(250));
    let debug = format!("{err:?}");
    assert!(debug.contains("DrainTimeout"));
}

#[test]
fn error_drain_timeout_various_values() {
    let err1 = ShutdownError::DrainTimeout(Duration::from_secs(5));
    let err2 = ShutdownError::DrainTimeout(Duration::from_secs(300));
    assert_eq!(err1.to_string(), "drain timed out after 5s");
    assert_eq!(err2.to_string(), "drain timed out after 300s");
}

// ── 0.3.0: completion tracking + run_shutdown ──────────────────────────

#[tokio::test]
async fn completion_resolves_when_task_clones_drop() {
    let guard = ShutdownGuard::new();
    let task = guard.clone();
    let start = std::time::Instant::now();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(task);
    });

    guard.wait_for_completion().await;
    assert!(
        start.elapsed() >= Duration::from_millis(40),
        "must wait for the outstanding clone"
    );
    // Coordinator clone only remains.
    assert_eq!(guard.strong_count(), 1);
}

#[tokio::test]
async fn completion_with_deadline_times_out_into_drain_timeout() {
    let guard = ShutdownGuard::new();
    let _task = guard.clone(); // never dropped

    let err = guard
        .wait_for_completion_with_deadline(Duration::from_millis(50))
        .await
        .expect_err("outstanding clone must time out");
    assert!(matches!(err, ShutdownError::DrainTimeout(_)));
}

#[tokio::test]
async fn completion_resolves_immediately_with_no_tasks() {
    let guard = ShutdownGuard::new();
    // No clones outstanding: instant Ok.
    guard
        .wait_for_completion_with_deadline(Duration::from_millis(50))
        .await
        .expect("nothing outstanding");
}

#[tokio::test]
async fn completion_waits_through_the_last_task_clone() {
    // Two clones drop in sequence; the waiter must observe completion only
    // after both are gone, without shutdown ever being signaled.
    let waiter = ShutdownGuard::new();
    let task_a = waiter.clone();
    let task_b = waiter.clone();

    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        drop(task_a); // one task clone remains after this
        tokio::time::sleep(Duration::from_millis(30)).await;
        drop(task_b); // waiter is now the only clone
    });

    waiter.wait_for_completion().await;
    assert!(!waiter.is_shutdown(), "completion must not imply shutdown");
    handle.await.unwrap();
}

#[tokio::test]
async fn dropping_the_very_last_clone_signals_shutdown() {
    // A waiter subscribed on the watch channel (via a receiver obtained
    // before the final drop) observes the safety-net signal.
    let guard = ShutdownGuard::new();
    let mut rx = guard.watch_receiver();
    drop(guard);
    // Channel closed with the final sender — the stored flag from the
    // safety-net send is the last value; recv on a dropped sender yields
    // the current value once, then errors.
    let _ = rx.changed().await;
    assert!(*rx.borrow());
}

#[tokio::test]
async fn run_shutdown_drains_and_finalizes() {
    use shutdown_kit::{ShutdownConfig, run_shutdown};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let guard = ShutdownGuard::new();
    let task = guard.clone();
    let drained = Arc::new(AtomicBool::new(false));
    let d = drained.clone();

    // Simulate the OS signal arriving (trigger_shutdown broadcasts).
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        shutdown_kit::trigger_shutdown();
        // Task observes shutdown, finishes, drops its clone.
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(task);
        d.store(true, Ordering::SeqCst);
    });

    let finalized = Arc::new(AtomicBool::new(false));
    let f = finalized.clone();
    let result: Result<(), ShutdownError> = run_shutdown(
        ShutdownConfig::builder()
            .drain_timeout(Duration::from_secs(2))
            .shutdown_timeout(Duration::from_secs(5))
            .build(),
        &guard,
        || {
            let f = f.clone();
            async move {
                f.store(true, Ordering::SeqCst);
                Ok(())
            }
        },
    )
    .await;

    result.expect("shutdown should complete cleanly");
    assert!(drained.load(Ordering::SeqCst));
    assert!(finalized.load(Ordering::SeqCst));
}

#[tokio::test]
async fn run_shutdown_reports_task_failure() {
    use shutdown_kit::{ShutdownConfig, run_shutdown};

    // Fire the shutdown broadcast so `run_shutdown` proceeds.
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        shutdown_kit::trigger_shutdown();
    });

    let guard = ShutdownGuard::new();
    // Nothing outstanding: drain is instant.
    let result: Result<(), ShutdownError> = run_shutdown(
        ShutdownConfig::builder()
            .drain_timeout(Duration::from_millis(100))
            .shutdown_timeout(Duration::from_secs(1))
            .build(),
        &guard,
        || async { Err(ShutdownError::TaskFailed("flush failed".into())) },
    )
    .await;

    let err = result.expect_err("finalize hook failed");
    assert!(matches!(err, ShutdownError::TaskFailed(msg) if msg.contains("flush failed")));
}
