# CLI 出力方針

tsumugai の CLI は **人間向け出力** と **機械向け JSON 出力** を分けます。

---

## コマンド一覧

```bash
# シナリオの静的検証
tsumugai check scenario.md           # 人間向け出力
tsumugai check scenario.md --json    # 機械向け JSON 出力

# シナリオの対話再生
tsumugai play scenario.md
tsumugai play scenario.md --debug    # デバッグ情報付き
```

> `trace` / `dry-run` コマンドは将来追加予定（Issue #38, #40）。

---

## check コマンド：人間向け出力

### 問題なし（正常）

```
✓ 問題は見つかりませんでした。
```

- 終了コード: **0**

---

### 問題あり（エラー・警告・情報）

```
[エラー][undefined_label] 未定義ラベル 'good_end' へのジャンプが存在します
  提案: '[LABEL name=good_end]' を追加するか、ジャンプ先を修正してください
[警告][single_choice_branch] BRANCH 命令の選択肢が1つしかありません（分岐になっていません）
[情報][unreferenced_label] ラベル 'unused' はどこからも参照されていません

エラー: 1件  警告: 1件  情報: 1件
```

- `[レベル][rule_id] メッセージ` の形式
- 提案がある場合は次行に `  提案: ...` を表示
- サマリー行はエラー・警告・情報の件数を表示
- 終了コード: エラーあり → **1**、エラーなし → **0**

---

## check コマンド：JSON 出力（`--json`）

### 問題なし

```json
{
  "status": "ok",
  "error_count": 0,
  "warning_count": 0,
  "issues": []
}
```

### 問題あり（エラー）

```json
{
  "status": "error",
  "error_count": 1,
  "warning_count": 0,
  "issues": [
    {
      "rule_id": "undefined_label",
      "level": "error",
      "message": "未定義ラベル 'good_end' へのジャンプが存在します",
      "span": null,
      "related_spans": [],
      "suggestion": "'[LABEL name=good_end]' を追加するか、ジャンプ先を修正してください"
    }
  ]
}
```

### パースエラー時

```json
{
  "status": "error",
  "error_count": 1,
  "warning_count": 0,
  "issues": [
    {
      "rule_id": "parse_error",
      "level": "error",
      "message": "Undefined label 'missing' referenced in scenario",
      "span": null,
      "related_spans": [],
      "suggestion": null
    }
  ]
}
```

- パースエラーでも JSON 形式は崩れない
- `status` は `"ok"` または `"error"`（`"warning"` はなく、警告のみなら `"ok"`）
- 終了コード: `status == "error"` → **1**、それ以外 → **0**

---

## JSON スキーマ（check）

```json
{
  "status": "ok" | "error",
  "error_count": number,
  "warning_count": number,
  "issues": [
    {
      "rule_id": string,
      "level": "error" | "warning" | "info",
      "message": string,
      "span": { "line": number, "column": number } | null,
      "related_spans": [{ "line": number, "column": number }],
      "suggestion": string | null
    }
  ]
}
```

`span` は現時点では常に `null`（パーサーが行番号を付与するようになれば有効化）。

---

## rule_id 一覧（check）

| rule_id | level | 説明 |
|---|---|---|
| `parse_error` | error | パース失敗（構文エラー） |
| `undefined_label` | error | 未定義ラベルへのジャンプ・分岐参照 |
| `empty_branch` | error | BRANCH 命令に選択肢がない |
| `single_choice_branch` | warning | BRANCH 命令の選択肢が1つだけ |
| `unreferenced_label` | info | どこからも参照されないラベル |

---

## 終了コードまとめ

| 状況 | 終了コード |
|---|---|
| 問題なし | 0 |
| 警告・情報のみ（エラーなし） | 0 |
| エラーあり | 1 |
| 不明なコマンド / 引数不足 | 1 |

---

## 将来の拡張（予定）

```bash
tsumugai trace scenario.md
tsumugai trace scenario.md --json

tsumugai dry-run scenario.md
tsumugai dry-run scenario.md --json
```

- `trace`: ステップ実行のトレースログ（Issue #38）
- `dry-run`: 全分岐探索・エンディング網羅レポート（Issue #40）
- いずれも `--json` で機械向け出力を提供する方針
