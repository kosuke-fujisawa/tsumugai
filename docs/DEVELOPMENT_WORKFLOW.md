# DEVELOPMENT WORKFLOW

この文書は、tsumugai の開発・レビュー・CI・LLM連携の方針を定義します。

tsumugai は、Markdown ベースのノベルゲームシナリオを静的に検査・検証する CLI です。主要ユーザーは、Rust やコード読解に詳しいとは限りません。そのため、コードそのものだけでなく、入力、出力、診断、ログ、レポート、テスト、CI をレビュー対象にします。

## 基本方針

tsumugai の開発では、次を重視します。

- LLM によるデバッグ容易性
- ログ基盤と構造化出力
- CI / テストによる回帰検出
- コードを読まなくても確認できる入出力例
- 非RustユーザーでもレビューしやすいPR
- 入力 Markdown と出力結果の対応が追跡できること

目標は、次の状態です。

```text
このMarkdownを入力すると
このDiagnosticが出る
このEndingに到達する
このTraceになる
このStoryBundleが生成される
```

Rust コードを読めなくても、仕様と挙動を確認できる開発プロセスを作ります。

## 役割分担

- ChatGPT: 要件・仕様検討、Issue整理、リファクタリング方針、PRレビュー
- Claude Code: 実装、テスト追加、PR前セルフレビュー
- CodeRabbit: PRレビュー、細かな指摘
- Codex: コードレビュー、CI失敗調査、修正案
- Gemini CLI: ドキュメント整理、文章レビュー

役割は固定ではありませんが、レビュー観点を分けることで、仕様・実装・ドキュメントのズレを見つけやすくします。

## Issue ラベル方針

優先度ラベルは、単純な緊急度スケールではなく、次のカテゴリを表します。

| ラベル | 意味 |
|---|---|
| `P0` | 最優先：先に積むべき issue |
| `P1` | ゲーム制作に効く issue |
| `P2` | 連携・運用 issue |
| `P3` | 将来検討：着手時期未定の低優先 issue |

`P0`〜`P2` のどれにも明確に当てはまらない、着手時期未定の将来拡張・低優先issueには `P3` を付けます。`question` ラベルの issue（仕様や方針の検討）は、決定前のため優先度ラベルの対象外とします。

closed issue に遡って優先度ラベルを付け直す運用はしません。ラベルは、これから着手判断をする open issue のためのものです。

## 標準ワークフロー

1. 要求仕様を詰める
   - 目的、非目標、入力、出力、異常系、不変条件を整理する
   - 必要なら `SPEC.md` や Issue に SPEC-ID を付ける

2. レビュー可能な成果物を決める
   - 入力 Markdown
   - Diagnostic
   - Trace
   - RoutesReport
   - StoryBundle
   - Golden JSON
   - CLI 出力

3. テストを先に追加する
   - 可能な限り TDD で進める
   - 振る舞いが分かるテスト名にする
   - SPEC-ID がある場合はテスト名またはコメントに含める

4. 実装する
   - 変更範囲を小さく保つ
   - 不要な抽象化を避ける
   - `parse -> check/trace/routes/fmt -> report` の流れを崩さない

5. ローカル確認を行う
   - `cargo fmt --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`

6. PR本文にレビュー材料を載せる
   - 何を変えたか
   - なぜ変えたか
   - 入力例
   - 出力例
   - 追加・変更したテスト
   - LLMに確認させやすいログ・JSON
   - 既知の未対応

7. CI と Botレビューを確認する
   - fmt / clippy / test が通ること
   - テスト削除・弱体化がないこと
   - docs と実装が矛盾しないこと

8. 必要ならテスト追加と修正を繰り返す

9. 対応すべきものがなくなれば merge する

## CI 失敗時の対応

CI は `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test` の順に実行されます。

| 失敗ステップ | 確認すること | ローカルコマンド |
|---|---|---|
| Format check | フォーマットが崩れている | `cargo fmt` |
| Clippy check | lint 警告がある | `cargo clippy --all-targets -- -D warnings` |
| Tests | テストが失敗している | `cargo test -- --nocapture` |

ローカルで全ステップを通してから PR を出してください。ローカルで通れば CI も通ります。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## コード設計の判断基準

tsumugai では、過度に高度なRust設計よりも、LLMと人間が追跡しやすい構造を優先します。

優先するもの:

- enum 中心
- struct 中心
- 純関数に近い設計
- 明示的な `State`
- 単純な責務分離
- 読みやすいモジュール名
- 入出力が説明しやすい関数

避けるもの:

- 深い trait 階層
- generic の乱用
- macro の多用
- core logic への早すぎる async 導入
- 複雑な DI
- 仕様がコードに埋もれる抽象化

抽象化は、実際の重複や複雑さを減らす場合だけ導入します。

## Diagnostic 方針

エラーや警告は、構造化 Diagnostic として扱います（実装済み）。型定義とフィールドの意味の正本は [DIAGNOSTIC.md](DIAGNOSTIC.md)、check ルールの一覧は [SPEC.md](../SPEC.md) 6章、JSON 出力形式は [CLI_OUTPUT.md](CLI_OUTPUT.md) を参照してください。

Diagnostic は、人間の修正作業だけでなく、LLM に再修正を依頼するための入力としても使います。opaque な文字列エラーに閉じ込めず、「どこを、なぜ、どう直すか」を判断できる情報（`rule_id` / `span` / `suggestion` など）を持たせます。

## Trace / Routes 方針

実行フローの検証は 2 つのコマンドで行います（SPEC 5章の実行モデルに基づく。実装済み）。

- `trace`: 1 経路の再現。どのブロックを通り、どこで入力待ち・終了になったかを追跡する。正本は [TRACE.md](TRACE.md)
- `routes`: 全分岐探索。到達可能なエンディング、到達不能なシーン、エンディングに到達しないルート、循環を検出する。正本は [ROUTES.md](ROUTES.md)

いずれも「コードではなくレポートで挙動を確認できる」ことを目的にしており、JSON 出力を LLM への調査依頼にそのまま渡せます。

## CLI 出力方針

CLI は人間向け出力と機械向け JSON 出力を分けます。現行コマンド:

```bash
tsumugai check path/           # --format human|json|sarif
tsumugai trace scenario.md     # --choices 1,3 --format human|json
tsumugai routes scenario.md    # --format human|json
tsumugai fmt scenario.md       # --write --format human|json
tsumugai compile scenario.md --target web --output bundle.json
```

方針:

- 人間向け出力は読みやすくする
- JSON 出力は LLM、CI、Golden テストで使いやすくする
- エラー時も JSON 形式が崩れないようにする
- 出力形式の正本は [CLI_OUTPUT.md](CLI_OUTPUT.md) に置く

## Golden JSON 方針

Rustコードを読まなくても挙動差分をレビューできるよう、代表シナリオに対する期待出力を JSON で保存します。

現行:

```text
tests/fixtures/compile/golden/
  spring_001.json    # examples/spring の StoryBundle 期待値
```

`tests/compile_test.rs` が `examples/spring` の compile 結果とこの Golden を比較します。意図した出力変更の場合は Golden を更新し、PR にその理由を書きます。Golden JSON は、仕様変更なのかバグなのかを判断するためのレビュー材料です。

## CI 方針

CI（`.github/workflows/ci.yml`）は push / PR ごとに以下を実行します。

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test                      # Golden JSON 比較・CLI テストを含む
cargo run -- compile examples/spring/scenario/spring_001.md --target web --output ...
```

将来的に追加したいもの:

- サンプルシナリオの `check` / `routes` の実行
- README サンプルの doctest

CI が失敗した場合、PRでは「どの入力で、どの出力が、どう変わったか」を説明できるようにします。

## PR テンプレート方針

PR には、非Rustユーザーでもレビューできる材料を載せます。

推奨テンプレート:

```md
## 何を変えたか

## なぜ変えたか

## レビューしてほしい観点

## 入力例

## 出力例

## 追加・変更したテスト

## LLMに確認させやすいログ・JSON

## 既知の未対応
```

コードを読んでほしい場合でも、可能な限り入力例と出力例を添えます。

## LLM にデバッグ依頼するときに渡す情報

LLM に原因分析を依頼するときは、できるだけ次をまとめます。

- 入力 Markdown
- 実際の出力 JSON
- 期待する出力 JSON
- Diagnostic
- Trace
- 関連するテスト名
- 直近の差分概要

「コード全体を読ませる」のではなく、「再現入力と観測結果」を渡せる状態を目指します。

## 非目標

この開発プロセスでは、次を目的にしません。

- 高度な Clean Architecture 化
- 複雑な DI 導入
- Bevy / iOS アダプターの同時実装
- UI 実装
- アセットロード実装
- async 対応
- レビュー不能な大規模リファクタ

## まとめ

tsumugai では、レビュー容易性をコード読解能力に依存させません。

コードそのものだけでなく、入力、出力、診断、ログ、レポート、テスト、CIをレビュー対象にすることで、LLMと人間の両方がデバッグしやすい開発プロセスを整えます。
