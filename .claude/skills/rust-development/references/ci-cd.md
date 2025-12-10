# CI/CD Reference

## Table of Contents

1. [GitHub Actions Workflow](#github-actions-workflow)
2. [Security Scanning](#security-scanning)
3. [Release Automation](#release-automation)
4. [Cross-Compilation](#cross-compilation)

## GitHub Actions Workflow

### Comprehensive CI Pipeline

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --all-features --all-targets -- -D warnings

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features

  doc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: -D warnings

  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: miri
      - uses: Swatinem/rust-cache@v2
      - run: cargo miri test
```

### Caching

`Swatinem/rust-cache@v2` provides intelligent caching of `~/.cargo` and `target/`, reducing build times by 50-80%.

## Security Scanning

### cargo-deny Configuration

Create `deny.toml`:

```toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "CC0-1.0",
    "MPL-2.0",
]
copyleft = "warn"
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"
deny = [
    { name = "openssl", wrappers = ["openssl-sys"] },  # Prefer rustls
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

### CI Integration

```yaml
security:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: EmbarkStudios/cargo-deny-action@v1
      with:
        command: check
```

### cargo-audit (Scheduled)

```yaml
audit:
  runs-on: ubuntu-latest
  schedule:
    - cron: '0 0 * * *'  # Daily
  steps:
    - uses: actions/checkout@v4
    - uses: rustsec/audit-check@v1
      with:
        token: ${{ secrets.GITHUB_TOKEN }}
```

### cargo-vet (Dependency Auditing)

For organizational trust sharing and audit trail:

```bash
cargo vet init
cargo vet                    # Check all dependencies
cargo vet certify <crate>    # Certify a crate after review
cargo vet import <org>       # Import trusted audits
```

## Release Automation

### cargo-release Workflow

```bash
# Install
cargo install cargo-release

# Release patch version
cargo release patch --execute

# Release with changelog
cargo release minor --execute
```

### GitHub Actions Release

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --release

      - name: Generate changelog
        run: |
          cargo install git-cliff
          git cliff --latest --strip header > CHANGELOG.md

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          body_path: CHANGELOG.md
          files: target/release/myapp
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Publish to crates.io
        run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

## Cross-Compilation

### Using cross

```bash
# Install
cargo install cross

# Build for target
cross build --target x86_64-unknown-linux-musl --release
cross build --target aarch64-unknown-linux-gnu --release
```

### Cross.toml Configuration

```toml
[target.x86_64-unknown-linux-musl]
pre-build = [
    "apt-get update && apt-get install -y musl-tools"
]

[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main"
```

### Common Targets

| Target | Description |
|--------|-------------|
| `x86_64-unknown-linux-musl` | Static Linux binaries |
| `aarch64-unknown-linux-gnu` | ARM64 Linux |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows |

### Multi-Platform Release CI

```yaml
build:
  strategy:
    matrix:
      include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-musl
        - os: ubuntu-latest
          target: aarch64-unknown-linux-gnu
        - os: macos-latest
          target: x86_64-apple-darwin
        - os: macos-latest
          target: aarch64-apple-darwin
        - os: windows-latest
          target: x86_64-pc-windows-msvc
  runs-on: ${{ matrix.os }}
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: ${{ matrix.target }}
    - run: cargo build --release --target ${{ matrix.target }}
    - uses: actions/upload-artifact@v4
      with:
        name: binary-${{ matrix.target }}
        path: target/${{ matrix.target }}/release/myapp*
```

## Dependency Vendoring

For offline or air-gapped builds:

```bash
cargo vendor

# Add to .cargo/config.toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```
