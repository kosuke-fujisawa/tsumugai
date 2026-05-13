# DEVELOPMENT WORKFLOW

この文書は、tsumugai の開発・レビュー・CI・LLM連携の方針を定義します。

tsumugai は、Markdown ベースのノベルゲームシナリオを「検証可能な意味構造」として扱う semantic runtime です。主要ユーザーは、Rust やコード読解に詳しいとは限りません。そのため、コードそのものだけでなく、入力、出力、診断、ログ、レポート、テスト、CI をレビュー対象にします。

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
このEventが出る
このDiagnosticが出る
このEndingに到達する
このTraceになる
```

Rust コードを読めなくても、仕様と挙動を確認できる開発プロセスを作ります。

## 役割分担

- ChatGPT: 要件・仕様検討、Issue整理、リファクタリング方針、PRレビュー
- Claude Code: 実装、テスト追加、PR前セルフレビュー
- CodeRabbit: PRレビュー、細かな指摘
- Codex: コードレビュー、CI失敗調査、修正案
- Gemini CLI: ドキュメント整理、文章レビュー

役割は固定ではありませんが、レビュー観点を分けることで、仕様・実装・ドキュメントのズレを見つけやすくします。

## 標準ワークフロー

1. 要求仕様を詰める
   - 目的、非目標、入力、出力、異常系、不変条件を整理する
   - 必要なら `SPEC.md` や Issue に SPEC-ID を付ける

2. レビュー可能な成果物を決める
   - 入力 Markdown
   - 出力 Event
   - Diagnostic
   - Trace
   - DryRunReport
   - Golden JSON
   - CLI 出力

3. テストを先に追加する
   - 可能な限り TDD で進める
   - 振る舞いが分かるテスト名にする
   - SPEC-ID がある場合はテスト名またはコメントに含める

4. 実装する
   - 変更範囲を小さく保つ
   - 不要な抽象化を避ける
   - `parser -> analyzer -> compile -> runtime -> output` の流れを崩さない

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

tsumugai-core では、過度に高度なRust設計よりも、LLMと人間が追跡しやすい構造を優先します。

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

エラーや警告は、最終的に構造化 Diagnostic として扱える形を目指します。

想定する構造:

```rust
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub related_spans: Vec<Span>,
    pub suggestion: Option<String>,
}

pub struct Span {
    pub line: usize,
    pub column: usize,
}
```

JSON例:

```json
{
  "rule_id": "undefined_label",
  "severity": "error",
  "message": "ラベル 'good_end' が定義されていません",
  "span": { "line": 12, "column": 8 },
  "related_spans": [],
  "suggestion": "LABEL name=good_end を追加するか、参照先を修正してください"
}
```

Diagnostic は、人間の修正作業だけでなく、LLM に再修正を依頼するための入力としても使います。

## Runtime Trace 方針

runtime の挙動は、LLM と人間が追跡できる形で出力できるようにします。

想定する構造:

```rust
pub struct RuntimeTrace {
    pub steps: Vec<TraceStep>,
}

pub struct TraceStep {
    pub pc_before: usize,
    pub op: String,
    pub pc_after: usize,
    pub events: Vec<Event>,
    pub state_diff: StateDiff,
}
```

Trace で確認したいこと:

- どの命令を実行したか
- PC がどう変化したか
- State がどう変化したか
- どの Event が出たか
- どこで入力待ちになったか

## Dry Run Report 方針

全分岐探索やエンディング検証は、コードではなくレポートとして確認できるようにします。

想定する構造:

```rust
pub struct DryRunReport {
    pub endings: Vec<EndingPath>,
    pub unreachable_labels: Vec<String>,
    pub loops: Vec<LoopInfo>,
    pub diagnostics: Vec<Diagnostic>,
}
```

Dry Run で検出したいこと:

- 各エンディングへの到達パス
- 到達不能ラベル
- 到達不能ノード
- 無限ループまたは最大深度超過
- エンディングに到達しないルート
- 条件によって選べない選択肢

## CLI 出力方針

CLI は人間向け出力と機械向け JSON 出力を分けます。

想定:

```bash
tsumugai check scenario.md
tsumugai check scenario.md --json

tsumugai trace scenario.md
tsumugai trace scenario.md --json

tsumugai dry-run scenario.md
tsumugai dry-run scenario.md --json
```

方針:

- 人間向け出力は読みやすくする
- JSON 出力は LLM、CI、Golden テストで使いやすくする
- エラー時も JSON 形式が崩れないようにする
- 出力例を docs または tests/golden に残す

## Golden JSON 方針

Rustコードを読まなくても挙動差分をレビューできるよう、代表シナリオに対する期待出力を JSON で保存します。

想定:

```text
tests/fixtures/
  simple.md
  branch.md

tests/golden/
  simple.events.json
  branch.events.json
  branch.dryrun.json
```

Golden JSON は、仕様変更なのかバグなのかを判断するためのレビュー材料です。

## CI 方針

最低限、以下を CI で実行します。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

将来的に追加したいもの:

- サンプルシナリオの `check`
- サンプルシナリオの `dry-run`
- Golden JSON 比較
- README サンプルの doctest
- CLI 出力のスナップショットテスト

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
