# Auth API Coding Patterns

This document codifies style and implementation patterns for `auth-api`.

## Handler Pattern
- Keep handlers thin: validate/request mapping + service orchestration + HTTP response mapping only.
- Move provider/config branching into service modules.
- Use shared error payload shape (`code`, `message`) for API-facing failures.
- Keep best-effort side effects non-blocking unless explicitly required by contract.

## Config/Env Pattern
- Parse env values through shared helpers in `service/config.rs`.
- Normalize non-empty strings once:
  - trim whitespace
  - strip matching wrapping quotes (`"..."` or `'...'`)
- Parse booleans only via `1|true` (case-insensitive), with explicit defaults.
- Parse numeric envs via typed helper functions (`u16`, `u64`), with explicit defaults.
- Prefer lowercasing only where semantics require it (e.g., provider selectors).

## Email Delivery Pattern
- Use `service/email.rs::try_send_verification_email` as the single dispatch entrypoint.
- Keep provider-specific implementations (`smtp`, `resend`) as focused functions.
- Keep shared template/content generation in one helper to prevent drift.
- Return actionable errors from service layer; handlers log once with stable warning format.

## Test Pattern
- Smoke tests must validate API contract boundaries (e.g., forbidden fields absent), not only happy paths.
- E2E verification tokens should come from email channels (Mailpit/Resend), not register responses.
- Keep smoke execution gated by env (`RUN_SMOKE_AUTH`) to avoid slowing default `cargo test`.
