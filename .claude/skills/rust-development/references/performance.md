# Performance Reference

## Table of Contents

1. [Release Profile Tuning](#release-profile-tuning)
2. [Allocator Selection](#allocator-selection)
3. [Compile Time Optimization](#compile-time-optimization)
4. [Runtime Optimization Patterns](#runtime-optimization-patterns)
5. [Profiling Tools](#profiling-tools)
6. [Profile-Guided Optimization](#profile-guided-optimization)

## Release Profile Tuning

### Maximum Performance Profile

```toml
[profile.release]
opt-level = 3       # Maximum optimization
lto = "fat"         # Full link-time optimization
codegen-units = 1   # Single codegen unit for better optimization
panic = "abort"     # No unwinding overhead
strip = "symbols"   # Remove debug symbols from binary
```

### Build Command

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Enables CPU-specific optimizations including auto-vectorization.

### Profile Variants

```toml
# Fast dev builds
[profile.dev]
opt-level = 0
debug = "line-tables-only"  # Faster builds, smaller binaries

# Optimized dev for testing
[profile.dev.package."*"]
opt-level = 2  # Optimize dependencies

# Release with debug info
[profile.release-with-debug]
inherits = "release"
debug = true
strip = "none"
```

## Allocator Selection

**Critical for musl targets**: Default allocator can cause 7x slowdown.

### jemalloc (Recommended for multi-threaded)

```toml
[dependencies]
tikv-jemallocator = "0.5"
```

```rust
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

### mimalloc (Alternative)

```toml
[dependencies]
mimalloc = "0.1"
```

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

### Arena Allocators

For short-lived allocations:

```rust
// bumpalo - heterogeneous types
use bumpalo::Bump;

let arena = Bump::new();
let s = arena.alloc_str("hello");
let n = arena.alloc(42u64);

// typed-arena - single type, supports cycles
use typed_arena::Arena;

let arena: Arena<Node> = Arena::new();
let node = arena.alloc(Node::new());
```

## Compile Time Optimization

### sccache (Distributed Cache)

```bash
cargo install sccache
export RUSTC_WRAPPER=sccache

# Disable incremental for better cache hits
export CARGO_INCREMENTAL=0
```

### Faster Linker

```toml
# .cargo/config.toml

# Linux (mold)
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

# macOS (lld)
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

# Windows (lld)
[target.x86_64-pc-windows-msvc]
linker = "lld-link"
```

### Split Debug Info

```toml
[profile.dev]
split-debuginfo = "unpacked"  # Faster incremental builds
```

## Runtime Optimization Patterns

### Static vs Dynamic Dispatch

```rust
// Static dispatch - monomorphized, inlined (prefer for hot paths)
fn process<T: Processor>(p: &T) {
    p.execute();
}

// Dynamic dispatch - single codegen, smaller binary
fn process_dyn(p: &dyn Processor) {
    p.execute();
}
```

### Iterator Patterns

Zero-cost abstractions—compiles to same code as manual loops:

```rust
// These compile to equivalent machine code
let sum: i32 = data.iter().filter(|x| **x > 0).sum();

let mut sum = 0;
for x in &data {
    if *x > 0 { sum += *x; }
}
```

### Avoid Bounds Checks

```rust
// Bounds checked
let val = arr[i];

// Unchecked (unsafe)
let val = unsafe { *arr.get_unchecked(i) };

// Better: use iterators
for val in arr.iter() { /* ... */ }
```

## Profiling Tools

### Flamegraph

```bash
cargo install flamegraph
cargo flamegraph --bin myapp -- [args]
```

Output: `flamegraph.svg`

### perf (Linux)

```bash
# Record
perf record -g cargo run --release

# Report
perf report
```

### Instruments (macOS)

```bash
cargo instruments --release --bin myapp -t time
```

### Memory Profiling

```bash
# DHAT (heap profiling via Valgrind)
cargo install cargo-dhat
cargo dhat --bin myapp

# heaptrack
heaptrack ./target/release/myapp
heaptrack_gui heaptrack.myapp.*.gz
```

## Profile-Guided Optimization

### Using cargo-pgo

```bash
cargo install cargo-pgo

# Step 1: Build instrumented binary
cargo pgo build

# Step 2: Run with representative workload
./target/x86_64-unknown-linux-gnu/release/myapp [workload]

# Step 3: Build optimized binary using profiles
cargo pgo optimize
```

Can yield **10%+ improvement** for CPU-bound workloads.

### Manual PGO

```bash
# Instrumented build
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release

# Run workload
./target/release/myapp [representative workload]

# Merge profiles
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

# Optimized build
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" cargo build --release
```

## Summary

| Optimization | Typical Gain | When to Use |
|--------------|--------------|-------------|
| LTO + codegen-units=1 | 10-20% | Always for releases |
| target-cpu=native | 5-15% | Single-platform deployment |
| jemalloc/mimalloc | 0-50%+ (musl: critical) | Multi-threaded, musl |
| PGO | 10%+ | CPU-bound workloads |
| Arena allocators | Varies | Many short-lived allocations |
