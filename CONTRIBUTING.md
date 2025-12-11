# Contributing Guidelines

This document defines the strict implementation rules for the misskey-rs project.
All contributors must follow these rules.

---

## 0. General Principles

### 0.1 Language Policy

| Target | Language |
|--------|----------|
| Code (variable names, function names, type names) | **English** |
| Test function names | **English** |
| Code comments | **English** |
| Documentation (README, CONTRIBUTING, etc.) | **English** |
| Commit messages | **English** |

### 0.2 MSRV (Minimum Supported Rust Version)

- **MSRV**: `1.85.0` (Rust 2024 Edition)
- Specified in the `rust-version` field of `Cargo.toml`
- Raising the MSRV is treated as a Breaking Change

### 0.3 Runtime

- **Async runtime**: Tokio (`tokio = { features = ["full"] }`)
- Compatibility with other runtimes (async-std, etc.) is not guaranteed

### 0.4 Database / ORM

- **Database**: PostgreSQL
- **ORM**: Sea-ORM
- **Migrations**: sea-orm-migration

---

## 1. Coding Standards

### 1.1 Naming Conventions

| Target | Convention | Example |
|--------|------------|---------|
| Variables, functions, modules | `snake_case` | `get_user`, `user_id` |
| Types, traits, structs, enums | `UpperCamelCase` | `UserService`, `AccountError` |
| Enum variants | `UpperCamelCase` | `NotFound`, `InvalidInput` |
| Constants, statics | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_TIMEOUT` |
| Type parameters | Single uppercase letter or descriptive name | `T`, `E`, `Item` |
| Lifetimes | Short lowercase | `'a`, `'de`, `'ctx` |

**Additional Rules**:
- Acronyms are treated as a single word in CamelCase: `Uuid` (not `UUID`), `HttpClient` (not `HTTPClient`)
- No `get_` prefix for getters: `fn value(&self)` (not `fn get_value(&self)`)
- Use `is_`, `has_`, `can_` for functions returning bool

### 1.2 Formatting

- **Required**: Use `rustfmt` (enforced in CI)
- Line width: Maximum **100 characters**
- Indentation: **4 spaces** (no tabs)
- Trailing commas: Required in multi-line lists
- Trailing whitespace: Prohibited

```bash
# Check formatting
cargo fmt --check

# Auto-format
cargo fmt
```

### 1.3 Documentation

- All public APIs must have documentation comments (`///`)
- First line should be a concise one-sentence description
- Use `# Examples` section when including examples
- `# Safety` section is required for `unsafe`
- `# Errors` section is recommended for functions that return errors

```rust
/// Retrieves a user by their ID.
///
/// # Errors
///
/// Returns `UserError::NotFound` if the user does not exist.
pub async fn get_user(&self, id: UserId) -> Result<User, UserError> {
    // ...
}
```

---

## 2. Architecture

### 2.1 Crate Structure (Pragmatic Hexagonal Architecture)

```
crates/
├── common/      # Common utilities (datetime, ID generation, etc.)
├── core/        # Domain logic, services, trait definitions (ports)
├── db/          # Database adapter (repository implementations)
├── api/         # HTTP API adapter
├── federation/  # ActivityPub adapter
├── queue/       # Job queue adapter
├── mfm/         # MFM parser
└── server/      # Application entry point
```

### 2.2 Dependency Rules

```
server → api, queue, federation, db, core, common
api, queue, federation → core, db, common
db → core, common
core → common
common → (external dependencies only)
```

**Design Decisions**:

> **Note**: This project adopts a "Pragmatic Hexagonal Architecture" prioritizing practicality.
> In strict Hexagonal Architecture, adapter layers (api, federation, etc.) depend only on `core`,
> but this project allows direct dependencies on `db`.
>
> **Reasons**:
> - Easier optimization of read queries (no unnecessary service layer traversal)
> - Balance between implementation cost and project scale
> - Can be gradually strictified as needed

**Prohibited**:
- Dependencies from `core` to adapter layers (`db`, `api`, `federation`, `queue`)
- Direct dependencies between adapters (`api` → `federation`, etc.)
- Circular dependencies

### 2.3 Abstraction via Traits

- Define traits (ports) in the domain layer (`core`)
- Implement traits in adapter layers
- Enable swapping implementations via dependency injection

```rust
// core/src/repositories/user.rs
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;
}

// db/src/repositories/user.rs
pub struct PostgresUserRepository { /* ... */ }

#[async_trait]
impl UserRepository for PostgresUserRepository {
    // implementation
}
```

---

## 3. Error Handling

### 3.1 Crate Selection

| Layer | Crate | Purpose |
|-------|-------|---------|
| Library layer (`core`, `db`, `api`, etc.) | `thiserror` | Typed error definitions |
| Application layer (`server`) | `anyhow` | Error propagation with context |

> **Note**: Although `db` is architecturally an adapter layer, it uses `thiserror` from an error
> handling perspective because callers need to branch handling based on error types.

### 3.2 Error Type Design

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user not found: {0}")]
    NotFound(UserId),

    #[error("invalid email format: {0}")]
    InvalidEmail(String),

    #[error("database error")]
    Database(#[from] sea_orm::DbErr),
}
```

### 3.3 Adding Context

```rust
use anyhow::{Context, Result};

async fn process_user(id: UserId) -> Result<()> {
    let user = repository
        .find_by_id(id)
        .await
        .context("failed to fetch user from database")?;

    // ...
    Ok(())
}
```

### 3.4 Prohibited Practices

- Using `unwrap()` (use `expect()` with a reason, or use `?`)
- Suppressing errors (`let _ = ...` to ignore errors)
- Overusing `panic!` (only for unrecoverable states)

---

## 4. Security

### 4.1 Required Checks (Enforced in CI)

```bash
# Treat all Clippy warnings as errors
cargo clippy -- -D warnings

# Vulnerability check for dependencies
cargo audit

# Prohibit unsafe code (configured in Cargo.toml)
# [lints.rust]
# unsafe_code = "forbid"
```

### 4.2 Input Validation

**Using the Newtype Pattern**:

```rust
/// Validated email address.
#[derive(Debug, Clone)]
pub struct ValidatedEmail(String);

impl ValidatedEmail {
    pub fn new(email: &str) -> Result<Self, ValidationError> {
        // In actual implementation, use validator or email_address crate
        if email.contains('@') && email.len() <= 254 {
            Ok(Self(email.to_string()))
        } else {
            Err(ValidationError::InvalidEmail)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

> **Recommendation**: Use dedicated libraries such as the `validator` crate (derive macros) or
> `email_address` crate for actual implementation.

### 4.3 SQL Injection Prevention

```rust
// Required: Use parameterized queries (Sea-ORM)
User::find_by_id(user_id).one(&db).await?;

// Prohibited: Query construction via string concatenation
// let query = format!("SELECT * FROM users WHERE id = {}", user_id); // NG!
```

### 4.4 Authentication & Sessions

- Password hashing: Use Argon2
- JWT: Retrieve secret key from environment variables
- Cookies: `HttpOnly`, `Secure`, `SameSite=Strict` flags required
- Sessions: Configure appropriate timeouts

### 4.5 XSS Prevention

```rust
use ammonia::clean;

let safe_html = clean(user_input);
```

---

## 5. Testing

### 5.1 Coverage Goals

- **Minimum requirement**: 80% or higher
- Aim for 90% or higher for critical business logic

### 5.2 Test Types

| Type | Location | Purpose |
|------|----------|---------|
| Unit tests | `#[cfg(test)]` modules | Verify internal functions |
| Integration tests | `tests/` directory | Verify public APIs |
| Documentation tests | Code examples in `///` | Verify API usage examples work |

### 5.3 Test Naming Convention

**Format**: `test_<function_name>_<condition>_<expected_result>`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_with_valid_input_returns_ok() {
        // Arrange
        let input = "user@example.com";

        // Act
        let result = ValidatedEmail::new(input);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_without_at_sign_returns_error() {
        let result = ValidatedEmail::new("invalid-email");
        assert!(result.is_err());
    }
}
```

### 5.4 Async Tests

```rust
#[tokio::test]
async fn test_async_function() {
    // ...
}
```

### 5.5 Coverage Measurement

```bash
# Using cargo-tarpaulin (Linux x86_64)
cargo tarpaulin --out Html --output-dir coverage/

# Or grcov (cross-platform)
RUSTFLAGS="-C instrument-coverage" cargo test
grcov . -s . --binary-path ./target/debug/ -o ./coverage/
```

---

## 6. Logging / Tracing

### 6.1 Crates

- **tracing**: Structured logging and spans
- **tracing-subscriber**: Log output configuration

### 6.2 Log Level Guidelines

| Level | Usage |
|-------|-------|
| `error` | Unrecoverable errors requiring immediate attention |
| `warn` | Potential issues requiring attention but operation continues |
| `info` | Important state changes (server startup, connection established, etc.) |
| `debug` | Debug information for development |
| `trace` | Detailed trace information (may impact performance) |

### 6.3 Using Spans

```rust
use tracing::{info, instrument, warn};

#[instrument(skip(self, db), fields(user_id = %id))]
pub async fn get_user(&self, db: &DatabaseConnection, id: UserId) -> Result<User, UserError> {
    info!("fetching user");

    let user = User::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| {
            warn!("user not found");
            UserError::NotFound(id)
        })?;

    Ok(user)
}
```

### 6.4 Prohibited Practices

- Using `println!` / `eprintln!` (use `tracing` macros)
- Logging sensitive information (passwords, tokens, etc.)
- Enabling `trace` level constantly in production

---

## 7. Configuration Management

### 7.1 Priority Order (Higher Takes Precedence)

1. Environment variables
2. `.env` file (for development, not in Git)
3. Configuration files (`config/` directory)
4. Default values

### 7.2 Environment Variable Naming Convention

```
MISSKEY_<CATEGORY>_<NAME>
```

Examples:
- `MISSKEY_DATABASE_URL`
- `MISSKEY_REDIS_URL`
- `MISSKEY_JWT_SECRET`

### 7.3 Sensitive Information

- **Required**: Manage via environment variables
- **Prohibited**: Direct inclusion in configuration files or code
- `.env.example` should contain only placeholders

```bash
# .env.example
MISSKEY_DATABASE_URL=postgres://user:password@localhost/misskey
MISSKEY_JWT_SECRET=your-secret-key-here
```

---

## 8. Git / Commit Conventions

### 8.1 Conventional Commits

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### 8.2 Type List

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only changes |
| `style` | Changes that don't affect code meaning (formatting, etc.) |
| `refactor` | Code changes that neither fix bugs nor add features |
| `perf` | Performance improvements |
| `test` | Adding or modifying tests |
| `build` | Changes to build system or external dependencies |
| `ci` | Changes to CI configuration |
| `chore` | Other changes (not affecting build or documentation) |

### 8.3 Scope (For This Project)

- `common`, `core`, `db`, `api`, `federation`, `queue`, `mfm`, `server`
- `deps` (dependencies)
- `ci` (CI/CD)

### 8.4 Commit Message Rules

1. **Subject line should be 50 characters or less**
2. **Capitalize only the first letter** (of the description after the type)
3. **No period at the end of the subject line**
4. **Use imperative mood**: `Add feature` (not `Added feature`)
5. **Wrap body at 72 characters**
6. **Write what and why** (not how)

### 8.5 Examples

```
feat(api): Add user authentication endpoint

Implement JWT-based authentication for the REST API.
This enables secure access control for protected resources.

Closes #123
```

```
fix(db): Prevent SQL injection in user search

Use parameterized queries instead of string concatenation
to prevent potential SQL injection attacks.

BREAKING CHANGE: UserRepository.search() now requires SearchParams struct
```

### 8.6 Breaking Changes

- Add `BREAKING CHANGE:` to the footer
- Or append `!` to the type: `feat!:` or `feat(api)!:`

---

## 9. PR Review Process

### 9.1 Review Requirements

| PR Size | Required Approvals | Notes |
|---------|-------------------|-------|
| Small (~100 lines) | 1 | Typo fixes, small bug fixes |
| Medium (100~500 lines) | 1 | Feature additions, refactoring |
| Large (500+ lines) | 2 | Large features, architecture changes |

### 9.2 Review Criteria

- [ ] **Functionality**: Does it meet the requirements?
- [ ] **Code quality**: Naming, readability, duplication
- [ ] **Security**: Input validation, authentication/authorization
- [ ] **Testing**: Are appropriate tests added?
- [ ] **Performance**: N+1 problems, unnecessary clones
- [ ] **Error handling**: Appropriate error types, context
- [ ] **Documentation**: Documentation for public APIs

### 9.3 Self-Merge Policy

- Do not approve and merge your own PR
- In emergencies, another team member should merge; post-merge review is required

---

## 10. CI/CD Checklist

For a PR to be merged, all of the following must pass:

- [ ] `cargo fmt --check` (formatting)
- [ ] `cargo clippy -- -D warnings` (static analysis)
- [ ] `cargo test` (all tests)
- [ ] `cargo audit` (security)
- [ ] Coverage 80% or higher
- [ ] Commit messages follow Conventional Commits format
- [ ] Required review approvals obtained

---

## 11. References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [thiserror documentation](https://docs.rs/thiserror)
- [anyhow documentation](https://docs.rs/anyhow)
- [tracing documentation](https://docs.rs/tracing)
- [Sea-ORM documentation](https://www.sea-ql.org/SeaORM/)
