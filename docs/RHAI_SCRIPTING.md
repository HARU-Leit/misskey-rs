# Rhai Scripting Engine - AiScript代替実装

## Overview

MisskeyではAiScriptを使用してプラグインやPlay機能を実現しているが、Rust化に伴いRhaiをスクリプティングエンジンとして採用する。

---

## AiScript vs Rhai 比較

| 項目 | AiScript | Rhai |
|------|----------|------|
| 実装言語 | TypeScript | Rust |
| 型システム | 動的型付け | 動的型付け |
| サンドボックス | あり | あり（設計上の強み） |
| メモリ安全性 | JS依存 | Rustによる保証 |
| パフォーマンス | Node.js依存 | ネイティブ速度 |
| ドキュメント | 限定的 | 充実 (https://rhai.rs/) |
| エコシステム | Misskey専用 | 汎用・活発 |

---

## Rhai 基本情報

| 項目 | 内容 |
|------|------|
| バージョン | 1.21+ |
| リポジトリ | https://github.com/rhaiscript/rhai |
| ドキュメント | https://rhai.rs/book/ |
| ライセンス | MIT / Apache 2.0 |

### Cargo.toml

```toml
rhai = { version = "1.21", features = [
    "sync",           # マルチスレッド対応
    "metadata",       # 関数メタデータ
    "internals",      # 内部API（高度なカスタマイズ用）
]}
```

---

## セキュリティ・サンドボックス

Rhaiはサンドボックス環境として設計されており、以下が**デフォルトで無効化**されている：

- ファイルシステムアクセス
- ネットワークアクセス
- 外部プロセス実行
- 無限ループ（ステップ数制限）
- 過剰なメモリ使用

### 制限設定

```rust
use rhai::Engine;

fn create_sandboxed_engine() -> Engine {
    let mut engine = Engine::new();

    // 演算制限
    engine.set_max_operations(100_000);        // 最大演算回数
    engine.set_max_expr_depth(64);             // 式のネスト深度
    engine.set_max_call_levels(64);            // 関数呼び出し深度
    engine.set_max_string_size(10_000);        // 文字列最大長
    engine.set_max_array_size(10_000);         // 配列最大サイズ
    engine.set_max_map_size(10_000);           // マップ最大サイズ

    // 危険な機能を無効化（デフォルトで無効だが明示的に）
    engine.disable_symbol("eval");             // eval無効化

    engine
}
```

---

## AiScript構文 → Rhai構文 変換

### 変数宣言

```javascript
// AiScript
let x = 42
var y = "hello"

// Rhai
let x = 42;
let y = "hello";
```

### 関数定義

```javascript
// AiScript
@fn add(a, b) {
    return a + b
}

// Rhai
fn add(a, b) {
    a + b  // 最後の式が戻り値
}
// または
fn add(a, b) {
    return a + b;
}
```

### 条件分岐

```javascript
// AiScript
if (x == 1) {
    "one"
} elif (x == 2) {
    "two"
} else {
    "other"
}

// Rhai
if x == 1 {
    "one"
} else if x == 2 {
    "two"
} else {
    "other"
}
```

### ループ

```javascript
// AiScript
for (let i, 10) {
    <: i
}

// Rhai
for i in 0..10 {
    print(i);
}
```

### 配列・オブジェクト

```javascript
// AiScript
let arr = [1, 2, 3]
let obj = { a: 1, b: 2 }

// Rhai
let arr = [1, 2, 3];
let obj = #{ a: 1, b: 2 };  // Object Mapは #{ } 構文
```

---

## Misskey API バインディング

### ホスト関数の登録

```rust
use rhai::{Engine, Dynamic, Map, Array};

fn setup_misskey_api(engine: &mut Engine, ctx: MisskeyContext) {
    // Mk:api - API呼び出し
    let ctx_clone = ctx.clone();
    engine.register_fn("mk_api", move |endpoint: &str, params: Map| -> Dynamic {
        // 実際にはasync処理が必要なため、別途対応
        ctx_clone.call_api(endpoint, params)
    });

    // Mk:dialog - ダイアログ表示
    engine.register_fn("mk_dialog", |title: &str, text: &str| {
        // UIイベントを発行
    });

    // Mk:confirm - 確認ダイアログ
    engine.register_fn("mk_confirm", |title: &str, text: &str| -> bool {
        // UIイベントを発行して結果を待つ
        true
    });

    // Mk:save - データ保存
    let ctx_clone = ctx.clone();
    engine.register_fn("mk_save", move |key: &str, value: Dynamic| {
        ctx_clone.save_plugin_data(key, value);
    });

    // Mk:load - データ読み込み
    let ctx_clone = ctx.clone();
    engine.register_fn("mk_load", move |key: &str| -> Dynamic {
        ctx_clone.load_plugin_data(key)
    });
}
```

### AiScript互換レイヤー

AiScriptからの移行を容易にするため、互換関数を提供：

```rust
fn setup_aiscript_compat(engine: &mut Engine) {
    // Core:v - バージョン取得
    engine.register_fn("core_v", || "rhai-compat-1.0");

    // Core:type - 型取得
    engine.register_fn("core_type", |v: Dynamic| -> String {
        match v.type_name() {
            "i64" | "f64" => "num".into(),
            "string" | "ImmutableString" => "str".into(),
            "bool" => "bool".into(),
            "array" => "arr".into(),
            "map" => "obj".into(),
            _ => "unknown".into(),
        }
    });

    // Str:len - 文字列長
    engine.register_fn("str_len", |s: &str| -> i64 {
        s.chars().count() as i64
    });

    // Arr:len - 配列長
    engine.register_fn("arr_len", |arr: Array| -> i64 {
        arr.len() as i64
    });

    // Math系
    engine.register_fn("math_abs", |n: i64| n.abs());
    engine.register_fn("math_abs", |n: f64| n.abs());
    engine.register_fn("math_sqrt", |n: f64| n.sqrt());
    engine.register_fn("math_sin", |n: f64| n.sin());
    engine.register_fn("math_cos", |n: f64| n.cos());
}
```

---

## 非同期処理対応

RhaiはネイティブでAsyncをサポートしないため、以下のアプローチを採用：

### 方法1: ブロッキング実行（シンプル）

```rust
use tokio::runtime::Handle;

fn register_async_api(engine: &mut Engine, handle: Handle) {
    let h = handle.clone();
    engine.register_fn("mk_api", move |endpoint: &str, params: Map| -> Dynamic {
        h.block_on(async {
            // async API呼び出し
            call_api_async(endpoint, params).await
        })
    });
}
```

### 方法2: Promise風パターン

```rust
// スクリプト側でコールバックを渡す
engine.register_fn("mk_api_async", |endpoint: &str, params: Map, callback: FnPtr| {
    // 別スレッドで実行し、結果をコールバックで返す
});
```

### 方法3: Rhai + Tokio統合（推奨）

```rust
use rhai::{Engine, AST};
use tokio::sync::mpsc;

pub struct ScriptExecutor {
    engine: Engine,
    runtime: tokio::runtime::Runtime,
}

impl ScriptExecutor {
    pub async fn run_script(&self, ast: &AST) -> Result<Dynamic, Box<EvalAltResult>> {
        // Asyncコンテキストで実行
        let (tx, mut rx) = mpsc::channel(1);

        // APIコールはチャンネル経由で処理
        // ...
    }
}
```

---

## Plugin システム実装

### プラグイン構造

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: Option<String>,
    pub permissions: Vec<PluginPermission>,
    pub script: String,  // Rhaiスクリプト
    pub config: Option<PluginConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginPermission {
    ReadAccount,
    WriteAccount,
    ReadNotes,
    WriteNotes,
    ReadNotifications,
    // ...
}
```

### プラグイン実行環境

```rust
pub struct PluginRuntime {
    engine: Engine,
    plugins: HashMap<String, CompiledPlugin>,
}

struct CompiledPlugin {
    ast: AST,
    scope: Scope<'static>,
}

impl PluginRuntime {
    pub fn new() -> Self {
        let mut engine = create_sandboxed_engine();
        setup_misskey_api(&mut engine);
        setup_aiscript_compat(&mut engine);

        Self {
            engine,
            plugins: HashMap::new(),
        }
    }

    pub fn load_plugin(&mut self, plugin: &Plugin) -> Result<(), PluginError> {
        // スクリプトをコンパイル
        let ast = self.engine.compile(&plugin.script)?;

        // 初期スコープを作成
        let mut scope = Scope::new();
        scope.push("PLUGIN_ID", plugin.id.clone());

        self.plugins.insert(plugin.id.clone(), CompiledPlugin { ast, scope });
        Ok(())
    }

    pub fn call_hook(&mut self, plugin_id: &str, hook: &str, args: Dynamic) -> Result<Dynamic, PluginError> {
        let plugin = self.plugins.get_mut(plugin_id)
            .ok_or(PluginError::NotFound)?;

        self.engine.call_fn(&mut plugin.scope, &plugin.ast, hook, (args,))
            .map_err(PluginError::from)
    }
}
```

---

## Play (Misskey Play) 実装

### Play実行環境

```rust
pub struct PlayRuntime {
    engine: Engine,
}

impl PlayRuntime {
    pub fn new() -> Self {
        let mut engine = create_sandboxed_engine();

        // Play用UI関数
        engine.register_fn("ui_text", |id: &str, text: &str| -> Dynamic {
            // UIコンポーネント生成
            let mut map = Map::new();
            map.insert("type".into(), "text".into());
            map.insert("id".into(), id.into());
            map.insert("text".into(), text.into());
            map.into()
        });

        engine.register_fn("ui_button", |id: &str, label: &str, action: FnPtr| -> Dynamic {
            let mut map = Map::new();
            map.insert("type".into(), "button".into());
            map.insert("id".into(), id.into());
            map.insert("label".into(), label.into());
            map.into()
        });

        // ... 他のUIコンポーネント

        Self { engine }
    }

    pub fn execute(&self, script: &str, input: Dynamic) -> Result<PlayOutput, PlayError> {
        let ast = self.engine.compile(script)?;
        let mut scope = Scope::new();
        scope.push("input", input);

        let result: Dynamic = self.engine.eval_ast_with_scope(&mut scope, &ast)?;

        Ok(PlayOutput::from_dynamic(result))
    }
}
```

---

## AiScript → Rhai 変換ツール

自動変換スクリプト（部分的対応）:

```rust
pub fn convert_aiscript_to_rhai(aiscript: &str) -> String {
    let mut rhai = aiscript.to_string();

    // 関数定義変換
    rhai = rhai.replace("@fn ", "fn ");
    rhai = rhai.replace("@(", "fn (");

    // elif → else if
    rhai = rhai.replace("elif", "else if");

    // オブジェクトリテラル変換
    // { key: value } → #{ key: value }
    // 注: 完全な変換には構文解析が必要

    // 出力文変換
    // <: expr → print(expr)
    let re = regex::Regex::new(r"<:\s*(.+)").unwrap();
    rhai = re.replace_all(&rhai, "print($1);").to_string();

    // 変数宣言
    rhai = rhai.replace("var ", "let ");

    // セミコロン追加（簡易的）
    // 注: 完全な対応には構文解析が必要

    rhai
}
```

---

## マイグレーション戦略

### Phase 1: 基盤構築
- [ ] Rhaiエンジンのセットアップ
- [ ] サンドボックス設定
- [ ] 基本Misskey APIバインディング

### Phase 2: 互換レイヤー
- [ ] AiScript互換関数の実装
- [ ] 変換ツールの作成
- [ ] テストスイート構築

### Phase 3: プラグインシステム
- [ ] プラグインランタイム実装
- [ ] 権限システム
- [ ] プラグインストア連携

### Phase 4: Play機能
- [ ] UIコンポーネントバインディング
- [ ] Play実行環境
- [ ] 既存Playの互換性テスト

---

## 参考リンク

- [Rhai Book](https://rhai.rs/book/)
- [Rhai Playground](https://rhai.rs/playground/)
- [AiScript Documentation](https://github.com/aiscript-dev/aiscript)
- [Misskey Plugin System](https://misskey-hub.net/docs/for-developers/plugin/)

---

*Last Updated: 2025-12-11*
