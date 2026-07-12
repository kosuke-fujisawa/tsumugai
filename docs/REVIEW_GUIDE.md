# REVIEW_GUIDE

この文書は、Rust やコード読解に詳しくないユーザーが tsumugai の変更をレビューするための手引きです。

記法の正本は [SPEC.md](../SPEC.md)、出力形式の正本は [CLI_OUTPUT.md](CLI_OUTPUT.md) です。本書の例はレビューの読み方を示すためのもので、形式の詳細が食い違った場合はそれらの正本を優先してください。

tsumugai では、Rust コードそのものだけでなく、以下がレビュー対象になります。

- 入力 Markdown（v1 記法のシナリオファイル）
- `check` / `trace` / `routes` / `fmt` / `compile` の出力（人間向け・JSON・SARIF）
- Diagnostic（エラー・警告）
- Golden JSON（`tests/fixtures/compile/golden/`）
- テスト結果
- README / docs の説明

---

## コードを読まずに確認できること

| 確認したいこと | 何を見るか |
|---|---|
| このシナリオは valid か | `check` コマンドの出力 |
| エラーの内容と場所 | Diagnostic の `message` と `span` |
| 特定の選択でどう進むか | `trace --choices` の出力 |
| 全分岐がどこに到達するか | `routes` の出力 |
| 出力の形式が変わっていないか | `--format json` 出力の比較、Golden JSON の差分 |
| PR が何を変えたか | PR本文の「入力例」と「出力例」 |
| テストが通っているか | CI のチェック結果 |
| ドキュメントと実装が合っているか | README / docs の記述と CLI 出力の比較 |

---

## README / docs 差分の見方

PR に README や docs の変更が含まれる場合、以下の点を確認します。

- 入力 Markdown の例が変わっていれば、**新しい書き方が SPEC.md に沿っているか**を確認する
- 出力例（JSON など）が変わっていれば、**変更前と変更後の意味の差分**を確認する
- 新しいコマンドや機能が追加されている場合、**非目標（tsumugai がやらないこと）** に反していないかを確認する

---

## Markdown 入力と JSON 出力の見方

tsumugai の入力は v1 記法（[SPEC.md](../SPEC.md)）で書いた Markdown ファイルです。

**入力例（scene.md）**:

```markdown
---
id: typo_anchor
---

# 分岐テスト

校門までは、あと五百メートル。

- [一緒に走る](#run-togather)
- [全力で走る](#run-together)

## run-together

幼なじみ: ほら、急ぐよ！

<!-- ending: run -->
```

1つ目の選択肢のリンク先が `#run-togather`（typo）になっています。これを `check` で検証します。

```bash
cargo run -- check scene.md --format json
```

**出力例（エラーあり）**:

```json
{
  "diagnostics": [
    {
      "file": "scene.md",
      "message": "このファイルに「run-togather」という見出し（##）はありません。よく似た「## run-together」があります。`[一緒に走る](#run-together)` の間違いではありませんか？",
      "related_spans": [{ "column": null, "line": 12 }],
      "rule_id": "broken-link",
      "severity": "error",
      "span": { "column": null, "line": 9 },
      "suggestion": "[一緒に走る](#run-together)"
    }
  ],
  "error_count": 1,
  "files": ["scene.md"],
  "status": "error",
  "warning_count": 0
}
```

問題がなければ `"status": "ok"`、`"diagnostics": []` になります。`severity` が `"error"` のものが修正必須、`"warning"` のものは確認推奨です。exit code はエラー時 1、警告のみなら 0 です。

---

## Diagnostic の見方

Diagnostic はエラーや警告の構造化情報です（実装済み。型の正本は [DIAGNOSTIC.md](DIAGNOSTIC.md)、ルール一覧は [SPEC.md](../SPEC.md) 6章）。

| フィールド | 意味 |
|---|---|
| `rule_id` | どのルール違反か（例: `broken-link`） |
| `severity` | `error` または `warning` |
| `message` | 何が問題か（日本語で説明） |
| `file` / `span` | 問題箇所のファイルと行 |
| `related_spans` | 関連する箇所（リンク先候補など） |
| `suggestion` | そのまま貼り替えられる修正案 |

`message` と `suggestion` だけを読めば、コードを読まずに問題と対処が分かるよう設計されています。

---

## routes（全分岐探索）の見方

`routes` はシナリオの全分岐を探索し、各ルートの到達先をまとめます（正本は [ROUTES.md](ROUTES.md)）。

```bash
cargo run -- routes entry.md --format json
```

**出力例（`report` 部分の抜粋）**:

```json
{
  "reached_endings": ["b_end", "x_end", "y_end"],
  "unreached_endings": [],
  "unreachable_scenes": [],
  "routes": [
    { "choices": [1, 1], "end": { "id": "x_end", "reason": "ending" } },
    { "choices": [1, 2], "end": { "id": "y_end", "reason": "ending" } },
    { "choices": [2],    "end": { "id": "b_end", "reason": "ending" } }
  ],
  "truncated": false
}
```

確認するポイント:

- `reached_endings` — 意図したエンディングがすべて含まれているか
- `unreached_endings` — 定義したのに誰も到達できないエンディング。分岐ミスの可能性
- `unreachable_scenes` — どこからも到達できないシーン。削除し忘れやリンクミスの可能性
- `routes[].choices` — そのエンディングに到達する選択番号列。`trace --choices` にそのまま渡して再現できる

特定の1経路を確認したいときは `trace` を使います（正本は [TRACE.md](TRACE.md)）。

```bash
cargo run -- trace entry.md --choices 1,2
```

---

## テスト結果の見方

PR の CI にはテスト結果が表示されます。テスト名は日本語で「何を確認しているか」が分かるように命名されています。

```text
test 全ルールにsarif用の個別説明がある ... ok
test spring例のstorybundleはgolden_jsonと一致する ... ok
...
test result: ok. 120 passed; 0 failed; 0 ignored
```

- `failed` が 0 であることを確認します
- Golden JSON テストが失敗している場合、`compile` の出力形式が変わっています。PR がその変更を意図しているか（意図しているなら golden の更新理由が説明されているか）を確認します

---

## LLM にレビュー依頼するときに渡す情報

LLM（ChatGPT や Claude など）に変更の確認を依頼するときは、以下をまとめて渡すと効果的です。

```
## 変更の概要
（PR本文の「何を変えたか」）

## 入力 Markdown
（再現できる最小のシナリオ）

## 変更前の出力
（JSON または人間向け出力）

## 変更後の出力
（JSON または人間向け出力）

## Diagnostic（あれば）
（diagnostics フィールドの内容）

## 懸念している点
（レビューしてほしい具体的な観点）
```

「コード全体を読ませる」より「入力と出力の差分」を渡すほうが、的確なフィードバックが得られます。

---

## レビューコメント例

Rust を読めなくても書けるレビューコメントの例です。

**入力例に関するコメント**:
> PR の入力例を手元で試したところ、リンク先を `#run-togather` と typo した場合の `broken-link` エラーが確認できました。suggestion に修正後のリンクがそのまま出るのは分かりやすいと思います。

**出力形式に関するコメント**:
> `check --format json` の出力で `status` が `"error"` なのに `error_count` が 0 になるケースはありますか？ CI スクリプトから使うときに判定ロジックが混乱しそうです。

**ドキュメントと実装のズレに関するコメント**:
> docs には JSON 出力に `suggestion` が含まれると書かれていますが、実際の出力にはありませんでした。ドキュメントを合わせるか、実装を直す必要があると思います。

**テストに関するコメント**:
> 選択肢の上限チェックが追加されましたが、「ちょうど上限個のときはエラーにならない」ケースのテストが見当たりません。追加してもらえると安心です。

---

## 参照

- [SPEC.md](../SPEC.md): v1 記法と check ルールの正本
- [docs/CLI_OUTPUT.md](CLI_OUTPUT.md): human / JSON / SARIF 出力形式の正本
- [docs/CONCEPT.md](CONCEPT.md): tsumugai の存在意義と設計思想
- [docs/ARCHITECTURE.md](ARCHITECTURE.md): データフローとモジュール構成
- [docs/DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md): PR・レビュー・CI の手順
- [README.md](../README.md): セットアップと最小使用例
