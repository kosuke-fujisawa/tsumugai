# Diagnostic — 構造化エラー・警告の設計方針

この文書は、`tsumugai` の静的検査結果（`analyzer::Issue`）の型設計と JSON 出力形式を定義します。

---

## 1. 位置づけ

`analyzer::Issue` が Diagnostic の役割を担います。  
`tsumugai check` の結果として返され、LLM・CI・人間の三者が共通して扱える形式を目指します。

---

## 2. 型定義

```rust
pub struct Issue {
    pub rule_id: &'static str,       // ルール種別（機械的識別用）
    pub level: Level,                // Error / Warning / Info
    pub message: String,             // 人間向けの説明
    pub span: Option<Span>,          // 主要な位置（将来実装）
    pub related_spans: Vec<Span>,    // 関連位置（将来実装）
    pub suggestion: Option<String>,  // 修正提案（任意）
}

pub struct Span {
    pub line: usize,    // 1-origin
    pub column: usize,  // 1-origin
}
```

---

## 3. rule_id 一覧

| rule_id | level | 内容 |
|:---|:---|:---|
| `undefined_label` | Error | 未定義ラベルへのジャンプ・選択肢参照 |
| `unreferenced_label` | Info | どこからも参照されていないラベル |
| `empty_branch` | Error | 選択肢が0個の BRANCH |
| `single_choice_branch` | Warning | 選択肢が1個しかない BRANCH |
| `parse_error` | Error | パース失敗（`CheckJsonOutput::parse_error` が使用） |

---

## 4. JSON 出力例

### 正常

```json
{
  "status": "ok",
  "error_count": 0,
  "warning_count": 0,
  "issues": []
}
```

### エラーあり

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

---

## 5. CLIでの人間向け表示

```text
[エラー][undefined_label] 未定義ラベル 'good_end' へのジャンプが存在します
  提案: '[LABEL name=good_end]' を追加するか、ジャンプ先を修正してください
```

---

## 6. span について

現在 `span` は常に `null` です。  
パーサーが行番号を AstNode に付与するようになった段階で埋める予定です。  
`Span` 型はすでに定義済みのため、API の形式は変わりません。

---

## 7. 設計原則

- `rule_id` で LLM・CI がフィルタ・集計できるようにする
- `message` は日本語で人間が読める説明にする
- `suggestion` は省略可能だが、エラー系には極力付ける
- `span` / `related_spans` は将来のために型として予約し、現時点ではそれぞれ `null` / `[]`
