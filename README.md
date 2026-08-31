# graceful-shutdown

Graceful shutdown for Rust services — signal handling, connection draining, and RAII cleanup guards.

[![Crates.io](https://img.shields.io/crates/v/graceful.svg)](https://crates.io/crates/graceful)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](./LICENSE-MIT)

## Purpose

`graceful` provides primitives for implementing graceful shutdown in Rust services. It handles OS signals (Ctrl+C, SIGTERM), provides RAII guards for tracking shutdown state, and offers configurable drain timeouts.

## Features

- **Signal handling** — Ctrl+C (all platforms) and SIGTERM (Unix)
- **RAII guards** — automatic shutdown tracking via `ShutdownGuard`
- **Configurable timeouts** — drain and shutdown timeout configuration
- **Broadcast-based** — multiple tasks can subscribe to shutdown signals
- **No unsafe code** — `#![forbid(unsafe_code)]`

## Usage

```rust
use graceful::{ShutdownGuard, shutdown_signal, ShutdownConfig};

#[tokio::main]
async fn main() {
    let guard = ShutdownGuard::new();
    let config = ShutdownConfig::defaults();

    // Spawn background tasks
    let g = guard.clone();
    tokio::spawn(async move {
        loop {
            if g.is_shutdown() {
                tracing::info!("task shutting down gracefully");
                break;
            }
            // Do work...
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    // Run your server with graceful shutdown
    let app = axum::Router::new();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tokio::select! {
        result = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()) => {
            if let Err(e) = result {
                eprintln!("server error: {}", e);
            }
        }
        _ = shutdown_signal() => {
            tracing::info!("shutdown signal received");
        }
    }

    // Drop the guard to signal all tasks to stop
    drop(guard);

    // Wait for drain to complete (with timeout)
    tokio::time::sleep(config.drain_timeout).await;
    tracing::info!("shutdown complete");
}
```

## Comparison with Manual Signal Handling

| Feature | Manual | `graceful` |
|---------|--------|-----------|
| Ctrl+C handling | Manual `tokio::signal` | Automatic |
| SIGTERM handling | Manual `#[cfg(unix)]` | Automatic |
| RAII shutdown tracking | Custom `AtomicBool` | `ShutdownGuard` |
| Broadcast to tasks | Manual channels | Built-in broadcast |
| Configurable timeouts | Manual implementation | `ShutdownConfig` |

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT license](./LICENSE-MIT) at your option.
