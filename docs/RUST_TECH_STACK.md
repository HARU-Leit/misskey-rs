# Rust Tech Stack Reference (2025年12月版)

## Overview

このドキュメントはMisskey Rust Forkプロジェクトで使用する技術スタックの詳細リファレンスです。

---

## 1. Web Framework: Axum 0.8

### 基本情報

| 項目 | 内容 |
|------|------|
| バージョン | 0.8.7 |
| リリース日 | 2025年1月1日 (0.8.0) |
| 開発元 | Tokio チーム |
| ドキュメント | https://docs.rs/axum/0.8 |

### Cargo.toml

```toml
axum = { version = "0.8", features = ["macros"] }
axum-extra = { version = "0.10", features = [
    "typed-header",
    "query",
    "cookie",
    "multipart"
]}
tower = "0.5"
tower-http = { version = "0.6", features = [
    "cors",
    "trace",
    "timeout",
    "compression-gzip",
    "limit"
]}
```

### 0.8 破壊的変更

```rust
// パス構文の変更
// 0.7: /:id, /*path
// 0.8: /{id}, /{*path}

// Before
Router::new().route("/users/:id", get(handler))

// After
Router::new().route("/users/{id}", get(handler))
```

```rust
// async_trait 不要に
// Before (0.7)
#[async_trait]
impl<S> FromRequestParts<S> for MyExtractor { ... }

// After (0.8) - RPITIT使用
impl<S> FromRequestParts<S> for MyExtractor {
    type Rejection = MyError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // ...
    }
}
```

### 使用例

```rust
use axum::{
    extract::{Path, State, Json},
    routing::{get, post},
    Router,
};

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
    redis: fred::clients::RedisClient,
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<User>, AppError> {
    let user = state.db.find_user(&id).await?;
    Ok(Json(user))
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/users/{id}", get(get_user))
        .route("/notes", post(create_note))
        .with_state(state)
}
```

---

## 2. ActivityPub: activitypub-federation 0.6

### 基本情報

| 項目 | 内容 |
|------|------|
| バージョン | 0.6.5 |
| 開発元 | Lemmy Team |
| 本番実績 | Lemmy (大規模運用) |
| ドキュメント | https://docs.rs/activitypub_federation/0.6 |
| ライセンス | AGPL-3.0 |

### Cargo.toml

```toml
activitypub_federation = { version = "0.6", features = ["axum"] }
```

### 主要トレイト

```rust
// 1. Object トレイト - データ型変換
#[async_trait]
pub trait Object: Sized + Send + 'static {
    type DataType: Clone + Send + Sync + 'static;
    type Kind: serde::de::DeserializeOwned + serde::Serialize + Send + 'static;
    type Error: std::error::Error + Send + Sync + 'static;

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error>;

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error>;

    async fn from_json(
        json: Self::Kind,
        data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error>;
}

// 2. Actor トレイト - ユーザー/アカウント
pub trait Actor: Object {
    fn id(&self) -> Url;
    fn public_key_pem(&self) -> &str;
    fn private_key_pem(&self) -> Option<String>;
    fn inbox(&self) -> Url;
}

// 3. ActivityHandler トレイト - アクティビティ処理
#[async_trait]
pub trait ActivityHandler {
    type DataType: Clone + Send + Sync + 'static;
    type Error: std::error::Error + Send + Sync + 'static;

    fn id(&self) -> &Url;
    fn actor(&self) -> &Url;

    async fn verify(
        &self,
        data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error>;

    async fn receive(
        self,
        data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error>;
}
```

### Axum 統合

```rust
use activitypub_federation::axum::{
    inbox::receive_activity,
    json::FederationJson,
};

async fn inbox(
    State(state): State<AppState>,
    activity_data: ActivityData,
) -> Result<(), AppError> {
    receive_activity::<WithContext<Activity>, User, AppState>(activity_data, &state.federation_config)
        .await
}

fn federation_routes() -> Router<AppState> {
    Router::new()
        .route("/inbox", post(inbox))
        .route("/users/{name}/inbox", post(inbox))
        .route("/users/{name}", get(user_json))
}
```

### Misskey拡張プロパティ

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApNote {
    #[serde(rename = "type")]
    pub kind: NoteType,
    pub id: ObjectId<DbNote>,
    pub attributed_to: ObjectId<DbUser>,
    pub content: String,
    pub published: DateTime<Utc>,

    // Misskey extensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _misskey_content: Option<String>,  // MFM形式

    #[serde(skip_serializing_if = "Option::is_none")]
    pub _misskey_quote: Option<Url>,  // 引用

    #[serde(skip_serializing_if = "Option::is_none")]
    pub _misskey_reaction: Option<String>,  // リアクション
}
```

---

## 3. ORM: SeaORM 1.1

### 基本情報

| 項目 | 内容 |
|------|------|
| バージョン | 1.1.19 (stable) |
| 2.0 状態 | Release Candidate |
| ドキュメント | https://www.sea-ql.org/SeaORM/docs/ |

### Cargo.toml

```toml
sea-orm = { version = "1.1", features = [
    "sqlx-postgres",
    "runtime-tokio-rustls",
    "macros",
    "with-chrono",
    "with-json",
    "with-uuid"
]}
sea-orm-migration = "1.1"
```

### エンティティ定義

```rust
// src/models/user.rs
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    #[sea_orm(unique)]
    pub username: String,

    pub username_lower: String,

    /// NULL = local user, Some = remote user
    pub host: Option<String>,

    #[sea_orm(unique)]
    pub token: Option<String>,

    pub name: Option<String>,

    pub followers_count: i32,
    pub following_count: i32,
    pub notes_count: i32,

    pub is_bot: bool,
    pub is_cat: bool,
    pub is_locked: bool,
    pub is_suspended: bool,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::note::Entity")]
    Notes,

    #[sea_orm(has_one = "super::user_profile::Entity")]
    Profile,

    #[sea_orm(has_one = "super::user_keypair::Entity")]
    Keypair,
}

impl ActiveModelBehavior for ActiveModel {}
```

### クエリ例

```rust
// 基本クエリ
let user = User::find_by_id(id).one(&db).await?;

// 条件付きクエリ
let users = User::find()
    .filter(user::Column::Host.is_null())  // ローカルユーザーのみ
    .filter(user::Column::IsSuspended.eq(false))
    .order_by_desc(user::Column::CreatedAt)
    .limit(20)
    .all(&db)
    .await?;

// リレーション込み
let user_with_profile = User::find_by_id(id)
    .find_also_related(UserProfile)
    .one(&db)
    .await?;

// INSERT
let new_user = user::ActiveModel {
    id: Set(generate_id()),
    username: Set(username),
    username_lower: Set(username.to_lowercase()),
    host: Set(None),
    ..Default::default()
};
let result = User::insert(new_user).exec(&db).await?;

// UPDATE
let mut user: user::ActiveModel = user.into();
user.name = Set(Some(new_name));
user.update(&db).await?;
```

### マイグレーション

```rust
// migration/src/m20250101_000001_create_user_table.rs
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(ColumnDef::new(User::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(User::Username).string().not_null().unique_key())
                    .col(ColumnDef::new(User::UsernameLower).string().not_null())
                    .col(ColumnDef::new(User::Host).string())
                    .col(ColumnDef::new(User::Token).string().unique_key())
                    .col(ColumnDef::new(User::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_username_lower_host")
                    .table(User::Table)
                    .col(User::UsernameLower)
                    .col(User::Host)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(User::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum User {
    Table,
    Id,
    Username,
    UsernameLower,
    Host,
    Token,
    CreatedAt,
}
```

---

## 4. Redis: fred 10

### 基本情報

| 項目 | 内容 |
|------|------|
| バージョン | 10.1.0 |
| 対応 | Redis, Valkey |
| ドキュメント | https://docs.rs/fred/10 |

### Cargo.toml

```toml
fred = { version = "10", features = [
    "subscriber-client",
    "redis-json",
    "enable-rustls"
]}
```

### 接続設定

```rust
use fred::prelude::*;

async fn create_redis_client(config: &Config) -> Result<RedisClient> {
    let redis_config = RedisConfig::from_url(&config.redis_url)?;

    let client = Builder::from_config(redis_config)
        .with_performance_config(|perf| {
            perf.auto_pipeline = true;
            perf.max_command_attempts = 3;
        })
        .build()?;

    client.init().await?;
    Ok(client)
}
```

### 使用例

```rust
// 基本操作
client.set("key", "value", None, None, false).await?;
let value: Option<String> = client.get("key").await?;

// Expire付き
client.set("session:123", session_data, Some(Expiration::EX(3600)), None, false).await?;

// JSON (RedisJSON)
client.json_set("user:123", "$", &user_json, None).await?;
let user: User = client.json_get("user:123", None).await?;

// Pub/Sub
let subscriber = client.subscriber_client();
subscriber.subscribe("timeline:home").await?;

let mut stream = subscriber.message_rx();
while let Ok(message) = stream.recv().await {
    println!("Received: {:?}", message);
}

// パイプライン
let pipeline = client.pipeline();
pipeline.incr("counter").await?;
pipeline.expire("counter", 3600).await?;
pipeline.all().await?;
```

### 接続プール設計

```rust
// 用途別クライアント（Misskey方式）
pub struct RedisClients {
    pub default: RedisClient,      // 汎用
    pub cache: RedisClient,        // キャッシュ
    pub timeline: RedisClient,     // タイムライン
    pub queue: RedisClient,        // ジョブキュー
    pub pubsub: SubscriberClient,  // Pub/Sub
}

impl RedisClients {
    pub async fn new(config: &Config) -> Result<Self> {
        // 各クライアントを初期化
    }
}
```

---

## 5. Job Queue: Apalis 0.7

### 基本情報

| 項目 | 内容 |
|------|------|
| バージョン | 0.7.1 (stable), 1.0.0-beta.2 |
| バックエンド | Redis, PostgreSQL, SQLite |
| ドキュメント | https://docs.rs/apalis/0.7 |

### Cargo.toml

```toml
apalis = { version = "0.7", features = ["limit", "tracing", "retry"] }
apalis-redis = "0.7"
# or
apalis-sql = { version = "0.7", features = ["postgres"] }
```

### ジョブ定義

```rust
use apalis::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverActivityJob {
    pub activity_id: String,
    pub inbox_url: String,
    pub actor_id: String,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInboxJob {
    pub activity_json: serde_json::Value,
    pub signature: String,
    pub instance_host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendNotificationJob {
    pub user_id: String,
    pub notification_type: String,
    pub data: serde_json::Value,
}
```

### ワーカー実装

```rust
use apalis::prelude::*;
use apalis_redis::RedisStorage;

async fn deliver_activity(
    job: DeliverActivityJob,
    ctx: JobContext,
    state: Data<AppState>,
) -> Result<(), Error> {
    tracing::info!(
        job_id = %ctx.id(),
        inbox = %job.inbox_url,
        "Delivering activity"
    );

    let activity = state.activity_repo.find(&job.activity_id).await
        .map_err(|e| Error::Failed(e.into()))?;

    let actor = state.user_repo.find(&job.actor_id).await
        .map_err(|e| Error::Failed(e.into()))?;

    state.federation
        .send_activity(&activity, &actor, &job.inbox_url)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "Delivery failed");
            Error::Failed(e.into())
        })?;

    Ok(())
}

async fn process_inbox(
    job: ProcessInboxJob,
    state: Data<AppState>,
) -> Result<(), Error> {
    // ActivityPub inbox processing
    state.federation
        .process_incoming(&job.activity_json, &job.signature)
        .await
        .map_err(|e| Error::Failed(e.into()))
}
```

### ワーカー起動

```rust
use apalis::{layers::*, prelude::*};
use apalis_redis::RedisStorage;

pub async fn start_workers(state: AppState) -> Result<()> {
    let redis_storage = RedisStorage::new(state.redis.clone());

    // Delivery worker (concurrency: 10, retry: 5)
    let delivery_worker = WorkerBuilder::new("delivery")
        .layer(RetryLayer::new(RetryPolicy::retries(5)))
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(TraceLayer::new())
        .layer(ConcurrencyLimitLayer::new(10))
        .data(state.clone())
        .backend(redis_storage.clone())
        .build_fn(deliver_activity);

    // Inbox worker (concurrency: 5)
    let inbox_worker = WorkerBuilder::new("inbox")
        .layer(ConcurrencyLimitLayer::new(5))
        .layer(TraceLayer::new())
        .data(state.clone())
        .backend(redis_storage.clone())
        .build_fn(process_inbox);

    Monitor::new()
        .register(delivery_worker)
        .register(inbox_worker)
        .on_event(|e| tracing::info!("Worker event: {:?}", e))
        .run()
        .await?;

    Ok(())
}
```

### ジョブ投入

```rust
impl NoteService {
    pub async fn create(&self, input: CreateNoteInput) -> Result<Note> {
        let note = self.repo.create(input).await?;

        // フォロワーへの配信ジョブを投入
        let followers = self.follow_repo.get_followers(&note.user_id).await?;

        for follower in followers {
            if let Some(inbox) = follower.inbox_url {
                let job = DeliverActivityJob {
                    activity_id: note.activity_id.clone(),
                    inbox_url: inbox,
                    actor_id: note.user_id.clone(),
                    retry_count: 0,
                };
                self.queue.push(job).await?;
            }
        }

        Ok(note)
    }
}
```

---

## 6. MFM Parser: mfm.rs

### 基本情報

| 項目 | 内容 |
|------|------|
| リポジトリ | https://codeberg.org/87flowers/mfm.rs |
| 互換性 | mfm.js バグ互換 |

### Cargo.toml

```toml
# Gitから直接
mfm = { git = "https://codeberg.org/87flowers/mfm.rs" }
```

### 使用例

```rust
use mfm::{parse, to_html, Node};

// MFM → AST
let nodes: Vec<Node> = parse("Hello **world** :emoji:");

// MFM → HTML
let html = to_html("Hello **world**");
// => "Hello <b>world</b>"

// HTML → MFM (逆変換)
let mfm = from_html("<b>world</b>");
// => "**world**"
```

---

## 7. その他ユーティリティ

### エラーハンドリング

```toml
thiserror = "2"
anyhow = "1"
```

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Note not found: {0}")]
    NoteNotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error")]
    Database(#[from] sea_orm::DbErr),

    #[error("Redis error")]
    Redis(#[from] fred::error::RedisError),

    #[error("Federation error: {0}")]
    Federation(String),

    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}
```

### Logging/Tracing

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}

// 使用
tracing::info!(user_id = %user.id, "User created");
tracing::warn!(error = %e, "Failed to deliver activity");
```

### バリデーション

```toml
validator = { version = "0.19", features = ["derive"] }
```

```rust
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct CreateNoteInput {
    #[validate(length(min = 1, max = 3000))]
    pub text: Option<String>,

    #[validate(length(max = 100))]
    pub cw: Option<String>,

    pub visibility: Visibility,

    #[validate(length(max = 16))]
    pub file_ids: Vec<String>,
}

// Axum extractor
async fn create_note(
    State(state): State<AppState>,
    Json(input): Json<CreateNoteInput>,
) -> Result<Json<Note>, AppError> {
    input.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    // ...
}
```

### ID生成

```toml
ulid = "1"
# or
uuid = { version = "1", features = ["v7"] }
```

```rust
use ulid::Ulid;

pub fn generate_id() -> String {
    Ulid::new().to_string().to_lowercase()
}

// または UUID v7 (時間順序付き)
use uuid::Uuid;

pub fn generate_id() -> String {
    Uuid::now_v7().to_string()
}
```

---

## Quick Reference

### 依存関係一覧

```toml
[dependencies]
# Web
axum = { version = "0.8", features = ["macros"] }
axum-extra = { version = "0.10", features = ["typed-header", "cookie"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "timeout"] }

# ActivityPub
activitypub_federation = { version = "0.6", features = ["axum"] }

# Database
sea-orm = { version = "1.1", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros", "with-chrono", "with-json", "with-uuid"] }
sea-orm-migration = "1.1"

# Redis
fred = { version = "10", features = ["subscriber-client", "redis-json"] }

# Job Queue
apalis = { version = "0.7", features = ["limit", "tracing", "retry"] }
apalis-redis = "0.7"

# Runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP Client
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Validation
validator = { version = "0.19", features = ["derive"] }

# Error Handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "v7", "serde"] }
ulid = "1"
url = { version = "2", features = ["serde"] }
async-trait = "0.1"

# Security
argon2 = "0.5"
jsonwebtoken = "9"
```

---

*Last Updated: 2025-12-08*
