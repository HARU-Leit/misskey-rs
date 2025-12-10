# Misskey-rs 実装状況レポート

現在の実装状態を詳細に分析したレポートです。

*Last Updated: 2025-12-11*

---

## 全体サマリー

| カテゴリ | 完成度 | 状態 |
|---------|-------|------|
| データベーススキーマ | 100% | 32エンティティ + 29マイグレーション完了 |
| APIエンドポイント（Misskey） | 98% | 24モジュール完全動作 |
| APIエンドポイント（Mastodon） | 60% | 基本サポートのみ |
| コアビジネスロジック | 95% | サービス実装済み、ActivityPub配信連携済み |
| フェデレーション構造 | 85% | オブジェクト/アクティビティ定義済み |
| ActivityPub Inbox | 85% | 受信・パース可能、リモートアクター取得実装済み |
| ActivityPub 配信 | 80% | アクティビティ構築・キュー連携済み |
| リアルタイムストリーミング | 95% | インフラ完成、イベント発火実装済み |
| バックグラウンドジョブ | 50% | システム存在、機能未連携 |
| 認証 | 100% | トークン認証 + WebAuthn/パスキー + OAuth 2.0 |
| ユーザー管理 | 95% | 完全CRUD + リレーション |
| コンテンツ管理 | 95% | ノート、リアクション、お気に入り、ページ動作 |
| ソーシャル機能 | 90% | フォロー、ブロック、ミュート、Webhook完了 |

**総合完成度: 約90%**（基本SNS機能）
**フェデレーション完成度: 約60-70%**（ActivityPub完全対応まで）

---

## 1. APIエンドポイント

### Misskey API（実装済み）

| モジュール | 状態 | エンドポイント |
|-----------|------|---------------|
| **auth.rs** | ✅ 完了 | signup, signin, signout, regenerate-token |
| **notes.rs** | ✅ 完了 | create, timeline, local/global-timeline, show, delete, search, thread, conversation, replies, renotes |
| **users.rs** | ✅ 完了 | me, show, update, followers, following, notes |
| **following.rs** | ✅ 完了 | follow, unfollow, accept, reject, pending requests |
| **reactions.rs** | ✅ 完了 | create, delete, fetch reactions |
| **notifications.rs** | ✅ 完了 | get, read, delete |
| **blocking.rs** | ✅ 完了 | block, unblock, list |
| **muting.rs** | ✅ 完了 | mute, unmute, list |
| **drive.rs** | ✅ 完了 | upload, create_folder, delete, move |
| **poll.rs** | ✅ 完了 | create, vote, results |
| **search.rs** | ✅ 完了 | notes, users, hashtags |
| **hashtags.rs** | ✅ 完了 | trending, show |
| **announcements.rs** | ✅ 完了 | list, mark_as_read |
| **admin.rs** | ✅ 完了 | user management, stats |
| **emojis.rs** | ✅ 完了 | custom emoji CRUD |
| **favorites.rs** | ✅ 完了 | create, delete, list |
| **lists.rs** | ✅ 完了 | user lists, members |
| **messaging.rs** | ✅ 完了 | send, conversations |
| **meta.rs** | ✅ 完了 | instance metadata |
| **two_factor.rs** | ✅ 完了 | 2FA setup, verify, disable |
| **security_keys.rs** | ✅ 完了 | WebAuthn/Passkey registration, authentication |
| **oauth.rs** | ✅ 完了 | OAuth 2.0 apps, authorize, token, PKCE |
| **webhooks.rs** | ✅ 完了 | create, list, update, delete, test, regenerate-secret |
| **pages.rs** | ✅ 完了 | create, update, delete, show, like, unlike, featured |

### Mastodon互換API（部分実装）

| モジュール | 状態 | 備考 |
|-----------|------|------|
| **accounts.rs** | ⚠️ 部分 | lookup, profile取得のみ |
| **statuses.rs** | ⚠️ 部分 | 作成/削除のみ、メディア未対応 |
| **timelines.rs** | ⚠️ 部分 | home/public、base_url設定TODO |

---

## 2. データベースエンティティ

**合計: 32エンティティ、29マイグレーション**

### ユーザー関連
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| User | user | ✅ 完了 |
| UserProfile | user_profile | ✅ 完了 |
| UserKeypair | user_keypair | ✅ 完了 |
| UserList | user_list | ✅ 完了 |
| UserListMember | user_list_member | ✅ 完了 |
| UserSuspension | user_suspension | ✅ 完了 |

### コンテンツ関連
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| Note | note | ✅ 完了 |
| Reaction | reaction | ✅ 完了 |
| NoteFavorite | note_favorite | ✅ 完了 |
| Poll | poll | ✅ 完了 |
| PollVote | poll_vote | ✅ 完了 |
| Hashtag | hashtag | ✅ 完了 |

### ソーシャル関連
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| Following | following | ✅ 完了 |
| FollowRequest | follow_request | ✅ 完了 |
| Blocking | blocking | ✅ 完了 |
| Muting | muting | ✅ 完了 |
| Notification | notification | ✅ 完了 |

### ドライブ関連
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| DriveFile | drive_file | ✅ 完了 |
| DriveFolder | drive_folder | ✅ 完了 |

### その他
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| Emoji | emoji | ✅ 完了 |
| Announcement | announcement | ✅ 完了 |
| AnnouncementRead | announcement_read | ✅ 完了 |
| MessagingMessage | messaging_message | ✅ 完了 |
| AbuseReport | abuse_report | ✅ 完了 |

### 認証・セキュリティ関連 (2025-12-11 新規)
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| SecurityKey | security_key | ✅ 完了 |
| OAuthApp | oauth_app | ✅ 完了 |
| OAuthToken | oauth_token | ✅ 完了 |

### Webhook・ページ関連 (2025-12-11 新規)
| エンティティ | テーブル | 状態 |
|-------------|---------|------|
| Webhook | webhook | ✅ 完了 |
| Page | page | ✅ 完了 |
| PageLike | page_like | ✅ 完了 |

---

## 3. コアサービス

**合計: 23サービスモジュール、約6,500行**

| サービス | 完成度 | 未実装/TODO |
|---------|-------|------------|
| **note.rs** | 100% | ✅ ActivityPub配信+イベント発火済み |
| **user.rs** | 95% | - |
| **following.rs** | 100% | ✅ ActivityPub+イベント発火済み |
| **reaction.rs** | 100% | ✅ ActivityPub+イベント発火済み |
| **notification.rs** | 100% | ✅ イベント発火済み |
| **drive.rs** | 80% | ファイル実体削除、循環参照チェック (TODO) |
| **user_list.rs** | 90% | - |
| **poll.rs** | 85% | - |
| **messaging.rs** | 90% | ✅ イベント発火済み、ブロックチェック (TODO) |
| **blocking.rs** | 95% | - |
| **muting.rs** | 95% | - |
| **hashtag.rs** | 90% | - |
| **emoji.rs** | 90% | - |
| **announcement.rs** | 90% | - |
| **moderation.rs** | 85% | - |
| **note_favorite.rs** | 90% | - |
| **delivery.rs** | ✅ | ActivityPub配信トレイト定義 |
| **event_publisher.rs** | ✅ 新規 | リアルタイムイベント発火トレイト定義 |
| **two_factor.rs** | 100% | ✅ 完了 - TOTP 2FA セットアップ・検証・無効化 |
| **webauthn.rs** | 100% | ✅ 完了 - WebAuthn/Passkey 登録・認証 |
| **oauth.rs** | 100% | ✅ 完了 - OAuth 2.0 Authorization Code Flow + PKCE |
| **webhook.rs** | 100% | ✅ 完了 - Webhook管理・配信・HMAC署名 |
| **page.rs** | 100% | ✅ 完了 - ユーザーページ CRUD・いいね機能 |

### 重要な未実装箇所

```
crates/core/src/services/messaging.rs:81  // TODO: Check if blocked
crates/core/src/services/drive.rs:218     // TODO: Actually delete file from storage
```

### 新規実装済み: ActivityPub配信サービス

**`crates/core/src/services/delivery.rs`**:
- `ActivityDelivery` トレイト - 配信インターフェース定義
- `NoOpDelivery` - テスト/無効化用のnoop実装
- `DeliveryService` - Arc<dyn ActivityDelivery>のエイリアス

**`crates/queue/src/delivery_impl.rs`**:
- `RedisDeliveryService` - Redisベースのキュー実装
- apalis ジョブキューへの配信ジョブ追加

### 新規実装済み: リアルタイムイベント発火システム (2025-12-11)

**`crates/core/src/services/event_publisher.rs`**:
- `EventPublisher` トレイト - イベント発火インターフェース定義
- `NoOpEventPublisher` - テスト/無効化用のnoop実装
- `EventPublisherService` - Arc<dyn EventPublisher>のエイリアス
- サポートするイベント:
  - `publish_note_created` - ノート作成
  - `publish_note_deleted` - ノート削除
  - `publish_note_updated` - ノート更新
  - `publish_followed` - フォロー
  - `publish_unfollowed` - フォロー解除
  - `publish_reaction_added` - リアクション追加
  - `publish_reaction_removed` - リアクション削除
  - `publish_notification` - 通知
  - `publish_direct_message` - ダイレクトメッセージ

**`crates/queue/src/pubsub.rs`** (拡張):
- `RedisPubSub` に `EventPublisher` トレイトを実装
- 新チャンネル `misskey:messaging` 追加
- 新イベントタイプ `DirectMessage` 追加

---

## 4. フェデレーション

### ActivityPubオブジェクト/アクティビティ

| コンポーネント | 状態 | ファイル |
|--------------|------|---------|
| HTTP署名 | ✅ 完了 | signature.rs |
| WebFinger | ✅ 完了 | handler/webfinger.rs |
| NodeInfo | ⚠️ 部分 | handler/nodeinfo.rs (統計TODO) |
| Personアクター | ✅ 完了 | actors/person.rs |
| Noteオブジェクト | ✅ 完了 | objects/note.rs |
| Create | ✅ 完了 | activities/create.rs |
| Delete | ✅ 完了 | activities/delete.rs |
| Follow | ✅ 完了 | activities/follow.rs |
| Accept | ✅ 完了 | activities/accept.rs |
| Reject | ✅ 完了 | activities/reject.rs |
| Like | ✅ 完了 | activities/like.rs |
| Announce | ✅ 完了 | activities/announce.rs |
| Update | ✅ 完了 | activities/update.rs |
| Undo | ✅ 完了 | activities/undo.rs |

### アクティビティプロセッサー

| プロセッサー | 状態 | 未実装 |
|-------------|------|--------|
| CreateProcessor | ✅ 完了 | - (2025-12-10 リモートアクター取得実装) |
| DeleteProcessor | ✅ 完了 | - |
| FollowProcessor | ✅ 完了 | - (2025-12-10 リモートアクター取得実装) |
| LikeProcessor | ✅ 完了 | - (2025-12-10 リモートアクター取得実装) |
| AcceptProcessor | ✅ 完了 | - |
| RejectProcessor | ✅ 完了 | - |
| UndoProcessor | ✅ 完了 | - |
| AnnounceProcessor | ⚠️ 部分 | リモートアクター取得 |
| UpdateProcessor | ✅ 完了 | - |

### ActorFetcher (2025-12-10 新規実装)

**`crates/federation/src/processor/actor_fetcher.rs`**:
- 共通のリモートアクター取得ユーティリティ
- `find_or_fetch()` - URI検索またはリモートから取得
- ActivityPub JSONからユーザーエンティティを作成
- 既存ユーザーのURI更新もサポート

### 配信システム

| 機能 | 状態 |
|------|------|
| アクティビティ構築 | ✅ 完了 |
| アドレッシング | ✅ 完了 |
| ジョブキュー連携 | ✅ 完了 (2025-12-10実装) |
| HTTPクライアント | ✅ 完了 |

**実装済みアクティビティ配信**:
- Create (ノート作成) → フォロワーinboxへ配信
- Delete (ノート削除) → フォロワーinboxへ配信
- Follow (フォロー) → 対象ユーザーinboxへ配信
- Accept (フォロー承認) → フォロワーinboxへ配信
- Reject (フォロー拒否) → フォロワーinboxへ配信
- Undo (フォロー解除/リアクション削除) → 対象inboxへ配信
- Like (リアクション) → ノート作者inboxへ配信

---

## 5. ジョブキューシステム

### インフラ（実装済み）

| コンポーネント | 状態 |
|--------------|------|
| ジョブ定義 (DeliverJob, InboxJob) | ✅ 完了 |
| ワーカーフレームワーク | ✅ 完了 |
| Redis PubSub | ✅ 完了 |
| レート制限 | ✅ 完了 |
| リトライロジック | ✅ 完了 |
| スケジューラー | ✅ 完了 |
| SharedInbox最適化 | ✅ 完了 |

### 2025-12-10 実装完了

**コアサービスからジョブキューへの連携が実装されました**

- ✅ ノート作成時にCreateアクティビティ配信ジョブがキューされる
- ✅ ノート削除時にDeleteアクティビティ配信ジョブがキューされる
- ✅ フォロー時にFollowアクティビティが送信される
- ✅ フォロー解除時にUndo Followアクティビティが送信される
- ✅ フォロー承認時にAcceptアクティビティが送信される
- ✅ フォロー拒否時にRejectアクティビティが送信される
- ✅ リアクション時にLikeアクティビティが送信される
- ✅ リアクション削除時にUndo Likeアクティビティが送信される

---

## 6. ストリーミング

### WebSocket (streaming.rs)

| 機能 | 状態 |
|------|------|
| チャンネルタイプ | ✅ HomeTimeline, LocalTimeline, GlobalTimeline, Main, User |
| イベントタイプ | ✅ Note, NoteDeleted, Notification, Followed, Unfollowed, Mention |
| クライアントメッセージ | ✅ Connect, Disconnect, SubNote, UnsubNote, ReadNotification |
| 認証 | ✅ トークンベース |
| ブロードキャスト | ✅ グローバル/ローカルチャンネル |

### Server-Sent Events (sse.rs)

| 機能 | 状態 |
|------|------|
| イベントタイプ | ✅ Note, NoteDeleted, Notification, Followed, Unfollowed, Reaction |
| ルート | ✅ /global, /local, /user |
| Keep-Alive | ✅ 30秒ping |
| 認証 | ✅ AuthUser抽出 |
| チャンネルクリーンアップ | ✅ 非アクティブチャンネル削除 |

### 統合状況 ✅ 完了 (2025-12-11)

**サービスからイベントがトリガーされるようになりました**

各サービスに `EventPublisherService` を注入することで、Redis Pub/Sub経由でWebSocket/SSEにイベントが配信されます。

---

## 7. クリティカルな問題（修正必須）

### ~~問題1: ActivityPub配信未連携~~ ✅ 解決済み (2025-12-10)

**解決状況**: ActivityPub配信システムが実装されました

**実装内容**:
- `delivery.rs` - ActivityDeliveryトレイト定義
- `delivery_impl.rs` - RedisDeliveryService実装
- NoteService - Create/Delete配信
- FollowingService - Follow/Accept/Reject/Undo配信
- ReactionService - Like/Undo配信

### ~~問題2: リモートアクター取得未実装~~ ✅ 解決済み (2025-12-10)

**解決状況**: リモートアクター取得が実装されました

**実装内容**:
- `ActorFetcher` - 共通のリモートアクター取得ユーティリティ
- `ApClient.fetch_actor()` - HTTPでリモートアクターを取得
- ActivityPub JSON → ユーザーエンティティ変換
- 既存ユーザーのメタデータ更新サポート

**対応済みプロセッサー**:
- `CreateProcessor` - 不明なユーザーからのノート ✅
- `FollowProcessor` - リモートからのフォローリクエスト ✅
- `LikeProcessor` - リモートからのリアクション ✅

### ~~問題3: リアルタイムイベント未発火~~ ✅ 解決済み (2025-12-11)

**解決状況**: リアルタイムイベント発火システムが実装されました

**実装内容**:
- `EventPublisher` トレイト - コアサービス用の抽象インターフェース
- `RedisPubSub` に `EventPublisher` を実装
- 各サービスに `set_event_publisher()` メソッドを追加

**対応済みサービス**:
- `NoteService` - ノート作成/削除/更新時にイベント発火 ✅
- `FollowingService` - フォロー/アンフォロー時にイベント発火 ✅
- `ReactionService` - リアクション追加/削除時にイベント発火 ✅
- `NotificationService` - 通知作成時にイベント発火 ✅
- `MessagingService` - メッセージ送信時にイベント発火 ✅

---

## 8. 中優先度の問題

### メッセージング検証不足
- ブロックチェック未実装
- プライバシー/権限チェック未実装

### ドライブシステム
- ファイル実体削除未実装
- フォルダ循環参照チェック未実装

### Mastodon API
- base_url設定がハードコード
- メディア添付処理不完全

### NodeInfo
- 実際のDB統計ではなくプレースホルダー値

---

## 9. 次のステップ（推奨）

### 即時対応（フェデレーション有効化）

1. ~~**ジョブキュー連携の実装**~~ ✅ 完了 (2025-12-10)
   - ~~コアサービスにqueue引数を追加~~
   - ~~ノート/フォロー/リアクション時にジョブをキュー~~

2. ~~**リモートアクター取得の実装**~~ ✅ 完了 (2025-12-10)
   - ~~WebFinger + Actor fetchの実装~~
   - ~~アクターキャッシュ（user テーブル使用）~~
   - `ActorFetcher` ユーティリティ実装済み

3. ~~**リアルタイムイベント発火**~~ ✅ 完了 (2025-12-11)
   - ~~サービスにEventPublisher引数を追加~~
   - ~~状態変更時にイベント発行~~
   - `EventPublisher` トレイト + `RedisPubSub` 実装済み

4. ~~**サーバー起動時のワーカー初期化**~~ ✅ 完了 (2025-12-10)
   - ~~main.rsにRedisDeliveryServiceの初期化を追加~~
   - ~~apalisワーカーの起動~~

### 短期対応

5. メッセージングのブロックチェック
6. ドライブのファイル実体削除
7. NodeInfoの実統計取得

### 中期対応

8. Mastodon APIの完全対応
9. メディア処理（サムネイル生成）
10. 検索の全文検索エンジン連携

---

## ファイル統計

| ディレクトリ | ファイル数 | 総行数 |
|-------------|-----------|-------|
| crates/api/src/endpoints/ | 37 | ~6,500 |
| crates/db/src/entities/ | 32 | ~3,000 |
| crates/db/src/migrations/ | 29 | ~3,500 |
| crates/core/src/services/ | 23 | ~6,500 |
| crates/federation/src/ | 20+ | ~4,000 |
| crates/queue/src/ | 10 | ~1,500 |
| crates/api/src/ (streaming) | 2 | ~650 |
| **合計** | **150+** | **~26,000** |

---

## 10. 2025-12-11 新規実装機能

### WebAuthn/パスキー認証

**機能概要**:
- FIDO2/WebAuthn 標準に準拠したパスワードレス認証
- セキュリティキー（YubiKey等）とプラットフォーム認証（Face ID, Touch ID, Windows Hello）に対応
- 複数のセキュリティキーを登録可能

**実装ファイル**:
- `crates/db/src/entities/security_key.rs` - セキュリティキーエンティティ
- `crates/db/src/repositories/security_key.rs` - リポジトリ
- `crates/core/src/services/webauthn.rs` - WebAuthnサービス（webauthn-rs使用）
- `crates/api/src/endpoints/security_keys.rs` - APIエンドポイント

**エンドポイント**:
- `POST /api/i/security-keys/register/begin` - 登録開始
- `POST /api/i/security-keys/register/complete` - 登録完了
- `POST /api/i/security-keys/authenticate/begin` - 認証開始
- `POST /api/i/security-keys/authenticate/complete` - 認証完了
- `POST /api/i/security-keys/list` - キー一覧
- `POST /api/i/security-keys/remove` - キー削除

### OAuth 2.0 認証

**機能概要**:
- OAuth 2.0 Authorization Code Flow
- PKCE (Proof Key for Code Exchange) サポート
- スコープベースのアクセス制御
- リフレッシュトークンサポート

**実装ファイル**:
- `crates/db/src/entities/oauth_app.rs` - OAuthアプリエンティティ
- `crates/db/src/entities/oauth_token.rs` - OAuthトークンエンティティ
- `crates/db/src/repositories/oauth.rs` - リポジトリ
- `crates/core/src/services/oauth.rs` - OAuthサービス
- `crates/api/src/endpoints/oauth.rs` - APIエンドポイント

**エンドポイント**:
- `POST /api/oauth/apps/create` - アプリ作成
- `POST /api/oauth/apps/show` - アプリ情報取得
- `POST /api/oauth/apps/update` - アプリ更新
- `POST /api/oauth/apps/delete` - アプリ削除
- `POST /api/oauth/apps/mine` - 自分のアプリ一覧
- `POST /api/oauth/authorize` - 認可
- `POST /api/oauth/token` - トークン交換
- `POST /api/oauth/revoke` - トークン取消

**スコープ**:
- `read`, `write` - 全般的な読み取り/書き込み
- `read:account`, `write:account` - アカウント情報
- `read:notes`, `write:notes` - ノート
- `read:notifications`, `write:notifications` - 通知
- `read:messaging`, `write:messaging` - メッセージング
- `read:drive`, `write:drive` - ドライブ
- `read:favorites`, `write:favorites` - お気に入り
- `read:following`, `write:following` - フォロー
- `read:mutes`, `write:mutes` - ミュート
- `read:blocks`, `write:blocks` - ブロック

### Webhookシステム

**機能概要**:
- ユーザーごとにWebhookを設定可能
- イベント発生時にHTTP POSTで通知
- HMAC-SHA256署名によるペイロード検証
- 失敗時の自動リトライと自動無効化

**実装ファイル**:
- `crates/db/src/entities/webhook.rs` - Webhookエンティティ
- `crates/db/src/repositories/webhook.rs` - リポジトリ
- `crates/core/src/services/webhook.rs` - Webhookサービス
- `crates/api/src/endpoints/webhooks.rs` - APIエンドポイント

**エンドポイント**:
- `POST /api/i/webhooks/create` - Webhook作成
- `POST /api/i/webhooks/list` - 一覧取得
- `POST /api/i/webhooks/show` - 詳細取得
- `POST /api/i/webhooks/update` - 更新
- `POST /api/i/webhooks/delete` - 削除
- `POST /api/i/webhooks/regenerate-secret` - シークレット再生成
- `POST /api/i/webhooks/test` - テスト送信

**対応イベント**:
- `note` - ノート作成
- `reply` - リプライ
- `renote` - リノート
- `mention` - メンション
- `follow` - フォローされた
- `followed` - フォローした
- `unfollow` - フォロー解除
- `reaction` - リアクション

### ページ（Page）機能

**機能概要**:
- Misskeyスタイルのカスタマイズ可能なユーザーページ
- JSONベースのコンテンツブロック構造
- スクリプトによるインタラクティブページ対応
- いいね機能とビューカウント

**実装ファイル**:
- `crates/db/src/entities/page.rs` - ページエンティティ
- `crates/db/src/entities/page_like.rs` - ページいいねエンティティ
- `crates/db/src/repositories/page.rs` - リポジトリ
- `crates/core/src/services/page.rs` - ページサービス
- `crates/api/src/endpoints/pages.rs` - APIエンドポイント

**エンドポイント**:
- `POST /api/pages/create` - ページ作成
- `POST /api/pages/mine` - 自分のページ一覧
- `POST /api/pages/show` - ページ詳細
- `POST /api/pages/show-by-name` - ユーザー名+名前でページ取得
- `POST /api/pages/update` - ページ更新
- `POST /api/pages/delete` - ページ削除
- `POST /api/pages/like` - いいね
- `POST /api/pages/unlike` - いいね解除
- `POST /api/pages/featured` - 人気ページ一覧
