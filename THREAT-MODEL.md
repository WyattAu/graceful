# Threat Model — graceful

Reference: STRIDE. Scope: the crate's public API surface. Trust boundary:
(1) bytes/inputs entering public constructors and parsers, (2) concurrent
callers sharing interior state. graceful is an in-process library — it opens
no sockets and inherits the embedding process's trust domain.

Purpose: Shutdown coordination (`shutdown-kit`) — graceful drain with deadlines and shutdown reasons

## Assets

| ID | Asset | Exposed via |
|----|-------|-------------|
| A1 | bounded shutdown latency | hostile input, concurrent callers |
| A2 | no lost in-flight work before drain | hostile input, concurrent callers |

## STRIDE Analysis

| # | Threat | Category | Surface | Mitigation | Residual risk |
|---|--------|----------|---------|------------|---------------|
| T1 | Hang: task ignores completion, blocks shutdown forever | DoS | `drain` | hard deadline aborts drain and reports timed-out tasks | documented |
| T2 | Premature shutdown drops requests | DoS | `signal handling` | drain-first semantics with tests pinning in-flight completion | documented |

## Repudiation

The crate keeps no audit trail; attribution of calls to callers is out of
scope for an in-process library.

## Out of Scope

- Network transport security (the crate never opens sockets).
- Storage-host compromise: an attacker who controls the host can bypass all
  in-process mitigations.
- Denial of service via resource exhaustion of the host process beyond the
  bounds enforced above.

Reviewed: 2026-09-11
