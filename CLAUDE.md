# CLAUDE.md

この文書は `tsumugai` 開発と利用のためのガイドラインです。  
LLM（Claude, ChatGPT など）にプロンプトを与える際に参照することで、統一された出力と開発体験を確保します。

---

## 🎯 開発設計方針

### 新アーキテクチャ（2024年実装完了）
`tsumugai` は DDD/クリーンアーキテクチャから、よりシンプルで保守しやすい3モジュール構成に移行完了しました。

#### 新モジュール構成
- **`parser/`** - Markdown DSL → AST 変換
  - `parse(markdown: &str) -> Result<Ast>` 関数を提供
  - AST の妥当性検証（未定義ラベル検出など）

- **`runtime/`** - AST 実行と状態管理
  - `step(state: State, ast: &Ast, event: Option<Event>) -> (State, Output)` 関数
  - ステートレスな実行エンジン
  - 1ステップずつの確定的実行

- **`storage/`** - セーブ/ロード機能
  - `save(state: &State) -> Result<Vec<u8>>` でJSON形式保存
  - `load(bytes: &[u8]) -> Result<State>` で復元
  - バージョン管理対応済み

#### 統一API
```rust
// 新しいシンプルAPI
use tsumugai::{parser, runtime, storage, types::{State, Event}};

let ast = parser::parse(markdown)?;
let mut state = State::new();
let (new_state, output) = runtime::step(state, &ast, None);
let save_data = storage::save(&new_state)?;
```

### 設計原則
- **TDD / テストファースト**
  - 必ず `cargo test` を先に書く。
  - ユニットテストと統合テストを両輪で活用する。

- **DRY 原則 & tidy first**
  - 明確な重複はまとめる。
  - ただし過剰抽象化は避け、まず動かしてから整理する。

- **関心の分離 (Separation of Concerns)**
  - モジュールは責務ごとに分割する。
  - テストもモジュール境界で切る。

- **単一責任原則 (Single Responsibility Principle)**
  - 各モジュールは一つの理由でのみ変更される。
  - 例：文法拡張で `parser` が変わる、実行仕様変更で `runtime` が変わる。

- **品質ゲート (Quality Gate)**
  - コミット前に `cargo fmt` と `cargo clippy -- -D warnings` を実行し、修正する。

---

## 📝 シナリオ生成ルール

### 基本フォーマット
- シナリオは Markdown で書く。
- シーンラベルは `# scene: NAME`
- コマンドは `[COMMAND key=value ...]`
- コメントは `<!-- -->` で LLM にも人間にも読みやすく残す。

### サポートされるコマンド
- `[SAY speaker=...]` 台詞
- `[PLAY_BGM name=...]` BGM 再生
- `[SHOW_IMAGE name=...]` 画像表示
- `[PLAY_MOVIE name=...]` ムービー再生
- `[WAIT 2s]` 待機
- `[BRANCH choice=... label=...]` 選択肢分岐
- `[IF flag=... value=...]` 条件分岐
- `[SET flag=... value=...]` 内部状態変更

### 制約
- 出力はシナリオ本体のみ。説明文は不要。
- パラメータは必ず `key=value` 形式。
- 英語・日本語の混在可。
- 演出はタグで表現するが、描画実装は UI 側の責務。

---

## 📦 出力テンプレート

```markdown
# scene: intro

[SAY speaker=Alice]
Welcome to this world.

[PLAY_BGM name=intro.mp3]

[WAIT 1.5s]

[SAY speaker=Bob]
はじめようか？

[BRANCH choice=Start label=start, choice=Exit label=end]
```

---

## ✅ テスト方針

### テスト戦略
- **ユニットテスト**: 各モジュールごと（parser, runtime, storage）
- **統合テスト**: シナリオを通しで実行し、出力JSONをgolden testで比較
- **回帰テスト**: セーブデータの互換性を常に検証

### テスト実行
```bash
# 全テスト実行
cargo test

# 品質ゲート
cargo fmt
cargo clippy -- -D warnings
```
