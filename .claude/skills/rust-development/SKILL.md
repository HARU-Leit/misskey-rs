---
name: rust-development
description: "Modern Rust development best practices for 2025. Use when working on Rust projects including: (1) Project setup and Cargo.toml configuration, (2) Clippy/rustfmt linting and formatting, (3) Error handling with thiserror/anyhow, (4) Async programming with Tokio, (5) Testing strategies (unit, integration, property-based), (6) CI/CD pipelines and security scanning, (7) Performance optimization and profiling, (8) Observability with tracing, (9) Unsafe code review."
---

# Rust Development (2024 Edition)

Rust 2024 edition (stabilized in Rust 1.85.0, February 2025) introduces async closures (`async || {}`), improved lifetime capture rules, and `AsyncFn*` traits in prelude.

## Quick Reference

### Error Handling Split

| Context | Crate | Usage |
|---------|-------|-------|
| Libraries | `thiserror` | Matchable, structured error types with `#[error]`, `#[from]` |
| Applications | `anyhow`/`eyre` | `Context`, `bail!` for error propagation |
| CLI diagnostics | `miette` | Source code context in error messages |

### Async Ecosystem

Tokio is the standard runtime—reqwest, sqlx, axum, tonic all require it. Native `async fn` in traits stable since 1.75; use `trait_variant` for Send bounds.

### Essential Crates

- **HTTP**: reqwest (with `rustls-tls`)
- **Web framework**: axum (Tower middleware, `#![forbid(unsafe_code)]`)
- **CLI**: clap (derive macros)
- **Serialization**: serde
- **Database**: sqlx (compile-time checked) or diesel (ORM)
- **Logging**: tracing + tracing-subscriber

## Cargo.toml Lints Configuration

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
cargo = "warn"
unwrap_used = "deny"
module_name_repetitions = "allow"
```

## Release Profile (Maximum Performance)

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

Build with `RUSTFLAGS="-C target-cpu=native"` for CPU-specific optimizations.

**Critical for musl targets**: Replace default allocator with jemalloc or mimalloc to avoid 7x slowdown.

## Project Structure

```
project/
├── src/
│   ├── lib.rs          # Library code
│   ├── main.rs         # Binary entry point
│   └── bin/            # Additional binaries
├── tests/              # Integration tests
├── benches/            # Criterion benchmarks
└── examples/           # Example programs
```

For workspaces: use `resolver = "2"`, `[workspace.package]`, `[workspace.dependencies]` with `.workspace = true`.

## Feature Flags

- Features must be **additive** (enabling never disables functionality)
- Use `dep:` prefix for optional dependencies
- Use `?` syntax for conditional activation
- Provide `std` feature (not `no_std`) when supporting both

## Documentation Conventions (RFC 1574)

1. One-line summary
2. Detailed explanation
3. `# Examples` (use `?` not `unwrap()`)
4. `# Errors`
5. `# Panics`
6. `# Safety` (for unsafe code)

## Detailed References

- **Linting and formatting details**: See [references/configuration.md](references/configuration.md)
- **Testing strategies and tools**: See [references/testing.md](references/testing.md)
- **CI/CD and security scanning**: See [references/ci-cd.md](references/ci-cd.md)
- **Performance optimization**: See [references/performance.md](references/performance.md)
- **Observability and error handling**: See [references/observability.md](references/observability.md)

## 2024 Edition Migration

1. Upgrade to Rust 1.85+
2. Run `cargo fix --edition`
3. Update `Cargo.toml`: `edition = "2024"`
4. Rename `rand` crate's `.gen()` to `.random()` (keyword reservation)
5. Review temporaries in `if let` and block tail expressions (tighter drop scopes)
