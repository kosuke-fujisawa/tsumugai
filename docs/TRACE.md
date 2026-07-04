# Trace — 経路の再現（tsumugai trace）

関連: [SPEC.md 5.1](../SPEC.md)、issue #77

## 概要

`tsumugai trace` は、SPEC 5章の実行モデルに従ってシナリオを **1 経路ぶん**自動実行し、「どこを通り、何が起き、どう終わったか」を順に表示する。

想定する使い方:

- 書いたシナリオが意図した順で流れるかを、コードを読まずに確認する
- LLM にトレースを貼って「なぜこのルートに入ったか」「なぜここで終わったか」を分析してもらう
- `--choices` で経路を固定し、CI や Golden テストで回帰を検出する

全分岐の網羅は trace の責務ではない（routes #78 が担当）。

## CLI

```bash
tsumugai trace scenario.md                     # 最初の選択肢まで実行して停止
tsumugai trace scenario.md --choices 1,3,1     # 選択肢で 1 → 3 → 1 を選んで経路を再現
tsumugai trace scenario.md --format json       # 機械向け JSON（--json も同じ）
tsumugai trace scenario.md --no-assets         # 実行前検査のアセットチェックを省略
```

### 実行前検査（SPEC 6.1）

trace は実行の前に **check とまったく同じ検査**を行う。

- error があれば実行せず、check と同じ形式で Diagnostic を報告して終了コード 1
- warning のみなら、warning を表示したうえで実行する
- ユーザーが check を知らなくても、trace から入って同じ指摘に到達できる

### --choices の規則（SPEC 5.1）

- 選択肢ブロックに到達するたび、`--choices` の番号を先頭から 1 つ消費する
- 番号は**そのブロック内の項目の並び順（1 始まり）**。trace の出力に表示される番号がそのまま使える
- 番号が尽きたら入力待ちとして停止し、選択肢一覧と「次に足す番号の例」を表示する（終了コード 0）
- 項目数を超える番号と 0 は error（終了コード 1）
- 実行が終了した時点で未消費の番号が残っていれば、その旨を報告する

## 人間向け出力の例

`examples/spring` を `--choices 1` で実行した場合:

```text
=== Trace: examples/spring/scenario/spring_001.md ===

▶ シーン spring_001「春・出会い」 (examples/spring/scenario/spring_001.md)
      background: ../assets/bg/school_gate.png
      bgm: ../assets/bgm/spring.ogg
     9| 桜の花びらが舞う通学路。いつもと同じ朝のはずだった。
    11| 幼なじみ: おはよう。今日も遅刻しそうだね。
    13| 主人公: まだ間に合うよ。
    15| 校門までは、あと五百メートル。始業のチャイムまで、あと三分。
  ── セクション「選択肢」（17 行目）
    19| 選択肢:
          1. [一緒に走る](#run-together)
          2. [諦めて歩く](#walk-together)
          3. [先に行ってもらう](spring_002.md)
        → 1 を選択「一緒に走る」
  ── セクション「run-together」（23 行目）
    25| 幼なじみ: ほら、急ぐよ！
    27| 主人公: 待ってってば！
    29| エンディング: childhood_route

結果: エンディング「childhood_route」に到達しました
入力した選択肢: --choices 1
```

- 行頭の `9|` などは入力 Markdown の行番号。出力と入力の対応をそのまま追跡できる
- `▶ シーン` はシーンファイルへの進入（開始時とファイルをまたぐ移動時）
- `── セクション` はセクション（H2）への進入。フォールスルーとリンク着地の両方で表示されるため、**意図しない合流（implicit-fallthrough）も trace 上で見える**

`--choices` を付けずに実行すると、最初の選択肢で停止して番号を案内する:

```text
    19| 選択肢:
          1. [一緒に走る](#run-together)
          2. [諦めて歩く](#walk-together)
          3. [先に行ってもらう](spring_002.md)
        → （入力待ちで停止）

結果: 選択肢の入力待ちで停止しました。--choices に選択番号を足すと先へ進めます（例: --choices 1）
```

## JSON 出力（`--format json`）

check の JSON（[CLI_OUTPUT.md](CLI_OUTPUT.md)）の上位互換。`file`（開始シーン）と `trace` が加わる。

```json
{
  "status": "ok",
  "file": "examples/spring/scenario/spring_001.md",
  "files": [
    "examples/spring/scenario/spring_001.md",
    "examples/spring/scenario/spring_002.md"
  ],
  "error_count": 0,
  "warning_count": 0,
  "diagnostics": [],
  "trace": {
    "steps": [
      {
        "type": "scene_enter",
        "file": "examples/spring/scenario/spring_001.md",
        "id": "spring_001",
        "title": "春・出会い",
        "background": "../assets/bg/school_gate.png",
        "bgm": "../assets/bgm/spring.ogg"
      },
      {
        "type": "narration",
        "file": "examples/spring/scenario/spring_001.md",
        "line": 9,
        "text": "桜の花びらが舞う通学路。いつもと同じ朝のはずだった。"
      },
      {
        "type": "dialogue",
        "file": "examples/spring/scenario/spring_001.md",
        "line": 11,
        "speaker": "幼なじみ",
        "text": "おはよう。今日も遅刻しそうだね。"
      },
      {
        "type": "section_enter",
        "file": "examples/spring/scenario/spring_001.md",
        "line": 17,
        "heading": "選択肢",
        "anchor": "選択肢"
      },
      {
        "type": "choice",
        "file": "examples/spring/scenario/spring_001.md",
        "line": 19,
        "options": [
          { "label": "一緒に走る", "target": "#run-together" },
          { "label": "諦めて歩く", "target": "#walk-together" },
          { "label": "先に行ってもらう", "target": "spring_002.md" }
        ],
        "selected": 1
      },
      {
        "type": "ending",
        "file": "examples/spring/scenario/spring_001.md",
        "line": 29,
        "id": "childhood_route"
      }
    ],
    "end": { "reason": "ending", "id": "childhood_route" },
    "choices_requested": [1],
    "choices_used": 1
  }
}
```

### steps のステップ種別（`type`）

| type | 意味 | 主なフィールド |
|---|---|---|
| `scene_enter` | シーンファイルに進入 | `id` / `title` / `background` / `bgm` |
| `section_enter` | セクション（H2）に進入 | `heading` / `anchor` / `line` |
| `narration` | ナレーション | `text` / `line` |
| `dialogue` | セリフ | `speaker` / `text` / `line` |
| `choice` | 選択肢ブロックに到達 | `options`（`label` / `target`）/ `selected`（選んだ番号。停止時は null） |
| `jump` | ジャンプ段落 | `label` / `target` / `line` |
| `ending` | エンディング到達 | `id` / `line` |

すべてのステップが `file` を持ち、`scene_enter` 以外は入力 Markdown の行番号 `line` を持つ。

### end の終わり方（`reason`）

| reason | 意味 | status / 終了コード |
|---|---|---|
| `ending` | `<!-- ending: id -->` に到達（`id` 付き） | ok / 0 |
| `end_of_file` | ファイル末尾に到達（暗黙の終了） | ok / 0 |
| `awaiting_choice` | 選択番号が尽きて入力待ちで停止 | ok / 0 |
| `invalid_choice` | 選択番号が範囲外（`given` / `available` 付き） | error / 1 |
| `truncated` | ステップ数が上限 `max_steps`（10000）に達した。ジャンプのループの可能性が高い | error / 1 |

### エラー時も形式は崩れない

実行前検査が error のとき、および入力パスが読めないときも同じ形式で出力される（`trace` が `null` になり、`diagnostics` に内容が残る）:

```json
{
  "status": "error",
  "file": "scenario/spring_001.md",
  "files": ["scenario/spring_001.md"],
  "error_count": 1,
  "warning_count": 0,
  "diagnostics": [
    {
      "rule_id": "broken-link",
      "severity": "error",
      "message": "このファイルに「run-togather」という見出し（##）はありません。…",
      "file": "scenario/spring_001.md",
      "span": { "line": 19 },
      "related_spans": [],
      "suggestion": "[一緒に走る](#run-together)"
    }
  ],
  "trace": null
}
```

## ライブラリ API

```rust
use tsumugai::scenario::{trace_path, TraceOptions, render_trace_human, render_trace_json};

let result = trace_path(Path::new("scenario.md"), &TraceOptions::default());
// result.check  : 実行前検査の結果（CheckResult）
// result.trace  : 実行記録（check が error のときは None）
// result.has_errors() : 終了コードを 1 にすべきか
```

`trace_path` は infallible（panic / Err にしない）。入出力エラーも `io-error` の Diagnostic として `result.check` に含まれる。

## 旧記法の trace からの変更（v0 → v1）

- 旧 `RuntimeTrace`（pc / step 単位、`[SAY]` 等の旧記法）は廃止し、v1 記法のブロック単位のステップ列になった
- v1 記法には変数・フラグがないため、`state_diff` / `var_changes` は存在しない（v2 で変数導入時に再検討）
- 選択肢の自動選択（常に先頭を選ぶ）は行わず、`--choices` で明示するか入力待ちで停止する
