# misskey-rs TODO リスト

優先順位付きの統合タスクリスト。全ての機能要望・改善項目を一元管理。

*Last Updated: 2025-12-11* (チャンネルタイムラインのストリーミング実装)

---

## 凡例

| 記号 | 意味 |
|------|------|
| 🔴 | 最優先（本家との差別化・重要要望） |
| 🟡 | 中優先（UX改善・機能拡張） |
| 🟢 | 低優先（あると嬉しい機能） |
| ✅ | 完了 |
| 🔧 | 進行中/部分実装 |

---

## Tier 1: 最優先タスク 🔴

### フェデレーション完全対応

| タスク | 状況 | 参照 |
|--------|------|------|
| ActivityPub Update activity対応（ノート編集連合） | ✅ 完了 | update.rs |
| いいね/リアクションの適切な連合（Mastodon/Pleroma向け） | ✅ 完了 | emoji_react.rs, like.rs |
| 引用リノートのMastodon連合（FEP-e232対応） | ✅ 完了 | note.rs |
| チャンネルのフェデレーション（Group actor） | 未実装 | COMMUNITY_FEATURES.md |
| ActivityPub Move activity対応（アカウント移行） | ✅ 完了 | move_activity.rs, move_processor.rs |

### Mastodon互換API完全対応

| タスク | 状況 | 参照 |
|--------|------|------|
| メディア添付API | ✅ 完了 | media.rs |
| OAuth 2.0完全対応 | ✅ 完了 | - |
| Statuses API (CRUD/context) | ✅ 完了 | statuses.rs |
| Accounts API (follow/block/mute) | ✅ 完了 | accounts.rs |
| Favourites API | ✅ 完了 | favourites.rs |
| Blocks/Mutes API | ✅ 完了 | blocks.rs, mutes.rs |
| Bookmarks API | ✅ 完了 | bookmarks.rs |
| Timelines API | ✅ 完了 | timelines.rs |

### 管理機能強化

| タスク | 状況 | 参照 |
|--------|------|------|
| ローカル/リモート別文字数制限 | ✅ 完了 | DB/エンティティ/API実装済 |
| 登録承認必須モード | ✅ 完了 | DB/エンティティ/API実装済 |
| メディアNSFW強制マーク | ✅ 完了 | DB/エンティティ/API実装済 |

---

## Tier 2: 中優先タスク 🟡

### パフォーマンス・インフラ

| タスク | 状況 | 参照 |
|--------|------|------|
| URLプレビューキャッシュ | ✅ 完了 | url_preview_cache.rs |
| Redis分散カウンター（レート制限） | ✅ 完了 | rate_limit.rs |
| 読み取りレプリカ対応 | 未実装 | COMMUNITY_FEATURES.md |

### タイムライン・フィード

| タスク | 状況 | 参照 |
|--------|------|------|
| バブルタイムライン | ✅ 完了 | timelines.rs, meta_settings |
| チャンネルタイムラインのストリーミング | ✅ 完了 | streaming.rs, pubsub.rs |
| Bot非表示オプション | ✅ 完了 | user_profile, notes.rs |

### 検索・発見

| タスク | 状況 | 参照 |
|--------|------|------|
| ドライブ検索（ファイル名/説明） | ✅ 完了 | drive.rs |
| インスタンス指定アンテナ | ✅ 完了 | antenna.rs (AntennaSource::Instances) |
| Meilisearch連携 | 未実装 | FORK_FEATURES.md |

### UI/UX API対応

| タスク | 状況 | 参照 |
|--------|------|------|
| ワンボタンいいね（Like/Reaction分離） | 未実装 | FORK_FEATURES.md |
| デフォルトリアクション設定 | ✅ 完了 | user_profile, users.rs |
| ユーザー単位Authorized Fetch | 未実装 | FORK_FEATURES.md |

### データ管理

| タスク | 状況 | 参照 |
|--------|------|------|
| ノートエクスポート（JSON/CSV） | ✅ 完了 | account.rs |
| ブロック/ミュートエクスポート | ✅ 完了 | account.rs |
| Mastodon形式インポート | ✅ 完了 | account.rs |

---

## Tier 3: 低優先タスク 🟢

### 拡張機能

| タスク | 状況 | 参照 |
|--------|------|------|
| スマートクリップ（条件ベース自動追加） | 未実装 | MISSING_FEATURES.md |
| クリップ間ノート移動/コピー | 未実装 | MISSING_FEATURES.md |
| 繰り返し投稿（日次/週次/月次） | 未実装 | MISSING_FEATURES.md |
| フィルターグループ（プリセット） | 未実装 | MISSING_FEATURES.md |
| アンテナ通知設定 | 未実装 | MISSING_FEATURES.md |

### グループ拡張

| タスク | 状況 | 参照 |
|--------|------|------|
| グループ内限定ノート | 未実装 | MISSING_FEATURES.md |
| グループDM（グループチャット） | 未実装 | MISSING_FEATURES.md |
| グループのActivityPub対応 | 未実装 | MISSING_FEATURES.md |

### セキュリティ強化

| タスク | 状況 | 参照 |
|--------|------|------|
| 信頼済みデバイス管理 | 未実装 | MISSING_FEATURES.md |
| ログイン通知（新規デバイス） | 未実装 | MISSING_FEATURES.md |
| セッション一覧と強制ログアウト | 未実装 | MISSING_FEATURES.md |

### メディア処理

| タスク | 状況 | 参照 |
|--------|------|------|
| image-rs完全統合 | 🔧 インターフェース設計済 | MISSING_FEATURES.md |
| 遅延処理（バックグラウンド変換） | 未実装 | MISSING_FEATURES.md |
| 外部ストレージ対応強化（R2, B2, MinIO） | 未実装 | MISSING_FEATURES.md |

### その他

| タスク | 状況 | 参照 |
|--------|------|------|
| プロフィール背景画像 | 未実装 | FORK_FEATURES.md |
| Listenbrainz統合 | 未実装 | FORK_FEATURES.md |
| robots.txt管理者設定 | 未実装 | FORK_FEATURES.md |

---

## Phase 7: 独自機能（将来計画）

これらは差別化機能として将来検討：

| タスク | 状況 | 参照 |
|--------|------|------|
| GraphQL API | 未着手 | MISSING_FEATURES.md |
| プラグインシステム（WASM） | 未着手 | MISSING_FEATURES.md |
| Rhaiスクリプティング | 未着手 | RHAI_SCRIPTING.md |
| AI/LLM統合 | 未着手 | MISSING_FEATURES.md |
| 分析・統計ダッシュボード | 未着手 | MISSING_FEATURES.md |
| 高度なモデレーション（AI支援） | 未着手 | MISSING_FEATURES.md |

---

## 残課題（バグ修正・技術的負債）

| タスク | ファイル | 参照 |
|--------|----------|------|
| メッセージングのブロックチェック | ✅ 完了 | messaging.rs:87-96 |
| ドライブのファイル実体削除 | ✅ 完了 | drive.rs:252-264 |
| NodeInfo実統計取得 | ✅ 完了 | nodeinfo.rs |
| Mastodon API base_url設定 | ✅ TODO残 | timelines.rs |
| フォルダ循環参照チェック | 未実装 | drive.rs:475 |
| 非フォロワーからのメッセージ制限 | 未実装 | messaging.rs:98 |

---

## 完了済み機能サマリー

### Phase 1-6 (100%完了)
- クリップ、ピン留め、予約投稿、ワードフィルター、ノート編集
- アンテナ、チャンネル、インスタンスブロック
- 2FA、レート制限、OAuth 2.0、WebAuthn
- ページ、ギャラリー、翻訳、プッシュ通知、メール通知、メディア処理
- アカウント移行/削除/エクスポート/インポート
- グループ機能、Webhook

### 上位互換化 (100%完了)
- カウンター直接更新、再帰CTE、リモートアクターキャッシュ
- リプレイ攻撃防止、インスタンス別レート制限
- PostgreSQL全文検索、高度な検索フィルタ
- 通知タイプフィルタ、トレンドノート、クリップ内検索
- タイムラインワードフィルター、チャンネルタイムライン
- 2FA/WebAuthnログイン検証、プッシュ通知ジョブサービス

### 管理機能強化 (100%完了)
- ローカル/リモート別文字数制限（`max_note_text_length` / `max_remote_note_text_length`）
- 登録承認必須モード（`require_registration_approval` + `registration_approval`テーブル）
- メディアNSFW強制マーク（`force_nsfw_media`）
- 管理APIエンドポイント（`/admin/meta`, `/admin/registration-approvals/*`）

### Mastodon互換API (100%完了)
- メディア添付API（アップロード/取得/更新）
- Statuses API（作成/取得/削除/コンテキスト）
- Accounts API（フォロー/ブロック/ミュート/関係性）
- Favourites/Bookmarks API
- Blocks/Mutes リスト取得
- Timelines API（home/public）

### ActivityPub拡張 (2025-12-11)
- **ActivityPub Update activity対応** - ローカルノート編集時にUpdate activityを配信、リモートからのUpdate activity受信でノート更新
- **EmojiReact Activity対応** - Pleroma/Akkoma形式のエモジリアクション受信に対応。Like Activityに`content`フィールドを追加してPleroma互換性向上
- **ActivityPub Move activity対応** - アカウント移行（FEP-7628）をサポート。`MoveActivity`定義、`MoveProcessor`による受信処理、`AccountService.migrate_account`による送信処理を実装。`movedTo`/`alsoKnownAs`フィールドをApPersonに追加し、移行先アカウントの検証（alsoKnownAs確認）も実装

### パフォーマンス最適化 (2025-12-11)
- **URLプレビューキャッシュ** - Redis-backed caching for URL previews. 24時間TTLでキャッシュ、失敗したURLは1時間ネガティブキャッシュ。`get_or_fetch()`メソッドで自動的にキャッシュ/フェッチを管理

### タイムライン拡張 (2025-12-11)
- **バブルタイムライン** - ローカルユーザー＋ホワイトリストに登録されたリモートインスタンスからの公開ノートを表示。`meta_settings.bubble_instances`で信頼インスタンスを設定可能。Mastodon API `/api/v1/timelines/bubble` エンドポイントを追加

### フェデレーション拡張 (2025-12-11)
- **FEP-e232/FEP-c16b 引用リノート連合** - 引用リノート（テキスト付きリノート）は`quoteUrl`と`_misskey_quote`フィールドを含むCreate Activityとして配信。純粋なリノート（テキストなし）はAnnounce Activityとして配信。Mastodon/Pleroma/Akkomaとの引用投稿の相互運用性を実現

### インフラ拡張 (2025-12-11)
- **Redis分散レート制限** - APIレート制限をRedis INCRコマンドを使った分散カウンターで実装。複数インスタンスのロードバランシング環境でも一貫したレート制限が可能。`RateLimiterState::with_redis()`で有効化、フェイルオープン動作でRedis障害時もサービス継続

### タイムラインUX (2025-12-11)
- **Bot非表示オプション** - `user_profile.hide_bots`設定でタイムラインからBotアカウントの投稿を非表示にできる。ホーム/ローカル/グローバル/バブル各タイムラインでBot投稿をフィルタリング。`/api/users/update`エンドポイントで`hideBots`パラメータを設定可能

### 検索機能強化 (2025-12-11)
- **ドライブ検索** - `/api/drive/files/find`エンドポイントでファイル名・説明（comment）による検索が可能。MIMEタイプ（`type`パラメータ）やフォルダーID（`folderId`）でのフィルタリング対応。ページネーション（`limit`, `untilId`）もサポート
- **インスタンス指定アンテナ** - アンテナのソースタイプとして`instances`を選択可能。指定したリモートインスタンスからのノートのみをマッチングできる。`AntennaSource::Instances`列挙値と`instances`フィールド（JSON配列）で実装

### UI/UX設定 (2025-12-11)
- **デフォルトリアクション設定** - ユーザーごとにデフォルトリアクション絵文字を設定可能。`user_profile.default_reaction`フィールドに絵文字（Unicode絵文字またはカスタム絵文字ショートコード）を保存。`/api/users/update`エンドポイントの`defaultReaction`パラメータで設定可能

### ストリーミング拡張 (2025-12-11)
- **チャンネルタイムラインのストリーミング** - WebSocket経由でチャンネルタイムラインのリアルタイム更新を購読可能。`StreamChannel::Channel { channel_id }`で特定のチャンネルに接続し、チャンネルに投稿されたノートをリアルタイムで受信。Redis Pub/Subの`misskey:channel:{channel_id}`チャンネルを使用してイベントを配信。

### データ管理 (2025-12-11)
- **ノートエクスポート（JSON/CSV）** - `/api/i/account/export/notes`エンドポイントでユーザーのノートをエクスポート可能。JSON形式（デフォルト）とCSV形式をサポート。各ノートはID、テキスト、CW、公開範囲、返信先、リノート先、ファイルID、タグ、URI、URL、作成日時、更新日時を含む。`limit`パラメータで最大件数指定（デフォルト10000）、`format`パラメータで形式選択（`json`または`csv`）。
- **ブロック/ミュートエクスポート** - `/api/i/account/export/blocking`および`/api/i/account/export/muting`エンドポイントでブロック・ミュートしているユーザーをエクスポート可能。`ExportedFollow`形式（acct、uri）で最大10000件まで出力。
- **Mastodon形式インポート** - `/api/i/account/import/blocking`および`/api/i/account/import/muting`エンドポイントでMastodon形式CSVからブロック・ミュートリストをインポート可能。ヘッダー行付きCSV（`Account address`列）またはシンプルな1行1acct形式に対応。インポート結果としてtotal/imported/skipped/failedカウントとエラー詳細を返却。

---

## 次のアクション推奨

1. **フェデレーション**: チャンネルのフェデレーション（Group actor）
2. **検索**: Meilisearch連携
3. **UI/UX**: ワンボタンいいね（Like/Reaction分離）

---

*関連ドキュメント:*
- [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) - 詳細な実装状況
- [FORK_FEATURES.md](FORK_FEATURES.md) - フォーク機能比較
- [RUST_TECH_STACK.md](RUST_TECH_STACK.md) - 技術スタック
