# Requirements — graceful

Numbered, testable requirements. Every requirement maps to at least one named
test or doc-comment contract; security-relevant items cite threat-model rows.

Scope: Shutdown coordination (`shutdown-kit`) — graceful drain with deadlines and shutdown reasons

## Functional

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-SK-001 | Shutdown signal triggers drain; in-flight tasks complete or deadline aborts them | MUST |
| REQ-SK-002 | Reason for shutdown is captured and observable | MUST |
| REQ-SK-003 | Double shutdown is idempotent | MUST |

## Security

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-SK-100 | Deadline enforcement cannot be bypassed by task count (bounded wait) | MUST |

## Observability & API hygiene

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-SK-900 | All fallible public APIs return typed errors; production `unwrap`/`expect` is denied or explicitly justified with an invariant comment | MUST |
| REQ-SK-901 | Public items carry doc comments with runnable examples where practical | SHOULD |
