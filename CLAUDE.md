# CLAUDE.md

この文書は `tsumugai` 開発と利用のためのガイドラインです。  
LLM（Claude, ChatGPT など）にプロンプトを与える際に参照することで、統一された出力と開発体験を確保します。

---

## 🎯 開発設計方針

### 設計原則
- **TDD / テストファースト**
  - 必ず `cargo test` を先に書く。
  - ユニットテストと統合テストを両輪で活用する。

- **DRY 原則 & tidy first**
  - 明確な重複はまとめる。
  - ただし過剰抽象化は避け、まず動かしてから整理する。

- **関心の分離 (Separation of Concerns)**
  - モジュールは責務ごとに分割する。
    - `parser`：Markdown DSL → AST
    - `runtime`：AST 実行、状態管理
    - `storage`：セーブ／ロード
    - `cli`：サンプルランナー
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

[BRANCH choice=Start label=start, choice=Exit label=end] ✅ テスト方針 •ユニットテスト：各モジュールごと（parser, runtime, storage）。
•統合テスト：シナリオを通しで実行し、出力 JSON を golden test で比較。
•回帰テスト：セーブデータの互換性を常に検証。
