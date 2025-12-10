# Memory Profiling Guide

This guide covers how to profile memory usage in misskey-rs.

## Prerequisites

### Linux

```bash
# Install heaptrack (recommended for heap profiling)
sudo apt install heaptrack heaptrack-gui

# Install valgrind (alternative, more detailed)
sudo apt install valgrind massif-visualizer
```

### macOS

```bash
# Use Instruments (built into Xcode)
xcode-select --install

# Or install heaptrack via Homebrew
brew install heaptrack
```

## Profiling Methods

### 1. Heaptrack (Recommended)

Heaptrack is the easiest way to profile heap allocations.

```bash
# Build with debug info (release with debug symbols)
cargo build --release

# Run with heaptrack
heaptrack target/release/misskey

# Analyze results
heaptrack_gui heaptrack.misskey.*.gz
```

Key metrics:
- Peak heap memory usage
- Total allocations
- Memory leaks
- Allocation hot spots

### 2. Valgrind Massif

For detailed heap profiling:

```bash
# Build in release mode
cargo build --release

# Run with massif
valgrind --tool=massif target/release/misskey

# View results
ms_print massif.out.*

# Or use GUI
massif-visualizer massif.out.*
```

### 3. jemallocator with DHAT

Add to Cargo.toml for detailed allocation tracking:

```toml
[profile.release]
debug = true  # Keep debug symbols

[dependencies]
dhat = "0.3"  # Add for heap profiling
```

Then in main.rs:
```rust
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();
    // ... rest of main
}
```

Run normally and check `dhat-heap.json` output.

### 4. Built-in Rust Memory Tracking

Use jemalloc stats for runtime monitoring:

```rust
use jemalloc_ctl::{epoch, stats};

// Advance the epoch to get fresh stats
epoch::advance().unwrap();

let allocated = stats::allocated::read().unwrap();
let resident = stats::resident::read().unwrap();

println!("Allocated: {} bytes", allocated);
println!("Resident: {} bytes", resident);
```

## Performance Targets

From RUST_FORK_PLAN.md:

| Metric | Target |
|--------|--------|
| Memory usage (idle) | < 256MB |

## Common Memory Issues

### 1. String Allocations

Look for:
- Unnecessary `.to_string()` calls
- `format!()` in hot paths
- Cloning large strings

Fix:
```rust
// Before
fn process(s: String) { ... }

// After
fn process(s: &str) { ... }
```

### 2. Vec Allocations

Look for:
- Growing Vecs without `with_capacity()`
- Collecting into Vecs unnecessarily

Fix:
```rust
// Before
let items: Vec<_> = (0..1000).map(|i| i * 2).collect();

// After
let mut items = Vec::with_capacity(1000);
items.extend((0..1000).map(|i| i * 2));
```

### 3. Async Task Memory

Look for:
- Large futures stored in tasks
- Holding data across await points

Fix:
- Use `Box::pin()` for large futures
- Minimize data held across await

### 4. Cache Growth

Look for:
- Unbounded caches
- Missing TTL on cached items

Fix:
- Use bounded LRU caches
- Add expiration to cached items

## Benchmarking Memory

Create a memory benchmark:

```rust
#[cfg(test)]
mod memory_tests {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingAllocator;

    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            System.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            System.dealloc(ptr, layout)
        }
    }

    #[test]
    fn test_operation_memory() {
        let before = ALLOCATED.load(Ordering::SeqCst);

        // ... perform operation

        let after = ALLOCATED.load(Ordering::SeqCst);
        assert!(after - before < 1_000_000, "Operation used too much memory");
    }
}
```

## Continuous Monitoring

### Prometheus Metrics

Add memory metrics to the server:

```rust
use prometheus::{register_gauge, Gauge};

lazy_static! {
    static ref MEMORY_USAGE: Gauge = register_gauge!(
        "misskey_memory_bytes",
        "Current memory usage in bytes"
    ).unwrap();
}

// Update periodically
fn update_memory_metrics() {
    if let Ok(usage) = get_memory_usage() {
        MEMORY_USAGE.set(usage as f64);
    }
}
```

### Logging Memory Usage

```rust
use tracing::info;
use sysinfo::{System, SystemExt, ProcessExt};

fn log_memory_usage() {
    let mut sys = System::new_all();
    sys.refresh_all();

    if let Some(process) = sys.process(sysinfo::get_current_pid().unwrap()) {
        info!(
            memory_mb = process.memory() / 1024 / 1024,
            "Process memory usage"
        );
    }
}
```

## Interpreting Results

### Heaptrack Output

```
total runtime: 10.5s
calls to allocation functions: 1,234,567
temporary allocations: 456,789 (37%)
peak heap memory consumption: 128MB
potential memory leaks: 12KB
```

- **Temporary allocations**: Should be < 50% ideally
- **Peak consumption**: Should be < 256MB target
- **Memory leaks**: Should be 0 for long-running server

### Valgrind Massif Output

```
    MB
200.0^                                      #
     |                                @@@@@@#
     |                          ::::::@ ::::#
     |                    ::::::@:::::@ ::::#
     |              ::::::@:::::@:::::@ ::::#
     |        ::::::@:::::@:::::@:::::@ ::::#
     |  ::::::@:::::@:::::@:::::@:::::@ ::::#
   0 +--------------------------------------->time
```

Look for:
- Steady growth (potential leak)
- Spikes during specific operations
- Final memory after processing
