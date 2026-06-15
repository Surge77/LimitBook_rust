## Summary

<!-- What does this PR do and why? -->

## Type of change

- [ ] feat — new feature
- [ ] fix — bug fix
- [ ] perf — performance improvement
- [ ] refactor — no behavior change
- [ ] test — tests only
- [ ] docs — documentation only
- [ ] chore — tooling / deps

## Matcher impact

- [ ] This PR does **not** touch `engine-core` matching logic, **or**
- [ ] It does, and I added/updated tests **before** the implementation and all property tests pass.

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo test --all` passes
- [ ] `cd frontend && npm run build` passes (if frontend touched)
- [ ] No `.unwrap()` / `.expect()` added to library code
- [ ] No new `unsafe` without a soundness comment
- [ ] Files stay under 300 lines

## Benchmark impact (if hot path touched)

<!-- Paste before/after criterion numbers (p50/p99/p999, orders/sec). -->
