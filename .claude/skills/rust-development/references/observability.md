# Observability Reference

## Table of Contents

1. [Error Handling](#error-handling)
2. [Structured Logging with tracing](#structured-logging-with-tracing)
3. [Metrics and Health Endpoints](#metrics-and-health-endpoints)
4. [Unsafe Code Guidelines](#unsafe-code-guidelines)

## Error Handling

### Library Errors (thiserror)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid format: {0}")]
    InvalidFormat(String),

    #[error("missing field: {field}")]
    MissingField { field: &'static str },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

Guidelines:
- Lowercase messages, no trailing punctuation
- Use `#[error(transparent)]` for wrapped errors
- `#[from]` enables `?` operator conversion

### Application Errors (anyhow)

```rust
use anyhow::{Context, Result, bail};

fn process_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .context("failed to read config file")?;

    if content.is_empty() {
        bail!("config file is empty");
    }

    let config: Config = toml::from_str(&content)
        .context("failed to parse config")?;

    Ok(config)
}
```

### Boundary Pattern

```rust
// Library crate - structured errors
#[derive(Debug, Error)]
pub enum LibError { /* ... */ }

// Application - wrap with anyhow
fn main() -> anyhow::Result<()> {
    let result = lib::process()?;  // LibError -> anyhow::Error
    Ok(())
}
```

## Structured Logging with tracing

### Basic Setup

```rust
use tracing::{info, warn, error, debug, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("application started");
}
```

### Instrumentation

```rust
#[instrument(skip(db), fields(user_id = %user.id), err)]
async fn process_order(user: &User, db: &Database) -> Result<()> {
    info!(order_count = 5, "processing orders");

    let orders = db.fetch_orders(user.id).await?;

    for order in orders {
        debug!(order_id = %order.id, "processing order");
        // ...
    }

    Ok(())
}
```

Attributes:
- `skip(field)`: Don't log sensitive data
- `fields(key = value)`: Add structured fields
- `err`: Log error on Result::Err
- `ret`: Log return value

### Layered Subscriber

```rust
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn,tower=warn"));

    // Console layer (pretty for dev)
    let console_layer = fmt::layer()
        .pretty()
        .with_target(true);

    // JSON file layer (production)
    let file = std::fs::File::create("app.log").unwrap();
    let json_layer = fmt::layer()
        .json()
        .with_writer(file);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(json_layer)
        .init();
}
```

### OpenTelemetry Integration

```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::sdk::trace::TracerProvider;

let provider = TracerProvider::builder()
    .with_simple_exporter(opentelemetry_jaeger::new_agent_pipeline().build()?)
    .build();

let tracer = provider.tracer("my-app");
let otel_layer = OpenTelemetryLayer::new(tracer);

tracing_subscriber::registry()
    .with(otel_layer)
    .init();
```

### Sentry Integration

```rust
let _guard = sentry::init(("DSN", sentry::ClientOptions {
    release: sentry::release_name!(),
    ..Default::default()
}));

// Integrate with tracing
use sentry_tracing::SentryLayer;

tracing_subscriber::registry()
    .with(SentryLayer::default())
    .init();
```

## Metrics and Health Endpoints

### Health Endpoints Pattern

```rust
// Liveness - is the process running?
async fn health_live() -> impl IntoResponse {
    StatusCode::OK
}

// Readiness - are dependencies available?
async fn health_ready(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.ping().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

// Router
let app = Router::new()
    .route("/health/live", get(health_live))
    .route("/health/ready", get(health_ready));
```

### Prometheus Metrics

```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref REQUESTS_TOTAL: Counter = register_counter!(
        "http_requests_total",
        "Total HTTP requests"
    ).unwrap();

    static ref REQUEST_DURATION: Histogram = register_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration"
    ).unwrap();
}

// Endpoint
async fn metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
```

### Panic Hook

```rust
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into());

        tracing::error!(
            location = %location,
            "panic occurred: {}", info.payload().downcast_ref::<&str>().unwrap_or(&"unknown")
        );
    }));
}
```

## Unsafe Code Guidelines

### Documentation Requirements

```rust
/// # Safety
/// - `ptr` must be aligned to `align_of::<T>()`
/// - `ptr` must point to initialized memory
/// - The memory must be valid for reads of `size_of::<T>()` bytes
unsafe fn deref_raw<T>(ptr: *const T) -> T {
    // SAFETY: Caller guarantees ptr is aligned, initialized, and valid
    unsafe { ptr.read() }
}
```

### Enable Lint

```rust
#![deny(unsafe_op_in_unsafe_fn)]  // Require explicit unsafe blocks
```

### Miri for UB Detection

```bash
# Install
rustup +nightly component add miri

# Run tests under Miri
cargo +nightly miri test
```

Detects:
- Out-of-bounds access
- Use-after-free
- Invalid alignment
- Data races
- Memory leaks

Limitations:
- Cannot check FFI code
- Only tests executed code paths

### Unsafe Superpowers (Rustonomicon)

1. Dereference raw pointers
2. Call unsafe functions
3. Access mutable statics
4. Implement unsafe traits
5. Access union fields

### Safe Encapsulation Pattern

```rust
mod internal {
    pub struct Buffer {
        ptr: *mut u8,
        len: usize,
    }

    impl Buffer {
        pub fn new(len: usize) -> Self {
            let ptr = unsafe { std::alloc::alloc(std::alloc::Layout::array::<u8>(len).unwrap()) };
            Self { ptr, len }
        }

        pub fn get(&self, index: usize) -> Option<u8> {
            if index < self.len {
                // SAFETY: index is bounds-checked
                Some(unsafe { *self.ptr.add(index) })
            } else {
                None
            }
        }
    }

    impl Drop for Buffer {
        fn drop(&mut self) {
            unsafe {
                std::alloc::dealloc(self.ptr, std::alloc::Layout::array::<u8>(self.len).unwrap());
            }
        }
    }
}
```

Key principles:
- Minimal unsafe scope
- Module privacy prevents invariant violation
- Safe public API
