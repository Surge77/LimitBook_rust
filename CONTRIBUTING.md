# Contributing to LimitBook

Thanks for your interest in contributing! This document describes how to set up the project,
the standards we hold code to, and the workflow for getting changes merged.

## Ground rules

- **Matcher correctness is non-negotiable.** Any change to `engine-core/src/matcher.rs` or
  `book.rs` must come with tests proving the new behavior *before* the implementation, and must
  not break existing property tests.
- **The engine-core hot path stays clean:** no async, no I/O, no logging, no heap allocation.
- **No `.unwrap()` / `.expect()` in library code.** Return `Result<_, EngineError>` and propagate
  with `?`. `unwrap` is acceptable only in tests and benches.
- **No `unsafe`** without a comment proving soundness and explaining why it is needed.
- **Files stay under 300 lines.** Split by responsibility before adding more.

## Development setup

```bash
# Rust toolchain (stable)
rustup component add clippy rustfmt

# Build + test the workspace
cargo build
cargo test

# Frontend
cd frontend && npm install && npm run dev
```

## Before you open a PR

Run the same gates CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cd frontend && npm run build
```

All four must pass. PRs with failing CI will not be reviewed until green.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`.

Examples:

```
feat(engine): add fill-or-kill order type
fix(matcher): preserve time priority on partial-fill amend
perf(book): pool order nodes in a slab arena
test(matcher): add proptest for quantity conservation
```

Keep each commit to one logical change.

## Testing expectations

- New engine logic requires unit tests (`#[cfg(test)]`) and, where it touches matching
  invariants, a `proptest`.
- Coverage target: **80%+ on engine-core, 100% on the matcher.** Check with
  `cargo llvm-cov --html`.
- No real network or filesystem in unit tests.

## Branching

- `main` is protected; never commit directly.
- Branch names: `feature/<desc>`, `fix/<desc>`, `perf/<desc>`.
- Open a PR; CI must pass; one approval before merge.

## Reporting bugs / requesting features

Use the GitHub issue templates. For security issues, **do not** open a public issue — see
[SECURITY.md](SECURITY.md).
