# tsumugai

> **注記（2026-07-08）**
> TypeScript / Svelte / Vite への全面移行（[epic #99](https://github.com/kosuke-fujisawa/tsumugai/issues/99)）は検討の結果、不採用が確定しました。tsumugai は、以下に説明する通り Rust 製の semantic runtime / シナリオチェッカーとして維持します。新たに `compile --target web` コマンド（[#128](https://github.com/kosuke-fujisawa/tsumugai/issues/128)）を追加し、外部の Web フロントエンド（arikoi の Svelte 製 player 等）向けに StoryBundle JSON を出力できるようになりました。連携は npm 依存ではなく CLI サブプロセス経由で行います。

---

tsumugai（つむがい）は、  
**Markdown で書かれたノベルゲーム用シナリオを解析し、  
実行前に意味を確定し、検証し、再利用可能にする  
Rust 製の semantic runtime / シナリオチェッカーライブラリ**です。

描画・音声再生・UI 操作は行いません。  
代わりに、

> **「次に何が起こるのか」「分岐や条件は正しいのか」**

を、構造化されたデータとして返します。

---

## これは何か？（一言で）

> **ノベルゲーム用シナリオのための  
> 検証可能な意味実行基盤**

- ゲームエンジンではありません
- UI や演出は含みません
- Markdown を AST / IR に変換し、意味を確定してから実行します
- 実行・静的検証・将来的なドライラン検証に使えるコアです

詳しい思想と責務境界は [docs/CONCEPT.md](docs/CONCEPT.md) を参照してください。

---

## tsumugai がやること

tsumugai は Markdown 形式のシナリオを読み取り、  
以下を **順次・一貫して処理**します。

### 1. シナリオの解釈（パース）

- テキスト（セリフ／ナレーション）
- 選択肢
- 分岐・ジャンプ
- 条件付き分岐（フラグ・論理式）
- シナリオ進行上の状態

### 2. 「次に起こること」を Event として返却

tsumugai は、  
**描画や再生を行う代わりに「Event（意味イベント）」を返します。**

例：

- このテキストを表示してほしい
- ここで選択肢を出してほしい
- 次はこのラベルにジャンプする
- ユーザー入力を待つ

### 3. チェック・検証

**静的検証（`tsumugai check`）**:
- リンク切れ・シーン ID 重複・アセット実在チェック
- 話者名の書き間違い検出（characters.yaml との突き合わせ）
- 到達不能セクション・暗黙のフォールスルーの検出

**今後強化する検証**:
- 全分岐ドライラン
- エンディング到達検証
- 無限ループ・最大深度超過の検出
- 修正候補付き Diagnostic

👉 **「動かさなくても問題点が分かる」**のが特徴です。

---

## tsumugai がやらないこと（重要）

tsumugai は以下を **一切行いません**。

- テキスト・画像の描画
- 音声・BGM・SE の再生
- UI 操作・演出制御
- アニメーション

tsumugai の責務は、あくまで  
**「物語の意味と進行を決め、検証すること」**です。

```text
[ Markdown シナリオ ]
↓
tsumugai
↓
[ Event / Diagnostic / DryRunReport ]
↓
あなたのアプリケーション
```

---

## 想定している利用者

- Rust でノベルゲームを作りたいエンジニア
- UI・演出は自分で制御したい開発者
- シナリオを Markdown / Git で管理したいチーム
- LLM でシナリオ生成・修正・レビューを回したい人
- 書いたシナリオが正しいかを自動で確認したい人

完成済みノベルゲームエンジンの代替ではありません。

---

## 最小の使用例

### ライブラリ API

```rust
use tsumugai::scenario;
use std::path::Path;

let result = scenario::check_path(Path::new("scenario.md"), &scenario::CheckOptions::default());
for diag in &result.diagnostics {
    println!("[{}] {}", diag.rule_id, diag.message);
}
```

`scenario` モジュールの契約（`check` / `trace` / `routes` / `fmt` の入出力型）は [docs/API.md](docs/API.md) を参照してください。

### CLI

```bash
cargo run -- check examples/spring                    # ディレクトリごと静的検査
cargo run -- check examples/spring/scenario/spring_001.md
cargo run -- check examples/spring --format json      # CI・LLM 連携用 JSON
cargo run -- check examples/spring --format sarif     # GitHub Code Scanning 用 SARIF
cargo run -- fmt examples/fmt/before.md               # よくある書き方を v1 記法へ推測整形
cargo run -- compile examples/spring/scenario/spring_001.md --target web --output story-bundle.json
                                                       # arikoi 等の Web フロントエンド向け StoryBundle JSON を生成
```

- `check`: v1 記法（SPEC.md）の静的検査。構文・リンク切れ・話者名の書き間違い・シーン ID 重複・アセット実在などを一括検出する
- すべての指摘は「どこが・なぜ・どう直すか」を含み、最初のエラーで止まらず全件報告する（SPEC 6.1「Diagnostic は学習教材である」）

人間向け出力の例（アンカーの書き間違い）:

```text
error[broken-link]: このファイルに「run-togather」という見出し（##）はありません。よく似た「## run-together」があります。`[一緒に走る](#run-together)` の間違いではありませんか？
  --> scenario/spring_001.md:9
   |
 9 | - [一緒に走る](#run-togather)
   |
   = help: [一緒に走る](#run-together)

エラー: 1件  警告: 0件（1 ファイルを検査）
```

エラー時は exit code 1 を返します（警告のみなら 0）。JSON / SARIF はエラー時でも形式が崩れません。出力形式の詳細は [docs/CLI_OUTPUT.md](docs/CLI_OUTPUT.md) を参照してください。

---

## シナリオ形式

- 標準的な Markdown ファイル
- 見出し・コメントが使える
- 人間にも LLM にも読みやすい
- 独自タグは最小限

👉 「ライターが書きやすいこと」を最優先

詳細な記法は docs/ を参照してください。

---

## ドキュメント

- [Concept](docs/CONCEPT.md): 存在意義、設計思想、責務境界
- [Architecture](docs/ARCHITECTURE.md): アーキテクチャとデータフロー
- [API](docs/API.md): `scenario` モジュールの契約
- [Development Workflow](docs/DEVELOPMENT_WORKFLOW.md): 開発ワークフロー
- [Review Guide](docs/REVIEW_GUIDE.md): Rust を読まなくてもレビューできる手引き

## 設計方針

- テストファースト（TDD）
- 最小責務・単純な構造
- シナリオ解釈と検証に特化
- ライブラリは静かに振る舞う
- 実装より「意味」を返す

---

## tsumugai が目指すもの

- これさえあればシナリオの正しさを確認できる
- 実行にもチェックにも使える
- LLM フレンドリーなノベルゲーム開発基盤
- Bevy / iOS / Web / CUI などに接続できる意味コア

---

## 開発・CI

### ローカル確認コマンド

PR 前に以下を実行してください。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI（GitHub Actions）も同じコマンドを実行します。ローカルで通れば CI も通ります。

詳細な開発ワークフローは [docs/DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md) を参照してください。

---

## ライセンス

MIT License
