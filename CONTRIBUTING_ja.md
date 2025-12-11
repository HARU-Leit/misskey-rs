# Contributing Guidelines / 実装ルール

このドキュメントは misskey-rs プロジェクトにおける厳格な実装ルールを定めます。
すべてのコントリビューターはこれらのルールに従う必要があります。

---

## 0. 基本方針

### 0.1 言語規約

| 対象 | 言語 |
|------|------|
| コード（変数名・関数名・型名） | **英語** |
| テスト関数名 | **英語** |
| コードコメント | 英語推奨、日本語可 |
| ドキュメント（README, CONTRIBUTING等） | **日本語** |
| コミットメッセージ | **英語** |

### 0.2 MSRV (Minimum Supported Rust Version)

- **MSRV**: `1.85.0` (Rust 2024 Edition)
- `Cargo.toml` の `rust-version` フィールドで明示
- MSRV を上げる場合は Breaking Change として扱う

### 0.3 ランタイム

- **非同期ランタイム**: Tokio（`tokio = { features = ["full"] }`）
- 他のランタイム（async-std等）との互換性は保証しない

---

## 1. コーディング規約

### 1.1 命名規則

| 対象 | 規則 | 例 |
|------|------|-----|
| 変数・関数・モジュール | `snake_case` | `get_user`, `user_id` |
| 型・トレイト・構造体・enum | `UpperCamelCase` | `UserService`, `AccountError` |
| enum variants | `UpperCamelCase` | `NotFound`, `InvalidInput` |
| 定数・static | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_TIMEOUT` |
| 型パラメータ | 1文字大文字または説明的な名前 | `T`, `E`, `Item` |
| ライフタイム | 短い小文字 | `'a`, `'de`, `'ctx` |

**追加ルール**:
- 頭字語は CamelCase では一語扱い: `Uuid` (not `UUID`), `HttpClient` (not `HTTPClient`)
- getter に `get_` プレフィックス不要: `fn value(&self)` (not `fn get_value(&self)`)
- `is_`, `has_`, `can_` は bool を返す関数に使用

### 1.2 フォーマット

- **必須**: `rustfmt` を使用（CI で強制）
- 行幅: 最大 **100 文字**
- インデント: **4 スペース**（タブ禁止）
- 末尾カンマ: 複数行リストでは必須
- 末尾空白: 禁止

```bash
# フォーマット確認
cargo fmt --check

# 自動フォーマット
cargo fmt
```

### 1.3 ドキュメント

- 公開 API には必ずドキュメントコメント (`///`) を付ける
- 最初の行は1文で簡潔に説明
- 例を含める場合は `# Examples` セクションを使用
- `unsafe` には `# Safety` セクション必須
- エラーを返す場合は `# Errors` セクション推奨

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

## 2. アーキテクチャ

### 2.1 クレート構造（Pragmatic Hexagonal Architecture）

```
crates/
├── common/      # 共通ユーティリティ
├── core/        # ドメインロジック・サービス・トレイト定義（ポート）
├── db/          # データベースアダプター（リポジトリ実装）
├── api/         # HTTP API アダプター
├── federation/  # ActivityPub アダプター
├── queue/       # ジョブキューアダプター
├── mfm/         # MFMパーサー
└── server/      # アプリケーションエントリポイント
```

### 2.2 依存関係ルール

```
server → api, queue, federation, db, core, common
api, queue, federation → core, db, common
db → core, common
core → common
common → (外部依存のみ)
```

**設計上の決定事項**:

> **Note**: このプロジェクトでは、実用性を優先した「Pragmatic Hexagonal Architecture」を採用しています。
> 厳密な Hexagonal Architecture ではアダプター層（api, federation等）は `core` のみに依存しますが、
> 本プロジェクトでは `db` への直接依存を許容しています。
>
> **理由**:
> - 読み取りクエリの最適化が容易（不要なサービス層を経由しない）
> - 実装コストとプロジェクト規模のバランス
> - 必要に応じて段階的に厳密化可能

**禁止事項**:
- `core` からアダプター層（`db`, `api`, `federation`, `queue`）への依存
- アダプター間の直接依存（`api` → `federation` など）
- 循環依存

### 2.3 トレイトによる抽象化

- ドメイン層（`core`）でトレイト（ポート）を定義
- アダプター層でトレイトを実装
- 依存性注入で実装を切り替え可能に

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

## 3. エラーハンドリング

### 3.1 クレート選択

| レイヤー | クレート | 用途 |
|---------|---------|------|
| ライブラリ層 (`core`, `db`, `api`等) | `thiserror` | 型付きエラー定義 |
| アプリケーション層 (`server`) | `anyhow` | エラー伝播・コンテキスト |

> **Note**: `db` はアーキテクチャ上はアダプター層ですが、エラーハンドリングの観点では
> 呼び出し元がエラー種別でハンドリングを分岐する必要があるため、`thiserror` を使用します。

### 3.2 エラー型設計

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

### 3.3 コンテキスト付与

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

### 3.4 禁止事項

- `unwrap()` の使用（`expect()` で理由を明記するか、`?` を使用）
- エラーの握りつぶし（`let _ = ...` でエラーを無視）
- `panic!` の乱用（回復不能な状態のみ）

---

## 4. セキュリティ

### 4.1 必須チェック（CI で強制）

```bash
# Clippy 全警告をエラー扱い
cargo clippy -- -D warnings

# 依存関係の脆弱性チェック
cargo audit

# unsafe コード禁止（Cargo.toml で設定済み）
# [lints.rust]
# unsafe_code = "forbid"
```

### 4.2 入力検証

**Newtype パターンの使用**:

```rust
/// Validated email address.
#[derive(Debug, Clone)]
pub struct ValidatedEmail(String);

impl ValidatedEmail {
    pub fn new(email: &str) -> Result<Self, ValidationError> {
        // 実際の実装では validator クレートや email_address クレートを使用
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

> **推奨**: 実装時は `validator` クレート（derive マクロ）や `email_address` クレート等の
> 専用ライブラリを使用してください。

### 4.3 SQLインジェクション対策

```rust
// 必須: パラメータ化クエリを使用（Sea-ORM）
User::find_by_id(user_id).one(&db).await?;

// 禁止: 文字列結合によるクエリ構築
// let query = format!("SELECT * FROM users WHERE id = {}", user_id); // NG!
```

### 4.4 認証・セッション

- パスワードハッシュ: Argon2 使用
- JWT: 環境変数からシークレットキーを取得
- Cookie: `HttpOnly`, `Secure`, `SameSite=Strict` フラグ必須
- セッション: 適切なタイムアウト設定

### 4.5 XSS対策

```rust
use ammonia::clean;

let safe_html = clean(user_input);
```

---

## 5. テスト

### 5.1 カバレッジ目標

- **最低要件**: 80% 以上
- 重要なビジネスロジックは 90% 以上を目指す

### 5.2 テスト種別

| 種別 | 場所 | 目的 |
|------|------|------|
| ユニットテスト | `#[cfg(test)]` モジュール | 内部関数の検証 |
| 統合テスト | `tests/` ディレクトリ | 公開 API の検証 |
| ドキュメントテスト | `///` 内のコード例 | API 使用例の動作確認 |

### 5.3 テスト命名規則

**形式**: `test_<function_name>_<condition>_<expected_result>`

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

### 5.4 非同期テスト

```rust
#[tokio::test]
async fn test_async_function() {
    // ...
}
```

### 5.5 カバレッジ計測

```bash
# cargo-tarpaulin を使用（Linux x86_64）
cargo tarpaulin --out Html --output-dir coverage/

# または grcov（クロスプラットフォーム）
RUSTFLAGS="-C instrument-coverage" cargo test
grcov . -s . --binary-path ./target/debug/ -o ./coverage/
```

---

## 6. ロギング / トレーシング

### 6.1 クレート

- **tracing**: 構造化ログ・スパン
- **tracing-subscriber**: ログ出力設定

### 6.2 ログレベル指針

| レベル | 用途 |
|--------|------|
| `error` | 復旧不能なエラー、即座の対応が必要 |
| `warn` | 潜在的な問題、注意が必要だが動作は継続 |
| `info` | 重要な状態変化（サーバー起動、接続確立等） |
| `debug` | 開発時のデバッグ情報 |
| `trace` | 詳細なトレース情報（パフォーマンス影響あり） |

### 6.3 スパンの使用

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

### 6.4 禁止事項

- `println!` / `eprintln!` の使用（`tracing` マクロを使用）
- 機密情報（パスワード、トークン等）のログ出力
- 本番環境での `trace` レベルの常時有効化

---

## 7. 設定管理

### 7.1 優先順位（高い方が優先）

1. 環境変数
2. `.env` ファイル（開発用、Git管理外）
3. 設定ファイル（`config/` ディレクトリ）
4. デフォルト値

### 7.2 環境変数命名規則

```
MISSKEY_<CATEGORY>_<NAME>
```

例:
- `MISSKEY_DATABASE_URL`
- `MISSKEY_REDIS_URL`
- `MISSKEY_JWT_SECRET`

### 7.3 機密情報

- **必須**: 環境変数で管理
- **禁止**: 設定ファイルやコードへの直接記述
- `.env.example` にはプレースホルダーのみ記載

```bash
# .env.example
MISSKEY_DATABASE_URL=postgres://user:password@localhost/misskey
MISSKEY_JWT_SECRET=your-secret-key-here
```

---

## 8. Git / コミット規約

### 8.1 Conventional Commits

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### 8.2 Type 一覧

| type | 説明 |
|------|------|
| `feat` | 新機能追加 |
| `fix` | バグ修正 |
| `docs` | ドキュメントのみの変更 |
| `style` | コードの意味に影響しない変更（フォーマット等） |
| `refactor` | バグ修正でも機能追加でもないコード変更 |
| `perf` | パフォーマンス改善 |
| `test` | テストの追加・修正 |
| `build` | ビルドシステム・外部依存関係の変更 |
| `ci` | CI 設定の変更 |
| `chore` | その他の変更（ビルドやドキュメントに影響しない） |

### 8.3 Scope（このプロジェクト用）

- `common`, `core`, `db`, `api`, `federation`, `queue`, `mfm`, `server`
- `deps` (依存関係)
- `ci` (CI/CD)

### 8.4 コミットメッセージルール

1. **件名は50文字以内**
2. **先頭のみ大文字**（型の後の説明部分）
3. **件名末尾にピリオドなし**
4. **命令形を使用**: `Add feature` (not `Added feature`)
5. **本文は72文字で折り返し**
6. **何を・なぜを書く**（どうやっては書かない）

### 8.5 例

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

- フッターに `BREAKING CHANGE:` を追加
- または型に `!` を付ける: `feat!:` or `feat(api)!:`

---

## 9. PR レビュープロセス

### 9.1 レビュー要件

| PR サイズ | 必要承認数 | 備考 |
|----------|-----------|------|
| 小（〜100行） | 1名 | typo修正、小さなバグ修正 |
| 中（100〜500行） | 1名 | 機能追加、リファクタリング |
| 大（500行〜） | 2名 | 大規模機能、アーキテクチャ変更 |

### 9.2 レビュー観点

- [ ] **機能性**: 要件を満たしているか
- [ ] **コード品質**: 命名、可読性、重複
- [ ] **セキュリティ**: 入力検証、認証・認可
- [ ] **テスト**: 適切なテストが追加されているか
- [ ] **パフォーマンス**: N+1問題、不要なクローン
- [ ] **エラーハンドリング**: 適切なエラー型、コンテキスト
- [ ] **ドキュメント**: 公開APIのドキュメント

### 9.3 セルフマージ禁止

- 自分の PR を自分で承認してマージしない
- 緊急時は事後レビューを必須とする

---

## 10. CI/CD チェックリスト

PR がマージされるためには、以下のすべてをパスする必要があります：

- [ ] `cargo fmt --check` (フォーマット)
- [ ] `cargo clippy -- -D warnings` (静的解析)
- [ ] `cargo test` (全テスト)
- [ ] `cargo audit` (セキュリティ)
- [ ] カバレッジ 80% 以上
- [ ] コミットメッセージが Conventional Commits 形式
- [ ] 必要なレビュー承認

---

## 11. 参考リンク

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [thiserror documentation](https://docs.rs/thiserror)
- [anyhow documentation](https://docs.rs/anyhow)
- [tracing documentation](https://docs.rs/tracing)
