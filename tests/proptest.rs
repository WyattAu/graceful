//! Property-based tests for shutdown-kit crate.

use proptest::prelude::*;
use std::time::Duration;

use shutdown_kit::ShutdownConfig;

#[test]
fn config_drain_timeout_always_positive() {
    proptest!(|(drain_ms in 1u64..1_000_000u64, shutdown_ms in 1u64..1_000_000u64)| {
        let config = ShutdownConfig::builder()
            .drain_timeout(Duration::from_millis(drain_ms))
            .shutdown_timeout(Duration::from_millis(shutdown_ms))
            .build();
        prop_assert!(config.drain_timeout > Duration::ZERO);
        prop_assert!(config.shutdown_timeout > Duration::ZERO);
    });
}

#[test]
fn config_builder_preserves_values() {
    proptest!(|(drain_ms in 1u64..60_000, shutdown_ms in 1u64..60_000)| {
        let config = ShutdownConfig::builder()
            .drain_timeout(Duration::from_millis(drain_ms))
            .shutdown_timeout(Duration::from_millis(shutdown_ms))
            .build();
        prop_assert_eq!(config.drain_timeout, Duration::from_millis(drain_ms));
        prop_assert_eq!(config.shutdown_timeout, Duration::from_millis(shutdown_ms));
    });
}

#[test]
fn config_clone_preserves_values() {
    proptest!(|(drain_ms in 1u64..60_000, shutdown_ms in 1u64..60_000)| {
        let config = ShutdownConfig::builder()
            .drain_timeout(Duration::from_millis(drain_ms))
            .shutdown_timeout(Duration::from_millis(shutdown_ms))
            .build();
        let cloned = config.clone();
        prop_assert_eq!(config.drain_timeout, cloned.drain_timeout);
        prop_assert_eq!(config.shutdown_timeout, cloned.shutdown_timeout);
    });
}

#[test]
fn default_config_has_sensible_values() {
    let config = ShutdownConfig::defaults();
    assert!(config.drain_timeout >= Duration::from_secs(1));
    assert!(config.shutdown_timeout >= Duration::from_secs(1));
    assert!(config.drain_timeout >= config.shutdown_timeout);
}
