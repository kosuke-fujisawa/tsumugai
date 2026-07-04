# Routes — 全分岐探索（tsumugai routes）

関連: [SPEC.md 5.2](../SPEC.md)、issue #78

## 概要

`tsumugai trace` が `--choices` で指定した **1 経路だけ**を再現するのに対し、`tsumugai routes` は選択肢ブロックのすべての項目を辿ることで**プロジェクト内の全経路**を機械的に探索する。

想定する使い方:

- 書いたシナリオに、書いたつもりのないルートや、逆に書いたのに辿り着けないルートがないかを確認する
- リンクの張り忘れで到達できなくなった ending・シーンを見つける
- CI で「循環（無限ループ）が入っていないか」を機械的にチェックする

各経路は、選んだ選択肢の**選択番号列**として表現される。この番号列はそのまま `tsumugai trace --choices ...` に渡せば、その経路の詳細（通った行・発生したイベント）を再現できる。

## CLI

```bash
tsumugai routes scenario.md                    # 全分岐を探索して一覧表示
tsumugai routes scenario.md --format json      # 機械向け JSON（--json も同じ）
tsumugai routes scenario.md --no-assets        # 実行前検査のアセットチェックを省略
```

### 実行前検査（SPEC 6.1）

routes も trace と同じく、実行の前に **check とまったく同じ検査**を行う。

- error があれば探索せず、check と同じ形式で Diagnostic を報告して終了コード 1
- warning のみなら、warning を表示したうえで探索する

### 経路の終わり方（SPEC 5.2）

各経路は次のいずれかで終わる。

| 終わり方 | 意味 | 深刻度 |
|---|---|---|
| エンディング到達 | `<!-- ending: id -->` に到達した | — |
| ファイル末尾 | 暗黙の終了（SPEC 5章） | — |
| **循環** | 同一経路内で以前と同じ地点に再到達した。変数を持たない v1 では、同じ地点への再到達は必ず同じ挙動を繰り返すため、その時点で探索を打ち切る | **error**（`circular-route`） |
| 深度超過 | 1 経路のステップ数が上限に達した（循環検出をすり抜けた場合の保護） | warning（`route-max-depth-exceeded`） |

探索する経路の総数にも上限があり、上限に達すると残りの分岐の探索を打ち切る（`truncated: true`、warning `route-limit-exceeded`）。

## 人間向け出力の例

`examples/spring` を実行した場合（4 経路すべてが ending に到達する）:

```text
=== Routes: examples/spring/scenario/spring_001.md ===
Route 1: --choices 1 → エンディング「childhood_route」
Route 2: --choices 2 → エンディング「calm_route」
Route 3: --choices 3,1 → エンディング「sprint_route」
Route 4: --choices 3,2 → エンディング「calm_route」

発見した経路数: 4
到達可能 Ending: calm_route、childhood_route、sprint_route
```

各行の `--choices ...` はそのままコピーして使える。例えば経路 3 の詳細を見たければ:

```bash
tsumugai trace examples/spring/scenario/spring_001.md --choices 3,1
```

### 到達不能の検出（check との役割分担）

`check` の `unreachable-section` は「そのセクションを指すリンクが**プロジェクト内のどこにも存在しないか**」を静的に見る。一方 `routes` は、実際に開始シーンから辿れる経路だけを動的に数える。そのため、**リンク自体は存在するがそのリンクを含むセクション自体が到達不能**、という間接的な到達不能性は routes でしか検出できない。

例えば、あるセクション `orphan` がどこからもリンクされておらず（check が `unreachable-section` の warning を出す）、その `orphan` の中に `sibling.md` へのリンクがある場合、`sibling.md` は check の走査対象には含まれる（リンクが存在するため）が、routes の実際の探索では一度も訪れない:

```text
warning[unreachable-section]: セクション「orphan」はどこからも参照されず、...
  --> entry.md:15

warning[unreachable-ending]: エンディング「sibling_end」はプロジェクト内で宣言されていますが、どの経路からも到達できません。...
  --> entry.md

warning[unreachable-scene]: sibling.md はプロジェクトに読み込まれていますが、entry.md からのどの経路からも実行されません。...
  --> sibling.md

エラー: 0件  警告: 3件（2 ファイルを検査）

=== Routes: entry.md ===
Route 1: (選択なし) → エンディング「main_end」

発見した経路数: 1
到達可能 Ending: main_end
到達不能 Ending: sibling_end
到達不能シーン: sibling.md
```

### 循環の例

```text
error[circular-route]: 経路（選択肢を経由しない）は同じ地点に戻り続けるため、これ以上進んでも同じ結果を繰り返します（無限ループの可能性があります）。`tsumugai trace loop.md` で該当箇所を確認してください
  --> loop.md

エラー: 1件  警告: 0件（1 ファイルを検査）

=== Routes: loop.md ===
Route 1: (選択なし) → 循環

発見した経路数: 1
到達可能 Ending: (なし)
```

選択肢を 1 つも経由しない経路（純粋なジャンプの繰り返し）では `--choices` を省いた自然な文で案内する。選択肢を経由した循環では、`--choices` の番号列とその番号列を渡した `tsumugai trace` の再実行コマンドが案内に含まれる。

## JSON 出力（`--format json`）

check の JSON（[CLI_OUTPUT.md](CLI_OUTPUT.md)）の上位互換。`file` と `report` が加わり、`diagnostics` / `error_count` / `warning_count` は check の Diagnostic と routes 由来の Diagnostic（`circular-route` 等）を合算した値になる。

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
  "report": {
    "reached_endings": ["calm_route", "childhood_route", "sprint_route"],
    "routes": [
      { "choices": [1], "end": { "reason": "ending", "id": "childhood_route" } },
      { "choices": [2], "end": { "reason": "ending", "id": "calm_route" } },
      { "choices": [3, 1], "end": { "reason": "ending", "id": "sprint_route" } },
      { "choices": [3, 2], "end": { "reason": "ending", "id": "calm_route" } }
    ],
    "truncated": false,
    "unreachable_scenes": [],
    "unreached_endings": []
  }
}
```

### report.routes[].end の終わり方（`reason`）

| reason | 意味 | 付加情報 |
|---|---|---|
| `ending` | エンディングに到達 | `id` |
| `end_of_file` | ファイル末尾（暗黙の終了） | — |
| `circular` | 循環を検出（error） | — |
| `max_depth_exceeded` | ステップ数の上限に達した（warning） | `max_depth` |

### エラー時も形式は崩れない

実行前検査が error のとき、および入力パスが読めない・ディレクトリを指定したときも同じ形式で出力される（`report` が `null` になり、`diagnostics` に内容が残る）。trace と同じ規約。

## ライブラリ API

```rust
use tsumugai::scenario::{routes_path, RoutesOptions, render_routes_human, render_routes_json};

let result = routes_path(Path::new("scenario.md"), &RoutesOptions::default());
// result.check   : 実行前検査の結果（CheckResult）
// result.report  : 探索結果（check が error のときは None）
// result.has_errors() : 終了コードを 1 にすべきか（check エラー or 循環検出）
```

`RoutesOptions` は `max_routes` / `max_depth`（探索する経路数・1 経路あたりのステップ数の上限。既定はどちらも 1000）を持つ。CLI からは公開しておらず、既定値で十分な規模を想定している。

`routes_path` は infallible（panic / Err にしない）。入出力エラーも `io-error` の Diagnostic として `result.check` に含まれる。

## #68・#69 との関係

- **#68**（`step` の無限ループ）: routes の循環検出（同一経路内での位置の再訪問）により、無限ループはハングせず `circular-route` の error として報告される
- **#69**（条件付き選択肢がすべて非表示になり進行不能になる）: v1 記法には条件付き選択肢が存在しないため、選択肢ブロックが実行時に項目 0 件になることは構造的に起こらない（`check` の `invalid-choice-item` / `linkless-list` が、項目のないリストを選択肢ブロックとして解釈しない）。この意味で #69 の再現経路は v1 に存在しない
