# Runtime Trace 設計

関連issue: #38

## 概要

`RuntimeTrace` は、シナリオを自動実行したときの「どの入力を与え、何が起き、状態がどう変わったか」を step 単位で記録する。

LLM にトレースを貼って「なぜこの台詞が出たか」「なぜこのルートに入ったか」を分析してもらう用途を想定する。

---

## 型定義

```rust
pub struct RuntimeTrace {
    pub total_steps: usize,
    /// MAX_STEPS(1000) に達した場合は true（無限ループ保護）
    pub truncated: bool,
    pub steps: Vec<TraceStep>,
}

pub struct TraceStep {
    pub step_no: usize,
    pub pc_before: usize,
    /// 与えた入力。最初の step は null
    pub input: Option<String>,
    pub pc_after: usize,
    pub events: Vec<Event>,
    pub state_diff: StateDiff,
    /// 次に何を待っているか。null = シナリオ終了
    pub waiting_for: Option<String>,
}

pub struct StateDiff {
    pub var_changes: Vec<VarChange>,
}

pub struct VarChange {
    pub key: String,
    pub before: Option<String>,  // 新規追加の場合は None
    pub after: String,
}
```

---

## 実行モデル

`trace_linear` は選択肢を常に先頭（表示条件を満たすもの）から選ぶ。

```
step(state, program, None)
  → waiting_for = Advance → step(_, _, Some(Advance))
  → waiting_for = Choice([opt0, opt1]) → step(_, _, Some(SelectChoice(opt0.id)))
  → waiting_for = Ended → 終了
```

各 `runtime::step()` 呼び出しが 1 つの `TraceStep` に対応する。

---

## CLI

```bash
# 人間向け出力
tsumugai trace scenario.md

# JSON 出力（CI・Golden テスト・LLM デバッグ用）
tsumugai trace scenario.md --json
```

---

## 人間向け出力例

```
=== Runtime Trace (3 steps) ===

Step 0 (pc 0 → 1)
  Input   : (初回実行)
  Event   : Say(Alice): こんにちは！
  State   : (変化なし)
  Waiting : Advance

Step 1 (pc 1 → 2)
  Input   : Advance
  Events  : (なし)
  State   : (変化なし)
  Waiting : (終了)
```

---

## JSON 出力例

```json
{
  "status": "ok",
  "trace": {
    "total_steps": 3,
    "truncated": false,
    "steps": [
      {
        "step_no": 0,
        "pc_before": 0,
        "input": null,
        "pc_after": 1,
        "events": [{ "Say": { "speaker": "Alice", "text": "こんにちは！" } }],
        "state_diff": { "var_changes": [] },
        "waiting_for": "Advance"
      }
    ]
  }
}
```

---

## 変数変化の例

`[SET name=score value=10]` を実行すると：

```json
"state_diff": {
  "var_changes": [
    { "key": "score", "before": null, "after": "10" }
  ]
}
```

`[MODIFY name=score op=add value=5]` の後は：

```json
"var_changes": [
  { "key": "score", "before": "10", "after": "15" }
]
```

---

## 現時点の制約

- `trace_linear` は「常に先頭選択肢を選ぶ」単一ルートのみトレースする
- 全分岐を探索する DFS/BFS トレースは [Dry Run Report (#40)](https://github.com/kosuke-fujisawa/tsumugai/issues/40) で扱う
- パーサーが行番号を付与していないため、`span` は現時点で持たない
