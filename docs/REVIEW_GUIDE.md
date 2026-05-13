# REVIEW_GUIDE

この文書は、Rust やコード読解に詳しくないユーザーが tsumugai の変更をレビューするための手引きです。

tsumugai では、Rust コードそのものだけでなく、以下がレビュー対象になります。

- 入力 Markdown（シナリオファイル）
- `check` の出力（人間向け・JSON向け）
- Diagnostic（エラー・警告）
- テスト結果
- README / docs の説明

---

## コードを読まずに確認できること

以下はコードを読まなくても確認できます。

| 確認したいこと | 何を見るか |
|---|---|
| このシナリオは valid か | `check` コマンドの出力 |
| エラーの内容と場所 | Diagnostic の `message` と `span` |
| 出力の形式が変わっていないか | `--json` 出力の比較 |
| PR が何を変えたか | PR本文の「入力例」と「出力例」 |
| テストが通っているか | CI のチェック結果 |
| ドキュメントと実装が合っているか | README / docs の記述と CLI 出力の比較 |

---

## README 差分の見方

PR に README の変更が含まれる場合、以下の点を確認します。

- 入力 Markdown の例が変わっていれば、**新しい書き方が正しいか**を確認する
- 出力例（JSON など）が変わっていれば、**変更前と変更後の意味の差分**を確認する
- 新しいコマンドや機能が追加されている場合、**非目標（tsumugai がやらないこと）** に反していないかを確認する

---

## Markdown 入力と JSON 出力の見方

tsumugai の入力はシナリオを記述した Markdown ファイルです。

**入力例（simple.md）**:

```markdown
[SAY speaker=Hero]
Hello, world!

[SAY speaker=Hero]
Goodbye!
```

これを `check --json` で検証すると、以下の出力になります。

```bash
cargo run -- check simple.md --json
```

**出力例（問題なし）**:

```json
{
  "status": "ok",
  "error_count": 0,
  "warning_count": 0,
  "issues": []
}
```

**出力例（エラーあり）**:

```json
{
  "status": "error",
  "error_count": 1,
  "warning_count": 0,
  "issues": [
    {
      "level": "error",
      "message": "ラベル 'good_end' へのジャンプが定義されていません"
    }
  ]
}
```

`status` が `"ok"` でなければ、`issues` の内容を確認します。`level` が `"error"` のものが修正必須、`"warning"` のものは確認推奨です。

---

## Diagnostic の見方

Diagnostic はエラーや警告の構造化情報です。将来的に以下の情報を持つ予定です。

| フィールド | 意味 |
|---|---|
| `rule_id` | どのルール違反か（例: `undefined_label`） |
| `severity` | `error` または `warning` |
| `message` | 何が問題か（日本語で説明） |
| `span` | 問題箇所の行・列 |
| `suggestion` | 修正の提案 |

**例**:

```json
{
  "rule_id": "undefined_label",
  "severity": "error",
  "message": "ラベル 'good_end' が定義されていません",
  "span": { "line": 12, "column": 8 },
  "suggestion": "LABEL name=good_end を追加するか、参照先を修正してください"
}
```

`message` と `suggestion` だけを読めば、コードを読まずに問題と対処が分かるよう設計されています。

---

## Dry Run Report の見方

（Dry Run は現在実装中です。この節は設計予定の内容です。）

Dry Run Report は、シナリオを全分岐探索した結果をまとめたものです。

```json
{
  "endings": [
    { "id": "good_end", "path": ["start", "go_right", "good_end"] },
    { "id": "bad_end",  "path": ["start", "go_left",  "bad_end"] }
  ],
  "unreachable_labels": ["unused_scene"],
  "loops": [],
  "diagnostics": []
}
```

確認するポイント:

- `endings` — 到達可能なエンディングの一覧。意図したエンディングがすべて含まれているか
- `unreachable_labels` — 到達不能なラベル。削除し忘れや分岐ミスの可能性
- `loops` — 無限ループが検出された場合に記録される
- `diagnostics` — 探索中に検出されたエラー・警告

---

## テスト結果の見方

PR の CI にはテスト結果が表示されます。

```text
test tests::check::undefined_label ... ok
test tests::runtime::branch_selects_correct_path ... ok
...
test result: ok. 42 passed; 0 failed; 0 ignored
```

- `failed` が 0 であることを確認します
- テスト名は「何をテストしているか」が分かるように命名されています。例: `undefined_label` → 未定義ラベルの検出テスト

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
（issues フィールドの内容）

## 懸念している点
（レビューしてほしい具体的な観点）
```

「コード全体を読ませる」より「入力と出力の差分」を渡すほうが、的確なフィードバックが得られます。

---

## レビューコメント例

Rust を読めなくても書けるレビューコメントの例です。

**入力例に関するコメント**:
> PR の入力例を手元で試したところ、`[JUMP label=undefined]` と書いた場合のエラーが確認できました。Diagnostic の message が「ラベル 'undefined' へのジャンプが定義されていません」と表示され、分かりやすいと思います。

**出力形式に関するコメント**:
> `check --json` の出力で `status` フィールドが `"error"` なのに `error_count` が 0 になるケースはありますか？ CI スクリプトから使うときに判定ロジックが混乱しそうです。

**ドキュメントと実装のズレに関するコメント**:
> README には `check --json` で `span` が含まれると書かれていますが、実際の出力には `span` フィールドがありませんでした。ドキュメントを合わせるか、実装を進める必要があると思います。

**テストに関するコメント**:
> 新しい BRANCH の挙動が追加されましたが、「条件が false のとき選択肢に表示されない」ケースのテストが見当たりません。追加してもらえると安心です。

---

## 参照

- [docs/CONCEPT.md](CONCEPT.md): tsumugai の存在意義と設計思想
- [docs/ARCHITECTURE.md](ARCHITECTURE.md): データフローとモジュール構成
- [docs/DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md): PR・レビュー・CI の手順
- [README.md](../README.md): セットアップと最小使用例
