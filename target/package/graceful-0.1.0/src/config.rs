use std::time::Duration;

/// Configuration for graceful shutdown behavior.
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight requests to complete.
    pub drain_timeout: Duration,
    /// Maximum time to wait for the overall shutdown process.
    pub shutdown_timeout: Duration,
}

impl ShutdownConfig {
    /// Create a new builder with default values.
    pub fn builder() -> ShutdownConfigBuilder {
        ShutdownConfigBuilder::default()
    }

    /// Create a config with default values (30s drain, 10s shutdown).
    pub fn defaults() -> Self {
        Self {
            drain_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self::defaults()
    }
}

/// Builder for `ShutdownConfig`.
#[derive(Debug, Clone)]
pub struct ShutdownConfigBuilder {
    drain_timeout: Duration,
    shutdown_timeout: Duration,
}

impl ShutdownConfigBuilder {
    /// Set the drain timeout.
    pub fn drain_timeout(mut self, timeout: Duration) -> Self {
        self.drain_timeout = timeout;
        self
    }

    /// Set the shutdown timeout.
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> ShutdownConfig {
        ShutdownConfig {
            drain_timeout: self.drain_timeout,
            shutdown_timeout: self.shutdown_timeout,
        }
    }
}

impl Default for ShutdownConfigBuilder {
    fn default() -> Self {
        Self {
            drain_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(10),
        }
    }
}
