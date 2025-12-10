# Configuration Reference

## Table of Contents

1. [Clippy Configuration](#clippy-configuration)
2. [rustfmt Configuration](#rustfmt-configuration)
3. [rust-analyzer Configuration](#rust-analyzer-configuration)
4. [Workspace Configuration](#workspace-configuration)

## Clippy Configuration

### clippy.toml

```toml
# Minimum supported Rust version
msrv = "1.85.0"

# Complexity threshold (default 25)
cognitive-complexity-threshold = 25

# Disallow specific methods
disallowed-methods = [
    { path = "std::env::var", reason = "Use config system instead" }
]

# Prevent false positives for proper nouns in docs
doc-valid-idents = ["GitHub", "PostgreSQL", "SQLite", "OpenSSL"]
```

### Common Lint Groups

```toml
[lints.clippy]
# Enable lint groups
all = "warn"
pedantic = "warn"
cargo = "warn"
nursery = "warn"  # Experimental but useful

# Specific denials
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"

# Common allows for pedantic
module_name_repetitions = "allow"
too_many_lines = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
```

## rustfmt Configuration

### rustfmt.toml

```toml
edition = "2024"
style_edition = "2024"

# Line width
max_width = 100

# Imports
imports_granularity = "Module"
group_imports = "StdExternalCrate"
reorder_imports = true

# Shorthand
use_field_init_shorthand = true
use_try_shorthand = true

# Comments and formatting
wrap_comments = true
format_code_in_doc_comments = true
normalize_comments = true
normalize_doc_attributes = true
```

## rust-analyzer Configuration

### VS Code settings.json

```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.procMacro.enable": true,
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.inlayHints.chainingHints.enable": true,
    "rust-analyzer.inlayHints.parameterHints.enable": true,
    "rust-analyzer.inlayHints.typeHints.enable": true
}
```

### Debug Configuration (launch.json)

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug",
            "cargo": {
                "args": ["build", "--bin=myapp", "--package=myapp"],
                "filter": { "name": "myapp", "kind": "bin" }
            },
            "args": [],
            "cwd": "${workspaceFolder}",
            "env": { "RUST_BACKTRACE": "1" }
        }
    ]
}
```

Debug profile control:

```toml
[profile.dev]
debug = 2           # Full debug info
# debug = "line-tables-only"  # Smaller binaries
```

## Workspace Configuration

### Virtual Manifest Pattern

```toml
# Root Cargo.toml (no [package] section)
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/org/repo"

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
```

### Member Crate Cargo.toml

```toml
[package]
name = "my-crate"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
tokio.workspace = true
serde.workspace = true
```

### Explicit Library and Binary Declaration

```toml
[lib]
name = "mylib"
path = "src/lib.rs"

[[bin]]
name = "mycli"
path = "src/main.rs"
```
