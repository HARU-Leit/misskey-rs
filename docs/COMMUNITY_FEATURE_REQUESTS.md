# 本家Misskeyコミュニティ機能要望調査

本家Misskey (misskey-dev/misskey) のIssue、Discussions、コミュニティフィードバックから収集した機能要望の調査レポート。misskey-rsの開発優先順位を決定する参考資料として活用する。

*調査日: 2025-12-11*

---

## サマリー

| カテゴリ | 要望数 | 優先度 |
|---------|--------|--------|
| パフォーマンス改善 | 8 | 🔴 高 |
| ActivityPub/フェデレーション | 7 | 🔴 高 |
| UI/UX改善 | 9 | 🟡 中 |
| API改善 | 5 | 🟡 中 |
| 新機能追加 | 12 | 🟢 低〜中 |

---

## 1. パフォーマンス改善要望 🔴

### 1.1 WebSocketの高負荷問題
**ソース**: [Discussion #11069](https://github.com/misskey-dev/misskey/discussions/11069)

**問題**: misskey.ioでのトラフィック急増時にWebSocketのパフォーマンスコストが高い

**misskey-rsでの対応案**:
- [ ] WebSocket接続のコネクションプーリング最適化
- [ ] メッセージバッチング（複数イベントをまとめて送信）
- [ ] WebSocket接続の圧縮対応
- [ ] 接続数に応じた自動スケーリング

### 1.2 データベースクエリの軽量化
**ソース**: [Discussion #11069](https://github.com/misskey-dev/misskey/discussions/11069)

**問題**: DBへのクエリ負荷が高い

**misskey-rsでの対応状況**: ✅ 一部実装済み
- [x] カウンター直接更新（ENHANCEMENT_OPPORTUNITIES.md参照）
- [x] 再帰CTE（祖先取得）
- [ ] 読み取りレプリカ対応
- [ ] クエリキャッシュの強化

### 1.3 リアクション処理の負荷
**ソース**: [gihyo.jp - Misskeyのパフォーマンス改善の取り組み](https://gihyo.jp/article/2023/07/misskey-05)

**問題**: フォロワー数の多いユーザーの投稿にリアクションが集中するとPostgreSQLの更新負荷が高い

**misskey-rsでの対応案**:
- [ ] **Misskey RBT（Reactions Boost Technology）** 相当の実装
- [ ] リアクションカウントの遅延集計（バッチ処理）
- [ ] Redisでのカウントバッファリング

### 1.4 クライアント起動時の重さ
**ソース**: [Issue #8525](https://github.com/misskey-dev/misskey/issues/8525)

**問題**: クライアント起動時にとても重くなる（特にFirefoxで顕著）

**misskey-rsでの対応案**（フロントエンド側の改善）:
- [ ] APIレスポンスの遅延読み込み対応
- [ ] 初期データの最小化オプション
- [ ] SSR（Server-Side Rendering）対応

### 1.5 カスタム絵文字検索の遅延
**ソース**: [gihyo.jp](https://gihyo.jp/article/2023/07/misskey-05)

**問題**: 大量のカスタム絵文字登録時、Array.filterの線形検索でパフォーマンスが劣化

**misskey-rsでの対応案**:
- [x] Aho-Corasickアルゴリズムによる高速マッチング（アンテナで実装済み）
- [ ] 絵文字検索APIの最適化
- [ ] 絵文字インデックスのキャッシュ

### 1.6 ActivityPub署名の計算コスト
**ソース**: [gihyo.jp](https://gihyo.jp/article/2023/07/misskey-05)

**問題**: RSA 4096bitの署名計算が重い（Mastodonは2048bit）

**misskey-rsでの対応案**:
- [ ] Ed25519/ECDSA への移行オプション
- [ ] 署名計算の並列化
- [ ] 署名キャッシュ

### 1.7 タイムライン配信の効率
**ソース**: [gihyo.jp](https://gihyo.jp/article/2023/11/misskey-08)

**問題**: Pull型タイムラインの効率問題（v2023.0でPush型FTTに移行済み）

**misskey-rsでの対応案**:
- [ ] **Fanout Timeline Technology (FTT)** 相当の実装
- [ ] Redis Sorted Setによるタイムライン管理
- [ ] サンプリングによる計算頻度削減

### 1.8 ぼかし効果（Blur）のパフォーマンス
**ソース**: [misskey.io Twitter](https://x.com/misskey_io/status/1677933966836518921)

**問題**: UIのぼかし効果がパフォーマンスに影響

**misskey-rsでの対応案**（API側）:
- [ ] ユーザー設定の同期API
- [ ] デバイス別設定の保存

---

## 2. ActivityPub/フェデレーション改善要望 🔴

### 2.1 チャンネルのフェデレーション
**ソース**: [Discussion #14049](https://github.com/misskey-dev/misskey/discussions/14049), [Issue #8475](https://github.com/misskey-dev/misskey/issues/8475)

**要望**: チャンネル機能をActivityPubで連合できるようにする

**misskey-rsでの対応案**:
- [ ] Group actorとしてのチャンネル実装
- [ ] チャンネルフォローのActivityPub対応
- [ ] チャンネル投稿の配信

### 2.2 リモートリアクションの取得
**ソース**: [Issue #10167](https://github.com/misskey-dev/misskey/issues/10167)

**要望**: リモートのノートについたリアクションを全て読み込めるようにする

**期待効果**: 連合Activityの8割削減可能性

**misskey-rsでの対応案**:
- [ ] ActivityPub Likesコレクション実装
- [ ] Fedibirdスタイルのemoji_reactions対応
- [ ] リアクション同期のバッチ処理

### 2.3 ノート編集の連合
**ソース**: [Issue #8364](https://github.com/misskey-dev/misskey/issues/8364)

**要望**: Mastodonのノート編集（Update activity）を受信・反映する

**misskey-rsでの対応状況**: 一部実装済み
- [x] ノート編集API実装
- [ ] ActivityPub Update activity受信対応
- [ ] 編集履歴の連合

### 2.4 引用リノートの他プラットフォーム対応
**ソース**: [Issue #8722](https://github.com/misskey-dev/misskey/issues/8722)

**要望**: 引用リノートを他のFediverseソフトウェアと互換性を持たせる

**misskey-rsでの対応案**:
- [ ] FEP-e232（Quote Posts）対応
- [ ] Mastodon互換の引用表示

### 2.5 MFMの連合問題
**ソース**: [Issue #4810](https://github.com/misskey-dev/misskey/issues/4810)

**問題**: MFM形式のテキストがMisskey以外で正しく表示されない

**misskey-rsでの対応案**:
- [ ] プレーンテキストのフォールバック提供
- [ ] `content` と `source` の使い分け
- [ ] MFMをHTMLに変換して配信するオプション

### 2.6 古いリモートノートの削除
**ソース**: [Issue #9972](https://github.com/misskey-dev/misskey/issues/9972), [Discussion - 未使用リモートノート削除](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: ディスク容量削減のため、古いリモートノートを削除する機能

**misskey-rsでの対応案**:
- [ ] リモートノート自動削除ジョブ
- [ ] 保持期間の設定
- [ ] 参照されているノートの保護

### 2.7 リモートフォローUIの改善
**ソース**: [Zenn記事](https://zenn.dev/okuoku/scraps/07bba084b6baed)

**問題**: Mastodonのような直接フォローUIがなく、使いにくい

**misskey-rsでの対応案**:
- [ ] WebFinger検索の改善
- [ ] リモートユーザー検索UI向けAPI

---

## 3. UI/UX改善要望 🟡

### 3.1 絵文字ピッカーのクリック改善
**ソース**: [Issue #16435](https://github.com/misskey-dev/misskey/issues/16435)

**要望**: 絵文字ボタン押下時に検索フィールドを自動選択

**misskey-rsでの対応**: フロントエンド側の改善（API変更不要）

### 3.2 代名詞（Pronouns）機能
**ソース**: [Issue #8726](https://github.com/misskey-dev/misskey/issues/8726)

**要望**: ユーザープロフィールに代名詞を設定し、返信時にリマインダーを表示

**misskey-rsでの対応状況**: ✅ 実装済み
- [x] ユーザープロフィールにpronouns フィールド追加
- [x] プロフィール更新API対応（`/users/update` でpronounsを設定可能）
- [ ] vCard形式での連合対応
- [ ] UIへのリマインダーAPI提供

### 3.3 アバターデコレーションのカテゴリ分け
**ソース**: [Issue #16854](https://github.com/misskey-dev/misskey/issues/16854)

**要望**: アバターデコレーションにカテゴリを導入（絵文字と同様）

**misskey-rsでの対応案**:
- [ ] デコレーションカテゴリエンティティ追加
- [ ] カテゴリ別取得API

### 3.4 投稿全体をクリック可能に
**ソース**: [Issue #8804](https://github.com/misskey-dev/misskey/issues/8804)

**要望**: ノート全体をクリックして詳細を開けるようにする

**misskey-rsでの対応**: フロントエンド側の改善

### 3.5 通知バッジの位置
**ソース**: [Issue #16042](https://github.com/misskey-dev/misskey/issues/16042)

**要望**: 通知バッジの位置を右上に戻してほしい

**misskey-rsでの対応**: フロントエンド側の改善

### 3.6 デフォルトぼかし効果設定
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: ぼかし効果のデフォルト設定（6票獲得）

**misskey-rsでの対応状況**: ✅ 実装済み
- [x] インスタンスデフォルト設定（`meta_settings.default_blur_nsfw`）
- [x] 管理者設定テーブル（`meta_settings`）でUIデフォルト値を管理
- [ ] ユーザー設定のデフォルト値API（metaエンドポイント経由で提供予定）

### 3.7 UI破壊的変更の移行期間
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: UI変更時に移行期間を設ける（5票獲得）

**misskey-rsでの対応案**:
- [ ] フィーチャーフラグシステム
- [ ] 段階的UI更新機能

### 3.8 Deck UIの改善
**ソース**: [Misskey Wiki](https://wiki.misskey.io/ja/function/ui)

**要望**: 情報密度の高いDeck UIのカスタマイズ性向上

**misskey-rsでの対応案**:
- [ ] カラムレイアウト保存API
- [ ] カラム設定の同期

### 3.9 「いいね」とリアクションの分離
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: Like機能とリアクション機能をより実用的に分離

**misskey-rsでの対応案**:
- [ ] Likeカウントの個別管理
- [ ] ActivityPub Like vs EmojiReactの明確な分離

---

## 4. API改善要望 🟡

### 4.1 ドメイン分離の許可
**ソース**: [Issue #6724](https://github.com/misskey-dev/misskey/issues/6724)

**要望**: Webアクセスドメインとは異なるドメインでの使用を許可

**misskey-rsでの対応案**:
- [ ] マルチドメイン対応
- [ ] ドメイン別設定

### 4.2 API Noteのreactionsオブジェクト改善
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: リアクションオブジェクトの構造改善

**misskey-rsでの対応案**:
- [ ] GraphQL APIでの柔軟なレスポンス形式
- [ ] リアクション詳細情報の選択的取得

### 4.3 Unixタイムスタンプ統合
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: APIでUnixタイムスタンプを使用（2票獲得）

**misskey-rsでの対応案**:
- [ ] ISO8601とUnixタイムスタンプの両方をサポート
- [ ] リクエストヘッダーでの形式指定

### 4.4 分割アップロード仕様
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: 大容量ファイルの分割アップロード対応

**misskey-rsでの対応案**:
- [ ] チャンクアップロードAPI
- [ ] アップロード再開機能
- [ ] S3マルチパートアップロード連携

### 4.5 OpenAPI仕様の改善
**ソース**: [Release Notes](https://misskey-hub.net/en/docs/releases/)

**最近の改善**: api.jsonのOpenAPI Specification 3.1.0対応

**misskey-rsでの対応状況**: 設計済み
- [ ] OpenAPI 3.1自動生成
- [ ] SDK自動生成

---

## 5. 新機能要望 🟢

### 5.1 Page機能の文字数上限緩和
**ソース**: [Issue #10574](https://github.com/misskey-dev/misskey/issues/10574)

**要望**: 小説投稿サーバー（ノベルスキー）のユーザーから、Page機能の文字数制限緩和

**misskey-rsでの対応状況**: ✅ 実装済み
- [x] 管理者設定可能な文字数上限（`meta_settings.max_page_content_length`、デフォルト65536文字）
- [x] ノート文字数上限も設定可能（`meta_settings.max_note_text_length`、デフォルト3000文字）
- [x] ユーザーあたりの最大ページ数設定（`meta_settings.max_pages_per_user`、デフォルト100ページ）
- [ ] ページタイプ別の上限設定

### 5.2 アンテナ機能の拡張
**ソース**: [Issue #10511](https://github.com/misskey-dev/misskey/issues/10511)

**要望**:
- 「全て既読にする」ボタン
- 未読ジャンプ機能
- 検索サーバー選択

**misskey-rsでの対応状況**: ✅ 一部実装済み
- [x] mark-all-as-read API
- [ ] 未読位置へのジャンプ機能
- [ ] サーバー指定検索

### 5.3 チャンネルへのリノート機能
**ソース**: [Issue #12921](https://github.com/misskey-dev/misskey/issues/12921)

**要望**: 特定のチャンネルにリノートする機能

**misskey-rsでの対応案**:
- [ ] チャンネル指定リノートAPI
- [ ] クロスチャンネル共有

### 5.4 MFMからのプラグインインストール
**ソース**: [Issue #12986](https://github.com/misskey-dev/misskey/issues/12986)

**要望**: MFMコードブロックにpluginと指定するとインストールボタンを表示

**misskey-rsでの対応**: フロントエンド側の改善

### 5.5 未添付ドライブデータの一括削除
**ソース**: [Issue #12843](https://github.com/misskey-dev/misskey/issues/12843)

**要望**: どのノートにも添付されていないドライブファイルを一括削除

**misskey-rsでの対応状況**: ✅ 実装済み
- [x] 未使用ファイル検出API（`/drive/files/cleanup/preview`）
- [x] 一括削除API（`/drive/files/cleanup/execute`）
- [x] 削除前プレビュー（削除対象ファイル一覧、ファイル数、合計サイズを表示）
- [x] ノート、ページ、スケジュール投稿、アバター/バナー使用状況をチェック

### 5.6 仮想通貨チップ機能
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: バーチャル通貨でのチップ送金機能

**misskey-rsでの対応案**:
- [ ] インスタンス内ポイントシステム
- [ ] 外部ウォレット連携（オプション）

### 5.7 カスタム絵文字リクエストフォーム
**ソース**: [Issue #10221](https://github.com/misskey-dev/misskey/issues/10221)

**要望**: Misskey内で完結するカスタム絵文字のリクエストフォーム

**misskey-rsでの対応案**:
- [ ] 絵文字リクエストエンティティ
- [ ] モデレーター承認フロー
- [ ] 自動適用

### 5.8 絵文字インポート制御
**ソース**: [Issue #10822](https://github.com/misskey-dev/misskey/issues/10822)

**要望**: 自サーバーで作成した絵文字の他サーバーでのインポート可否設定

**misskey-rsでの対応案**:
- [ ] 絵文字ライセンスフィールド
- [ ] インポート許可フラグ
- [ ] ActivityPub経由での権限表現

### 5.9 水平スケーリングテスト
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: 水平スケーリング環境でのテスト実施（2票獲得）

**misskey-rsでの対応案**:
- [ ] Kubernetes対応
- [ ] 水平スケーリングのドキュメント
- [ ] 分散キャッシュ対応

### 5.10 バックエンドテストのVitest移行
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: バックエンドテストをVitestに変更

**misskey-rsでの対応**: N/A（Rustではcargo testを使用）

### 5.11 ロールベース表示ルール
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: ロールに基づいた表示ルールでコミュニティ管理を改善

**misskey-rsでの対応案**:
- [ ] ロールベースの表示制御API
- [ ] カスタムロール定義
- [ ] ロール継承

### 5.12 返信のタイムライン非表示オプション
**ソース**: [Discussion Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)

**要望**: 返信をタイムラインから非表示にし、ノート詳細でのみ表示

**misskey-rsでの対応案**:
- [ ] タイムラインフィルターオプション
- [ ] ユーザー設定との連携

---

## 6. misskey-rsでの優先実装候補

調査結果を基に、misskey-rsで優先的に実装すべき機能を選定：

### Tier 1: 高優先度（本家との差別化＋強い要望）

| 機能 | 理由 | 実装難易度 |
|------|------|-----------|
| チャンネルのフェデレーション | 多くの要望、ActivityPub Group対応 | 高 |
| リモートリアクション取得 | パフォーマンス大幅改善の可能性 | 中 |
| 水平スケーリング対応 | Rust実装の強み活用 | 中 |
| WebSocket最適化 | パフォーマンス差別化 | 中 |

### Tier 2: 中優先度（UX改善＋実装済み機能の拡張）

| 機能 | 理由 | 実装難易度 | 状況 |
|------|------|-----------|------|
| ノート編集のフェデレーション | API実装済み、連合対応が必要 | 低 | 🔧 進行中 |
| 引用リノートの互換性 | FEP対応でFediverse互換性向上 | 中 | - |
| Page文字数上限設定 | 管理者設定追加のみ | 低 | ✅ **実装済み** |
| 分割アップロード | 大容量対応 | 中 | - |
| デフォルト設定API | UIデフォルト値の管理 | 低 | ✅ **実装済み** |

### Tier 3: 低優先度（あると嬉しい機能）

| 機能 | 理由 | 実装難易度 | 状況 |
|------|------|-----------|------|
| 代名詞フィールド | プロフィール拡張 | 低 | ✅ **実装済み** |
| 未添付ファイル一括削除 | 運用改善 | 低 | ✅ **実装済み** |
| 絵文字リクエストフォーム | コミュニティ機能 | 中 | - |
| 仮想通貨チップ | ニッチな要望 | 高 | - |

---

## 7. 参照リンク

### GitHub Issues/Discussions
- [misskey-dev/misskey Issues](https://github.com/misskey-dev/misskey/issues)
- [misskey-dev/misskey Discussions - Ideas](https://github.com/misskey-dev/misskey/discussions/categories/ideas)
- [Performance Improvements Discussion #11069](https://github.com/misskey-dev/misskey/discussions/11069)

### 技術記事
- [Misskeyのパフォーマンス改善の取り組み・2023年7月](https://gihyo.jp/article/2023/07/misskey-05)
- [Misskeyのパフォーマンス改善の取り組み・2023年11月](https://gihyo.jp/article/2023/11/misskey-08)
- [MisskeyのUI設計](https://gihyo.jp/article/2023/10/misskey-07)
- [Misskeyで学ぶPostgresパフォーマンスチューニング入門](https://zenn.dev/nekokansystems/articles/19122ec09d5ccd)

### 公式ドキュメント
- [Misskey Hub - Release Notes](https://misskey-hub.net/en/docs/releases/)
- [Misskey API Reference](https://misskey-hub.net/en/docs/for-developers/api/)

---

## 更新履歴

| 日付 | 更新内容 |
|------|----------|
| 2025-12-11 | 初版作成 |
| 2025-12-11 | Phase 1-2機能実装完了: 代名詞フィールド、Page文字数上限設定、デフォルト設定API、未添付ファイル削除API |
