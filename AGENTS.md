# AGENTS.md

> **📌 現在の方針（2026-07-08〜）**
> tsumugai は Rust 製 CLI のまま維持します（[epic #99](https://github.com/kosuke-fujisawa/tsumugai/issues/99) が目指したTypeScript化は撤回済み）。既存の `check` / `trace` / `routes` に加え、Web版ノベルゲーム「arikoi」（別リポジトリ）向けに `compile --target web` を追加し、StoryBundle JSONを出力する開発を進めています（[epic #129](https://github.com/kosuke-fujisawa/tsumugai/issues/129)）。以下の Rust 向け記述はそのまま現行実装に適用されます。

この文書は、Codex などの LLM エージェントが `tsumugai` を開発するときに必ず参照する行動指針です。

tsumugai の主要ユーザーは、Rust やコード読解に詳しいとは限りません。それでも仕様・挙動・出力に対するレビューは実施したい、という前提で開発します。

## 最優先方針

tsumugai では、テクニカルな設計美よりも次を優先します。

- デバッグしやすいこと
- レビューしやすいこと
- LLM に原因分析を依頼しやすいこと
- 入力 Markdown と出力結果の対応が追跡できること
- コードを読まなくても挙動を確認できること

レビューの単位は Rust コードだけではありません。入力 Markdown、出力 Event、Diagnostic、Trace、DryRunReport、Golden JSON、CLI 出力、テスト結果もレビュー対象です。

## 現在の責務分離

現行の主経路は次の通りです。

```text
Markdown
  -> parser::parse
  -> Ast
  -> runtime::compile
  -> IR Program
  -> runtime::step
  -> Output { events, waiting_for }
```

主要モジュール:

- `parser`: Markdown DSL を AST に変換する
- `analyzer`: AST を静的検証する
- `runtime`: AST を IR にコンパイルし、State/Input から Output を返す
- `player`: CUI の参照実装
- `types`: AST / State などの基本型

runtime は Markdown を直接読みません。parser は実行状態を持ちません。表示、音声再生、UI、アセットロードは core の責務ではありません。

## コード設計の制約

LLM と人間が追跡しやすい Rust を優先します。

優先するもの:

- enum / struct / 関数中心の単純な設計
- 明示的な `State`
- 入出力が分かる純粋関数に近い形
- 小さな責務分離
- 具体的な型名と分かりやすいモジュール名
- 失敗時に原因が分かるエラーと Diagnostic

避けるもの:

- 不要な trait 階層
- generic の乱用
- macro の多用
- core logic への早すぎる async 導入
- 複雑な DI
- 挙動が追いにくい抽象化
- コードを読まないと仕様が分からない変更

抽象化は、実際の重複や複雑さを減らす場合だけ導入します。

## 変更時に必要なレビュー材料

挙動を変える変更では、可能な限り次のいずれかを追加・更新します。

- 入力 Markdown 例
- 出力 Event 例
- Diagnostic 例
- Trace 例
- DryRunReport 例
- Golden JSON
- CLI 実行例
- テストケース
- README / docs の説明

PR や最終報告では、Rust コードを読まなくても何が変わったか分かるように説明します。

## TODO 管理方針

作業候補は `../claude_code/TODO.md` で管理します。

ユーザーは `../claude_code/TODO.md` の `Inbox` に自由にタスクを追加してかまいません。Codex は作業開始時に `../claude_code/TODO.md` を確認し、必要に応じてタスクへ ID を付け、`Backlog` / `Doing` / `Done` / `Dropped` を整理します。

タスクを完了した場合は、`Done` に移し、完了日、変更内容、確認したコマンドやレビュー材料を簡潔に残してください。タスクを削除せず、対応しない判断をした場合は `Dropped` に理由を残してください。

## Diagnostic 方針

エラーや警告は、最終的に構造化 Diagnostic として扱える形を目指します。

Diagnostic は以下を持つ想定です。

- `rule_id`
- `severity`
- `message`
- `span`
- `related_spans`
- `suggestion`

単なる opaque な文字列エラーに閉じ込めないでください。ユーザーや LLM が「どこを、なぜ、どう直すか」を判断できる情報を優先します。

## CLI / JSON / ログ方針

人間向け出力と機械向け JSON 出力を分けます。

将来的な CLI 方針:

```bash
tsumugai check scenario.md
tsumugai check scenario.md --json

tsumugai trace scenario.md
tsumugai trace scenario.md --json

tsumugai dry-run scenario.md
tsumugai dry-run scenario.md --json
```

JSON 出力は、CI、Golden テスト、LLM へのデバッグ依頼に使える安定した形を目指します。エラー時も形式が崩れないようにします。

## テストと品質ゲート

変更後は原則として以下を確認します。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

将来的に重視するテスト:

- Golden JSON 比較
- CLI 出力のスナップショット
- サンプルシナリオの `check`
- サンプルシナリオの `dry-run`
- README サンプルの doctest

テストを削除・弱体化する変更は慎重に扱い、理由を明記してください。

## ドキュメント更新

挙動、API、CLI、設計方針を変えた場合は、README または `docs/` を更新します。

特に参照する文書:

- `docs/CONCEPT.md`: 存在意義と責務境界
- `docs/ARCHITECTURE.md`: アーキテクチャ
- `docs/API.md`: Core と Host の契約
- `docs/DEVELOPMENT_WORKFLOW.md`: 開発・レビュー・CI 方針

ドキュメントと実装がズレると、非Rustユーザーのレビューが困難になります。ズレを見つけたら、小さくても修正してください。

## 最終報告の方針

作業完了時は、次を簡潔に報告します。

- 何を変えたか
- どのファイルを変えたか
- どの検証を通したか
- 未対応や注意点があるか

ユーザーがコードを読まなくても判断できる説明を優先します。
