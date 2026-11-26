## Summary
<!-- Brief description of the changes and motivation -->

## Related Issues / Plans
<!-- Closes #xyz or references plan items (e.g. SEC-X) -->

## Security Checklist
Please verify the following before requesting review:
- [ ] **No secrets or tokens** committed in code, fixtures, tests, or commit messages (invariant 11).
- [ ] **Remote command safety**: Any new `Command` variant is classified in `policy::validate_remote` (invariant 12).
- [ ] **Path traversal safety**: Any file serving, deletion, or disk access uses `safe_join` (invariant 13).
- [ ] **Authentication**: New routes require token authentication by default unless explicitly added to `PUBLIC_ROUTES`.
- [ ] **Security headers**: Standard security headers (`nosniff`, `no-referrer`, `DENY`, CSP, `x-request-id`) are preserved.
- [ ] **Sanitized errors**: Error responses do not leak internal database errors, table names, or server paths.
- [ ] **Audit logging**: State-altering operations log structured audit events at `tracing::info!(target: "audit", ...)`.
- [ ] **Tests**: New or changed behavior has regression tests in `tests/security.rs` or relevant unit test modules.
- [ ] **Code quality**: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all pass.
