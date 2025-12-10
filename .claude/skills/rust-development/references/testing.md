# Testing Reference

## Table of Contents

1. [Unit Tests](#unit-tests)
2. [Integration Tests](#integration-tests)
3. [Property-Based Testing](#property-based-testing)
4. [Mocking](#mocking)
5. [Benchmarking](#benchmarking)
6. [Code Coverage](#code-coverage)

## Unit Tests

### Conventions

```rust
// Place at bottom of source file
#[cfg(test)]
mod tests {
    use super::*;

    // Naming: <action>_<condition>_<expected_result>
    #[test]
    fn parse_returns_error_for_empty_string() {
        let result = parse("");
        assert!(result.is_err());
    }
}
```

### Useful Crates

- `pretty_assertions`: Colored diffs on assertion failures
- `test-case`: Parameterized tests

```rust
use test_case::test_case;

#[test_case(0, 0 ; "zero")]
#[test_case(1, 1 ; "one")]
#[test_case(2, 4 ; "two squared")]
fn square_returns_expected(input: i32, expected: i32) {
    assert_eq!(square(input), expected);
}
```

## Integration Tests

Place in `tests/` directory. Each file compiles as separate crate—only public API accessible.

```
tests/
├── common/
│   └── mod.rs      # Shared utilities (not compiled as test)
├── api_tests.rs
└── cli_tests.rs
```

### CLI Testing with assert_cmd

```rust
use assert_cmd::Command;

#[test]
fn cli_runs_successfully() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage"));
}
```

## Property-Based Testing

### proptest

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn reverse_reverse_is_identity(v: Vec<i32>) {
        let reversed: Vec<_> = v.iter().rev().rev().cloned().collect();
        prop_assert_eq!(&reversed, &v);
    }

    #[test]
    fn parse_valid_input(s in "[a-z]{1,10}") {
        let result = parse(&s);
        prop_assert!(result.is_ok());
    }
}
```

Key features:
- Automatic shrinking to minimal failing case
- Strategy combinators for custom generators
- Persistent regression files

## Mocking

### mockall

```rust
use mockall::{automock, predicate::*};

#[automock]
trait Database {
    fn get(&self, key: &str) -> Option<String>;
}

#[test]
fn service_uses_database() {
    let mut mock = MockDatabase::new();
    mock.expect_get()
        .with(eq("key"))
        .times(1)
        .returning(|_| Some("value".into()));

    let service = Service::new(mock);
    assert_eq!(service.lookup("key"), Some("value".into()));
}
```

## Benchmarking

### Criterion

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => n,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn bench_fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("fibonacci");
    for n in [10, 15, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            n,
            |b, &n| b.iter(|| fibonacci(black_box(n))),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_fibonacci);
criterion_main!(benches);
```

Commands:
- `cargo bench`: Run benchmarks
- `cargo bench -- --save-baseline main`: Save baseline
- `cargo bench -- --baseline main`: Compare against baseline

### iai-callgrind (Deterministic CI)

Measures instruction counts instead of wall-clock time—ideal for CI environments.

```rust
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

#[library_benchmark]
fn bench_sort() -> Vec<i32> {
    let mut v = vec![3, 1, 4, 1, 5, 9, 2, 6];
    v.sort();
    v
}

library_benchmark_group!(name = sort_group; benchmarks = bench_sort);
main!(library_benchmark_groups = sort_group);
```

## Code Coverage

### cargo-llvm-cov (Recommended)

Uses LLVM instrumentation for accurate coverage.

```bash
# Install
cargo install cargo-llvm-cov

# Generate HTML report
cargo llvm-cov --html --open

# Generate lcov format (for CI integration)
cargo llvm-cov --lcov --output-path lcov.info

# With all features
cargo llvm-cov --all-features --workspace
```

### Coverage in CI

```yaml
- name: Install cargo-llvm-cov
  uses: taiki-e/install-action@cargo-llvm-cov

- name: Generate coverage
  run: cargo llvm-cov --lcov --output-path lcov.info

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    files: lcov.info
```
