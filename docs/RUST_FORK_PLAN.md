# Misskey Rust Fork Project - Implementation Plan

## Project Overview

Misskeyをフォークし、バックエンドをRustで完全に書き換えるプロジェクト計画。

| 項目 | 内容 |
|------|------|
| **ベース** | Misskey 2025.12.0 |
| **対象** | バックエンド全体 + ActivityPub実装 |
| **言語** | Rust (Edition 2024) |
| **上流との関係** | 完全に独立 |
| **目的** | パフォーマンス向上、コード品質、保守性、新機能基盤 |

---

## Technology Stack (2025年12月版)

### Core Dependencies

```toml
[dependencies]
# Web Framework
axum = "0.8"                          # 2025/01 release
axum-extra = "0.10"
tower = "0.5"
tower-http = "0.6"

# ActivityPub
activitypub_federation = "0.6"        # Lemmy実績

# Database
sea-orm = "1.1"                       # Stable (2.0はRC)
sea-orm-migration = "1.1"

# Redis
fred = "10"                           # Valkey対応

# Job Queue
apalis = "0.7"                        # Stable
apalis-redis = "0.7"

# Runtime
tokio = "1"

# Serialization
serde = "1"
serde_json = "1"

# HTTP Client
reqwest = "0.12"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         Clients                                   │
│              (Web, Mobile, Third-party apps)                      │
└──────────────────────────────┬───────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│                        Axum 0.8                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐  │
│  │  REST API    │  │  ActivityPub │  │  WebSocket Streaming   │  │
│  │  /api/v1/*   │  │  /inbox      │  │  /streaming            │  │
│  │              │  │  /outbox     │  │                        │  │
│  │  Mastodon    │  │  /users/{id} │  │  Server-Sent Events    │  │
│  │  Compatible  │  │  /.well-known│  │                        │  │
│  └──────────────┘  └──────────────┘  └────────────────────────┘  │
└──────────────────────────────┬───────────────────────────────────┘
                               │
         ┌─────────────────────┼─────────────────────┐
         │                     │                     │
         ▼                     ▼                     ▼
┌─────────────────┐  ┌─────────────────────┐  ┌─────────────────┐
│  Service Layer  │  │ activitypub-        │  │  Apalis Workers │
│                 │  │ federation 0.6      │  │                 │
│  - NoteService  │  │                     │  │  - Delivery     │
│  - UserService  │  │  - Object trait     │  │  - Inbox proc   │
│  - AuthService  │  │  - Actor trait      │  │  - Scheduled    │
│  - etc.         │  │  - ActivityHandler  │  │  - Cleanup      │
└────────┬────────┘  └──────────┬──────────┘  └────────┬────────┘
         │                      │                      │
         └──────────────────────┼──────────────────────┘
                                │
         ┌──────────────────────┴──────────────────────┐
         │                                             │
         ▼                                             ▼
┌─────────────────────────┐               ┌─────────────────────────┐
│    SeaORM 1.1           │               │      fred 10            │
│    (PostgreSQL)         │               │      (Redis)            │
│                         │               │                         │
│  - Users, Notes         │               │  - Session cache        │
│  - Follows, Reactions   │               │  - Timeline cache       │
│  - DriveFiles           │               │  - Job queue backend    │
│  - Instances            │               │  - Pub/Sub              │
│  - Migrations           │               │  - Rate limiting        │
└─────────────────────────┘               └─────────────────────────┘
```

---

## Project Structure

```
misskey-rust/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── core/                     # Core domain logic
│   │   ├── src/
│   │   │   ├── entities/         # Domain entities
│   │   │   ├── services/         # Business logic
│   │   │   ├── repositories/     # Data access traits
│   │   │   └── events/           # Domain events
│   │   └── Cargo.toml
│   │
│   ├── db/                       # Database layer
│   │   ├── src/
│   │   │   ├── models/           # SeaORM entities
│   │   │   ├── migrations/       # DB migrations
│   │   │   └── repositories/     # Repository implementations
│   │   └── Cargo.toml
│   │
│   ├── federation/               # ActivityPub implementation
│   │   ├── src/
│   │   │   ├── actors/           # AP Actor implementations
│   │   │   ├── activities/       # Activity handlers
│   │   │   ├── objects/          # AP Object implementations
│   │   │   └── delivery/         # Outbox delivery
│   │   └── Cargo.toml
│   │
│   ├── api/                      # HTTP API layer
│   │   ├── src/
│   │   │   ├── routes/           # Axum routers
│   │   │   ├── handlers/         # Request handlers
│   │   │   ├── extractors/       # Custom extractors
│   │   │   ├── middleware/       # Auth, logging, etc.
│   │   │   └── responses/        # Response types
│   │   └── Cargo.toml
│   │
│   ├── queue/                    # Background job processing
│   │   ├── src/
│   │   │   ├── jobs/             # Job definitions
│   │   │   ├── workers/          # Worker implementations
│   │   │   └── scheduler/        # Cron jobs
│   │   └── Cargo.toml
│   │
│   ├── mfm/                      # MFM parser (or use mfm.rs)
│   │   └── Cargo.toml
│   │
│   └── common/                   # Shared utilities
│       ├── src/
│       │   ├── config/           # Configuration
│       │   ├── error/            # Error types
│       │   ├── id/               # ID generation
│       │   └── time/             # Time utilities
│       └── Cargo.toml
│
├── src/
│   └── main.rs                   # Application entry point
│
├── config/
│   ├── default.toml              # Default configuration
│   └── local.toml                # Local overrides
│
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml
│
└── tests/
    ├── integration/              # Integration tests
    └── federation/               # Federation tests
```

---

## Implementation Phases

### Phase 0: Foundation (2-3 weeks)

#### 0.1 Project Setup
- [x] Cargo workspace 初期化
- [x] CI/CD パイプライン構築 (GitHub Actions)
- [x] Docker 開発環境
- [x] pre-commit hooks (rustfmt, clippy)

#### 0.2 Core Infrastructure
- [x] Configuration システム (config crate)
- [x] Logging/Tracing 設定
- [x] Error handling 基盤 (thiserror/anyhow)
- [x] ID生成 (Snowflake互換 or ULIDv7)

#### 0.3 Database Setup
- [x] SeaORM 設定
- [x] 初期マイグレーション (User, Note基本)
- [x] Connection pooling
- [x] Test database 環境

```rust
// Phase 0 成果物例: Error handling
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation error: {0}")]
    Validation(#[from] validator::ValidationErrors),

    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Federation error: {0}")]
    Federation(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

### Phase 1: Core Entities (3-4 weeks)

#### 1.1 User System
- [x] User エンティティ (SeaORM)
- [x] UserProfile エンティティ
- [x] UserKeypair (RSA鍵ペア)
- [x] Authentication (session/token)
- [x] Password hashing (Argon2)

#### 1.2 Note System
- [x] Note エンティティ
- [x] Visibility (public/home/followers/specified)
- [x] Reply/Renote 関係
- [x] Mentions 抽出
- [x] Hashtags 抽出

#### 1.3 Follow System
- [x] Following エンティティ
- [x] FollowRequest エンティティ
- [x] Block/Mute 機能

#### 1.4 Drive System
- [x] DriveFile エンティティ
- [x] DriveFolder エンティティ
- [x] File upload handling
- [x] S3/Object storage 対応

```rust
// Phase 1 成果物例: Note entity
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "note")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub user_id: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub text: Option<String>,

    pub cw: Option<String>,

    pub visibility: Visibility,

    pub reply_id: Option<String>,
    pub renote_id: Option<String>,
    pub thread_id: Option<String>,

    #[sea_orm(column_type = "JsonBinary")]
    pub mentions: Vec<String>,

    #[sea_orm(column_type = "JsonBinary")]
    pub reactions: serde_json::Value,

    pub replies_count: i32,
    pub renote_count: i32,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(16))")]
pub enum Visibility {
    #[sea_orm(string_value = "public")]
    Public,
    #[sea_orm(string_value = "home")]
    Home,
    #[sea_orm(string_value = "followers")]
    Followers,
    #[sea_orm(string_value = "specified")]
    Specified,
}
```

---

### Phase 2: ActivityPub Integration (4-6 weeks)

#### 2.1 Actor Implementation
- [x] Person (User) Actor
- [x] Webfinger (.well-known/webfinger)
- [x] Actor endpoints (/users/{id})
- [x] Public key exposure

#### 2.2 Object Implementation
- [x] Note Object
- [x] Question Object (polls)
- [x] Image/Document attachments
- [x] Misskey extensions (_misskey_*)

#### 2.3 Activity Handlers
- [x] Create (Note作成)
- [x] Delete (Note削除)
- [x] Update (プロフィール更新)
- [x] Follow / Accept / Reject
- [x] Like (リアクション)
- [x] Announce (Renote)
- [x] Undo (各種取り消し)

#### 2.4 Delivery System
- [x] Outbox delivery queue
- [x] Shared inbox optimization
- [x] Retry strategy (exponential backoff)
- [x] Dead letter queue

```rust
// Phase 2 成果物例: ActivityPub Actor
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::actor::PersonType,
    protocol::public_key::PublicKey,
    traits::{Actor, Object},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPerson {
    #[serde(rename = "type")]
    pub kind: PersonType,
    pub id: ObjectId<DbUser>,
    pub preferred_username: String,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub inbox: Url,
    pub outbox: Url,
    pub followers: Url,
    pub following: Url,
    pub public_key: PublicKey,

    // Misskey extensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _misskey_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_cat: Option<bool>,
}

#[async_trait::async_trait]
impl Object for DbUser {
    type DataType = AppState;
    type Kind = ApPerson;
    type Error = AppError;

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(ApPerson {
            kind: PersonType::Person,
            id: self.ap_id(),
            preferred_username: self.username.clone(),
            name: self.name.clone(),
            summary: self.description.clone(),
            inbox: self.inbox_url(),
            outbox: self.outbox_url(),
            followers: self.followers_url(),
            following: self.following_url(),
            public_key: self.public_key(),
            _misskey_summary: self.description.clone(),
            is_cat: Some(self.is_cat),
        })
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        // Remote user creation/update logic
    }
}

#[async_trait::async_trait]
impl Actor for DbUser {
    fn id(&self) -> Url { self.ap_id().into_inner() }
    fn public_key_pem(&self) -> &str { &self.public_key_pem }
    fn private_key_pem(&self) -> Option<String> { self.private_key_pem.clone() }
    fn inbox(&self) -> Url { self.inbox_url() }
}
```

---

### Phase 3: REST API (3-4 weeks)

#### 3.1 Authentication API
- [x] POST /api/signup
- [x] POST /api/signin
- [x] POST /api/signout
- [x] Token refresh (regenerate-token)

#### 3.2 Notes API
- [x] POST /api/notes/create
- [x] POST /api/notes/delete
- [x] POST /api/notes/show
- [x] GET /api/notes/timeline
- [x] GET /api/notes/local-timeline
- [x] GET /api/notes/global-timeline

#### 3.3 Users API
- [x] POST /api/users/show
- [x] POST /api/following/create
- [x] POST /api/following/delete
- [x] POST /api/users/followers
- [x] POST /api/users/following

#### 3.4 Reactions API
- [x] POST /api/notes/reactions/create
- [x] POST /api/notes/reactions/delete

#### 3.5 Mastodon Compatible API (optional)
- [x] GET /api/v1/timelines/home
- [x] GET /api/v1/timelines/public
- [x] POST /api/v1/statuses
- [x] GET /api/v1/accounts/verify_credentials
- [x] GET /api/v1/accounts/:id

```rust
// Phase 3 成果物例: Axum router
use axum::{
    routing::{get, post},
    Router,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Notes
        .route("/api/notes/create", post(notes::create))
        .route("/api/notes/delete", post(notes::delete))
        .route("/api/notes/show", post(notes::show))
        .route("/api/notes/timeline", post(notes::timeline))

        // Users
        .route("/api/users/show", post(users::show))
        .route("/api/following/create", post(following::create))
        .route("/api/following/delete", post(following::delete))

        // ActivityPub
        .route("/users/{id}", get(activitypub::user))
        .route("/users/{id}/inbox", post(activitypub::inbox))
        .route("/inbox", post(activitypub::shared_inbox))
        .route("/.well-known/webfinger", get(activitypub::webfinger))
        .route("/.well-known/nodeinfo", get(nodeinfo::well_known))

        // Middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

---

### Phase 4: Background Jobs (2-3 weeks)

#### 4.1 Delivery Jobs
- [x] ActivityPub delivery worker
- [x] Retry with exponential backoff
- [x] Failed job handling (DLQ)

#### 4.2 Inbox Processing
- [x] Inbox activity processor
- [x] Signature verification
- [x] Rate limiting per instance

#### 4.3 Scheduled Jobs
- [x] Chart aggregation (hourly)
- [x] Expired muting cleanup
- [x] Instance health check
- [x] Old note cleanup (optional)

```rust
// Phase 4 成果物例: Delivery job
use apalis::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverActivityJob {
    pub activity: serde_json::Value,
    pub inbox: String,
    pub actor_id: String,
}

async fn deliver_activity(
    job: DeliverActivityJob,
    state: Data<AppState>,
) -> Result<(), Error> {
    let actor = state.user_repo.find_by_id(&job.actor_id).await?;

    let result = activitypub_federation::activity_sending::send_activity(
        job.activity,
        &actor,
        vec![Url::parse(&job.inbox)?],
        &state.federation_config,
    ).await;

    match result {
        Ok(_) => {
            tracing::info!(inbox = %job.inbox, "Activity delivered successfully");
            Ok(())
        }
        Err(e) => {
            tracing::warn!(inbox = %job.inbox, error = %e, "Delivery failed, will retry");
            Err(Error::Failed(e.into()))
        }
    }
}

// Worker setup
pub async fn start_workers(state: AppState) -> Result<()> {
    let storage = RedisStorage::new(state.redis.clone());

    let delivery_worker = WorkerBuilder::new("delivery")
        .layer(RetryLayer::new(RetryPolicy::retries(5)))
        .layer(TraceLayer::new())
        .data(state.clone())
        .backend(storage.clone())
        .build_fn(deliver_activity);

    Monitor::new()
        .register(delivery_worker)
        .run()
        .await?;

    Ok(())
}
```

---

### Phase 5: Streaming & Real-time (2 weeks)

#### 5.1 WebSocket Streaming
- [x] Home timeline stream
- [x] Local timeline stream
- [x] Global timeline stream
- [x] User-specific stream

#### 5.2 Server-Sent Events (alternative)
- [x] Notification stream
- [x] Timeline updates

#### 5.3 Redis Pub/Sub
- [x] Cross-instance event broadcasting
- [x] Real-time notification delivery

---

### Phase 6: MFM & Content (2 weeks)

#### 6.1 MFM Parser Integration
- [x] mfm.rs integration or custom parser
- [x] MFM to HTML conversion
- [x] HTML to MFM conversion
- [x] FEP-c16b compliance (Quote Posts)

#### 6.2 Content Processing
- [x] Mention extraction
- [x] Hashtag extraction
- [x] URL preview (Summaly equivalent)
- [x] Custom emoji handling

---

### Phase 7: Testing & Hardening (3-4 weeks)

#### 7.1 Unit Tests
- [x] Service layer tests
- [x] Repository tests
- [x] ActivityPub serialization tests

#### 7.2 Integration Tests
- [x] API endpoint tests
- [x] Database integration tests (MockDatabase)
- [x] Redis integration tests

#### 7.3 Federation Tests
- [x] Local federation (2 instances)
- [x] Mastodon compatibility (serialization tests)
- [x] Pleroma/Akkoma compatibility (serialization tests)
- [x] Existing Misskey compatibility (serialization tests)

#### 7.4 Performance Tests
- [x] Load testing (k6/wrk setup)
- [x] Memory profiling (guide & profiling profile)
- [x] Database query analysis

---

## Migration Strategy

### From Existing Misskey

```
┌─────────────────────────────────────────────────────────────┐
│  Option A: Fresh Start (推奨)                               │
├─────────────────────────────────────────────────────────────┤
│  1. 新規インスタンスとしてデプロイ                         │
│  2. ユーザーは新規アカウント作成                           │
│  3. 旧インスタンスからフォロー移行                         │
│                                                             │
│  メリット: クリーンな状態、マイグレーション不要            │
│  デメリット: 既存データ引き継ぎ不可                        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Option B: Data Migration                                   │
├─────────────────────────────────────────────────────────────┤
│  1. Misskey DBスキーマ → Rust版スキーマ変換ツール作成       │
│  2. データ移行スクリプト実行                               │
│  3. ActivityPub ID/URL維持                                  │
│                                                             │
│  メリット: 既存データ維持                                  │
│  デメリット: 複雑、ID体系の互換性リスク                    │
└─────────────────────────────────────────────────────────────┘
```

---

## Success Metrics

| 指標 | 目標 |
|------|------|
| API応答時間 (p50) | < 20ms |
| API応答時間 (p99) | < 100ms |
| メモリ使用量 | < 256MB (idle) |
| 起動時間 | < 2s |
| テストカバレッジ | > 70% |
| Mastodon互換性 | 主要機能100% |
| Misskey互換性 | コア機能100% |

---

## Risk Management

| リスク | 影響度 | 対策 |
|--------|-------|------|
| activitypub-federation の制限 | 中 | 必要なら fork/拡張 |
| Misskey固有機能の互換性 | 高 | 段階的実装、優先順位付け |
| MFM パーサーの互換性 | 中 | mfm.rs使用、バグ互換テスト |
| パフォーマンス目標未達 | 低 | プロファイリング、最適化 |
| 開発リソース不足 | 中 | スコープ調整、MVP優先 |

---

## Development Environment

### Required Tools

```bash
# Rust toolchain
rustup default stable
rustup component add rustfmt clippy

# Database
docker run -d --name postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:16
docker run -d --name redis -p 6379:6379 redis:7

# Development tools
cargo install cargo-watch sea-orm-cli sqlx-cli
```

### Running Locally

```bash
# Database setup
sea-orm-cli migrate up

# Development server (with auto-reload)
cargo watch -x run

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings
cargo fmt --check
```

---

## Timeline Overview

| Phase | 期間 | 内容 |
|-------|------|------|
| Phase 0 | Week 1-3 | Foundation, Infrastructure |
| Phase 1 | Week 4-7 | Core Entities |
| Phase 2 | Week 8-13 | ActivityPub Integration |
| Phase 3 | Week 14-17 | REST API |
| Phase 4 | Week 18-20 | Background Jobs |
| Phase 5 | Week 21-22 | Streaming |
| Phase 6 | Week 23-24 | MFM & Content |
| Phase 7 | Week 25-28 | Testing & Hardening |

**Total: 約7ヶ月** (フルタイム1人想定)

---

## Next Steps

1. [x] GitHub リポジトリ作成
2. [x] Cargo workspace 初期化
3. [x] CI/CD セットアップ
4. [x] Phase 0 開始

---

*Last Updated: 2025-12-11*
