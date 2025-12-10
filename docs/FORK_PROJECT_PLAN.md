# Misskey Fork Project - Internal Overhaul Plan

## Executive Summary

Misskeyをフォークし、本家から完全に独立した形で以下を重点的に改良する計画書。

**改良対象:** バックエンド全体 + ActivityPub実装
**主な目的:** パフォーマンス向上、コード品質・保守性、新機能追加の基盤、技術スタック刷新
**上流との関係:** 完全に独立（本家からのマージは行わない）

---

## Phase 0: 準備フェーズ

### 0.1 フォーク基盤の確立

- [ ] 新規リポジトリの作成とブランチ戦略策定
- [ ] CI/CD パイプラインの構築（GitHub Actions / GitLab CI）
- [ ] 開発環境のDocker化強化
- [ ] コントリビューションガイドライン策定

### 0.2 現状分析とベンチマーク

- [ ] 現行パフォーマンス計測（API応答時間、DB クエリ）
- [ ] ボトルネック特定のためのプロファイリング
- [ ] テストカバレッジ計測
- [ ] 依存関係の脆弱性スキャン

---

## Phase 1: コード品質・基盤整備

### 1.1 大規模サービスの分割

**対象ファイルと分割案:**

| 現行サービス | 行数 | 分割後 |
|-------------|------|-------|
| NoteCreateService | 1,197行 | NoteValidationService, NoteContentService, NoteDeliveryService |
| UserEntityService | 731行 | UserSerializationService, UserRelationService, UserCacheService |
| ApPersonService | 1,100+行 | ApPersonResolverService, ApPersonSyncService, ApPersonValidatorService |
| ApNoteService | 600+行 | ApNoteResolverService, ApNoteCreatorService |

**実装方針:**
```typescript
// Before: 単一巨大サービス
class NoteCreateService {
  async create(...) { /* 1200行の処理 */ }
}

// After: 責務分離
class NoteValidationService {
  async validate(data: NoteInput): Promise<ValidationResult> { }
}

class NoteContentService {
  async processContent(text: string): Promise<ProcessedContent> { }
}

class NoteDeliveryService {
  async deliverToFollowers(note: Note): Promise<void> { }
}

class NoteCreateService {
  constructor(
    private validation: NoteValidationService,
    private content: NoteContentService,
    private delivery: NoteDeliveryService,
  ) {}

  async create(data: NoteInput): Promise<Note> {
    await this.validation.validate(data);
    const processed = await this.content.processContent(data.text);
    const note = await this.save(processed);
    await this.delivery.deliverToFollowers(note);
    return note;
  }
}
```

### 1.2 イベント駆動アーキテクチャの形式化

**現状の問題:**
- GlobalEventService が複雑なイベント管理
- イベント型が暗黙的

**改善:**
```typescript
// イベントスキーマ定義
interface DomainEvents {
  'note.created': { note: Note; author: User };
  'note.deleted': { noteId: string; deletedBy: User };
  'user.followed': { follower: User; followee: User };
  'reaction.added': { note: Note; reaction: string; user: User };
}

// 型安全なイベントバス
class TypedEventBus {
  emit<K extends keyof DomainEvents>(event: K, data: DomainEvents[K]): void;
  on<K extends keyof DomainEvents>(event: K, handler: (data: DomainEvents[K]) => void): void;
}
```

### 1.3 エラーハンドリング統一

**改善点:**
- カスタムエラークラス階層の整理
- Result型パターンの導入検討
- エラーコードの体系化

```typescript
// エラー階層
abstract class AppError extends Error {
  abstract readonly code: string;
  abstract readonly httpStatus: number;
}

class NotFoundError extends AppError {
  code = 'NOT_FOUND';
  httpStatus = 404;
}

class ValidationError extends AppError {
  code = 'VALIDATION_ERROR';
  httpStatus = 400;
}

// Result型（Option）
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
```

---

## Phase 2: データベース層の改良

### 2.1 スキーマ最適化

**即時対応:**
- [ ] `Note.text` を `varchar` に移行（段階的）
- [ ] `NoteReaction.noteUserId` のdenormalization追加
- [ ] 未使用インデックスの特定と削除

**マイグレーション例:**
```sql
-- Note.text の型変更（大規模テーブル対応）
ALTER TABLE "note"
  ALTER COLUMN "text" TYPE varchar(8192)
  USING "text"::varchar(8192);

-- NoteReaction denormalization
ALTER TABLE "note_reaction"
  ADD COLUMN "noteUserId" character varying(32);

CREATE INDEX CONCURRENTLY "IDX_note_reaction_note_user_id"
  ON "note_reaction" ("noteUserId");
```

### 2.2 リポジトリパターンの強化

**改善:**
```typescript
// 現行
const note = await this.notesRepository.findOne({ where: { id } });

// 改善: 明示的なリポジトリメソッド
interface NotesRepository {
  findById(id: string): Promise<Note | null>;
  findByUser(userId: string, options: PaginationOptions): Promise<Note[]>;
  findByThread(threadId: string): Promise<Note[]>;
  createWithTransaction(data: CreateNoteInput, tx: EntityManager): Promise<Note>;
}
```

### 2.3 キャッシュ戦略の統一

**現状:**
- TypeORM クエリキャッシュ（Redis）
- 個別サービスでのアドホックキャッシュ

**改善:**
```typescript
// 統一キャッシュレイヤー
interface CacheService {
  // タグベース無効化
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T, options: CacheOptions): Promise<void>;
  invalidateByTag(tag: string): Promise<void>;
}

// 使用例
@Cacheable({ key: 'user:{id}', tags: ['user', 'user:{id}'], ttl: 3600 })
async getUser(id: string): Promise<User> { }

// 無効化
await cache.invalidateByTag(`user:${userId}`);
```

### 2.4 大規模テーブル対策

**Noteテーブル:**
- パーティショニング検討（月別/年別）
- アーカイブ戦略

```sql
-- パーティショニング例（PostgreSQL 12+）
CREATE TABLE note_partitioned (
  id varchar(32),
  created_at timestamptz,
  -- other columns
) PARTITION BY RANGE (created_at);

CREATE TABLE note_y2024m01 PARTITION OF note_partitioned
  FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

---

## Phase 3: ActivityPub実装の改良

### 3.1 型安全性の向上

**現状の問題:**
- `IObject` インターフェースが `any[]` を多用
- バリデーションが分散

**改善:**
```typescript
// 厳密な型定義
interface APNote {
  '@context': APContext;
  type: 'Note';
  id: string;
  attributedTo: string;
  content: string;
  published: string;
  to: string[];
  cc?: string[];
  inReplyTo?: string | null;
  // Misskey拡張
  _misskey_content?: string;
  _misskey_quote?: string;
}

// Zodによるバリデーション
const APNoteSchema = z.object({
  '@context': z.union([z.string(), z.array(z.unknown())]),
  type: z.literal('Note'),
  id: z.string().url(),
  attributedTo: z.string().url(),
  content: z.string(),
  published: z.string().datetime(),
  // ...
});
```

### 3.2 配信最適化

**改善項目:**
- [ ] sharedInbox集約の最適化
- [ ] 配信失敗時のインテリジェントリトライ
- [ ] Dead Letter Queue の実装

```typescript
// 配信リトライ戦略
interface DeliveryRetryStrategy {
  // 段階的バックオフ
  calculateBackoff(attempts: number): number;

  // 失敗パターン別処理
  handleFailure(error: DeliveryError): RetryDecision;

  // DLQ移行条件
  shouldMoveToDLQ(attempts: number, lastError: Error): boolean;
}

// DLQ処理
class DeadLetterQueueProcessor {
  async process(job: FailedDeliveryJob): Promise<void> {
    // 手動介入が必要なジョブを管理
    await this.notifyAdmins(job);
    await this.logForAnalysis(job);
  }
}
```

### 3.3 コレクション処理の改善

**現状の問題:**
- followers/following コレクションのページング未実装
- 大規模インスタンスでメモリ圧力

**改善:**
```typescript
// ストリーミング処理
async function* resolveCollectionItems(
  collectionUri: string
): AsyncGenerator<APObject> {
  let nextPage = collectionUri;

  while (nextPage) {
    const page = await fetch(nextPage);
    for (const item of page.items) {
      yield item;
    }
    nextPage = page.next;
  }
}

// 使用例
for await (const follower of resolveCollectionItems(followersUri)) {
  await processFollower(follower);
}
```

### 3.4 署名検証の強化

**追加検討:**
- [ ] LD Signatures (JSON-LD Signatures) 対応強化
- [ ] HTTP Message Signatures (RFC 9421) 対応準備
- [ ] 鍵ローテーション対応

---

## Phase 4: API層の改良

### 4.1 APIバージョニング

**現状:** バージョン管理なし
**改善:** `/api/v2/` プリフィックス導入

```typescript
// バージョン別ルーティング
const v1Endpoints = [/* 既存エンドポイント */];
const v2Endpoints = [/* 新規・改良エンドポイント */];

// 非推奨警告
@Deprecated({ since: '2025.1', removeIn: '2026.1', alternative: 'v2/notes/create' })
export class NotesCreate extends Endpoint { }
```

### 4.2 GraphQL導入検討

**メリット:**
- クライアント側のデータ取得最適化
- N+1問題の解決（DataLoader）
- 型安全なAPIスキーマ

**実装案:**
```typescript
// GraphQL スキーマ（並行運用）
type Query {
  note(id: ID!): Note
  user(id: ID!): User
  timeline(cursor: String, limit: Int): TimelineConnection
}

type Note {
  id: ID!
  text: String
  author: User!
  reactions: [Reaction!]!
  replyTo: Note
}

// DataLoader で N+1 解決
const userLoader = new DataLoader(async (ids) => {
  const users = await usersRepository.findByIds(ids);
  return ids.map(id => users.find(u => u.id === id));
});
```

### 4.3 レート制限の改善

**改善:**
```typescript
// 段階的レート制限
interface RateLimitConfig {
  // ユーザーレベル別
  anonymous: { requests: 60, window: '1m' };
  authenticated: { requests: 300, window: '1m' };
  premium: { requests: 1000, window: '1m' };

  // エンドポイント別オーバーライド
  endpoints: {
    'notes/create': { requests: 30, window: '1m' };
    'drive/files/create': { requests: 10, window: '1m' };
  };
}
```

---

## Phase 5: キュー処理の改良

### 5.1 監視・可観測性

**追加:**
- [ ] Prometheus メトリクス
- [ ] Grafana ダッシュボード
- [ ] アラート設定

```typescript
// メトリクス収集
const queueMetrics = {
  jobsProcessed: new Counter('queue_jobs_processed_total'),
  jobsFailed: new Counter('queue_jobs_failed_total'),
  jobDuration: new Histogram('queue_job_duration_seconds'),
  queueDepth: new Gauge('queue_depth'),
};
```

### 5.2 優先度キュー

**改善:**
```typescript
// 優先度別キュー
const queues = {
  critical: new Queue('critical', { priority: 1 }),  // 即時配信
  high: new Queue('high', { priority: 2 }),          // 通常配信
  normal: new Queue('normal', { priority: 3 }),      // バッチ処理
  low: new Queue('low', { priority: 4 }),            // エクスポート等
};
```

---

## Phase 6: 技術スタック刷新

### 6.1 依存関係の更新

| パッケージ | 現行 | 更新後 | 備考 |
|-----------|------|-------|------|
| NestJS | 11.x | 最新安定版 | 破壊的変更確認 |
| TypeORM | 0.3.x | 継続 or Drizzle検討 | パフォーマンス比較 |
| BullMQ | 5.x | 最新 | |
| Fastify | 5.x | 最新 | |

### 6.2 ORM移行検討

**Drizzle ORMの検討:**
```typescript
// Drizzle の利点
// - 型安全性が高い
// - クエリビルダーが直感的
// - パフォーマンスが良い

// 移行例
const notes = await db
  .select()
  .from(notesTable)
  .where(eq(notesTable.userId, userId))
  .orderBy(desc(notesTable.id))
  .limit(20);
```

**移行リスク:**
- 大規模な書き換えが必要
- マイグレーションシステムの変更
- 段階的移行の複雑さ

### 6.3 ビルドシステム

**現行:** SWC + TypeScript
**検討:**
- Bun ランタイムへの移行可能性
- ESBuild への統一

---

## Phase 7: テスト戦略

### 7.1 テストカバレッジ目標

| レイヤー | 現状 | 目標 |
|---------|------|------|
| ユニットテスト | 部分的 | 80%+ |
| インテグレーション | 限定的 | 60%+ |
| E2E | 限定的 | 主要フロー100% |
| コントラクトテスト | なし | API全体 |

### 7.2 テスト自動化

```typescript
// コントラクトテスト（PACT）
describe('Notes API Contract', () => {
  it('should create note with valid data', async () => {
    await provider.addInteraction({
      state: 'user is authenticated',
      uponReceiving: 'a request to create a note',
      withRequest: {
        method: 'POST',
        path: '/api/notes/create',
        body: { text: 'Hello' },
      },
      willRespondWith: {
        status: 200,
        body: like({ createdNote: { id: string(), text: 'Hello' } }),
      },
    });
  });
});
```

### 7.3 フェデレーションテスト強化

- [ ] 複数インスタンス間の自動テスト
- [ ] 互換性テスト（Mastodon, Pleroma等）
- [ ] パフォーマンステスト

---

## 実装優先順位

### 高優先度（Phase 1-2）
1. サービス分割（NoteCreateService, UserEntityService）
2. Note.text の型最適化
3. NoteReaction denormalization
4. キャッシュ戦略統一

### 中優先度（Phase 3-4）
5. ActivityPub型安全性向上
6. APIバージョニング導入
7. 配信最適化
8. テストカバレッジ向上

### 低優先度（Phase 5-6）
9. GraphQL導入
10. ORM移行検討
11. キュー監視強化

---

## リスクと対策

| リスク | 影響度 | 対策 |
|-------|-------|------|
| 大規模リファクタリングによるバグ | 高 | 段階的移行、徹底したテスト |
| ActivityPub互換性の破壊 | 高 | 互換性テストスイート、段階的導入 |
| パフォーマンス劣化 | 中 | ベンチマーク継続、A/Bテスト |
| 開発リソース不足 | 中 | 優先順位の明確化、コミュニティ参加 |

---

## 成功指標

- API応答時間: 平均50ms以下
- テストカバレッジ: 80%以上
- ActivityPub互換性: 主要実装との相互運用100%
- コードの認知的複雑度: 各メソッド15以下
- デプロイ頻度: 週1回以上

---

## 次のステップ

1. この計画書のレビューと承認
2. Phase 0 の準備作業開始
3. 詳細な技術設計書の作成（各Phase）
4. 開発チームの編成とタスク割り当て
