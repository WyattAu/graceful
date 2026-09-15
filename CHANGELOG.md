# Changelog

All notable changes are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [0.3.0] - 2026-09-15

### Fixed

- **`wait_for_shutdown` is event-driven** (`tokio::sync::watch`), replacing
  the 50 ms poll loop — signal-to-wake latency is now immediate, with no
  wasted wakeups.
- **`ShutdownConfig::drain_timeout` is no longer dead config**: the new
  [`run_shutdown`] and [`ShutdownGuard::wait_for_completion_with_deadline`]
  enforce it.
- SIGTERM/Ctrl-C handler-install failures now trigger an immediate
  shutdown broadcast instead of leaving `subscribe_shutdown` waiters
  hanging forever.
- Drop semantics clarified: dropping the last task clone completes the
  drain; dropping the very last clone signals shutdown as a safety net.

### Changed (breaking — 0.x minor bump)

- `wait_for_shutdown` is infallible (the safety net guarantees
  convergence); `ShutdownError::ChannelClosed` was unreachable and has
  been removed. Remaining variants — `DrainTimeout`, `ShutdownTimeout`,
  `TaskFailed` — are all produced by `run_shutdown` (phantom variants
  eliminated).

### Added

- `ShutdownGuard::wait_for_completion[_with_deadline]` — resolve when all
  task clones drop (drain complete).
- `run_shutdown(config, &guard, finalize)` — signal → drain (drain
  timeout) → finalize (overall shutdown timeout), folding every error
  variant.
- `ShutdownGuard::watch_receiver` for `tokio::select!` integrations.


# Changelog

All notable changes are documented here.


All notable changes to this project are documented here. Format: [Keep a
Changelog](https://keepachangelog.com/) — versions follow [semver](https://semver.org).

## [0.2.0] - 2026-09-05

### Added
- Initial public release.
