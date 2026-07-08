# CONCEPT - tsumugai

## tsumugai とは何か

tsumugai は、Markdown ベースで記述されたノベルゲームシナリオを、

> 実行前に意味を確定し、検証し、再利用可能にする

ための、静的検査・経路検証 CLI です。

単なるノベルゲームエンジンではありません。対話的に進行させる実行系（ランタイム）は持ちません。

tsumugai は、シナリオを書く、理解する、検証する、他環境へ受け渡す、という一連の行為を「意味が追跡可能な構造」として扱います。

## 背景

従来のノベルゲーム制作では、多くの場合、独自 DSL、実行時解釈、エンジン依存、人手によるテストプレイによってシナリオが管理されてきました。

この方法は小規模では機能します。しかし、分岐が増えるにつれて、次の問題を抱えやすくなります。

- 分岐が把握しづらくなる
- 到達不能ルートが埋もれる
- 実行しないと意味が分からない
- 他エンジンへの移植が難しい
- LLM が生成したシナリオの整合性を確認しづらい
- シナリオがブラックボックス化する

tsumugai は、この問題に対して「シナリオを意味として先に固定し、実行せずに検証する」という方向からアプローチします。

## 基本思想

### 1. 実行前に意味を確定する

tsumugai は Markdown を逐次解釈する実行エンジンではありません。

Markdown の解釈（parse）で意味を確定し、そこから先の検査・経路探索・変換はすべて確定済みの構造（[`Scene`](../src/scenario/mod.rs)）に対して行います。

```text
Markdown
  -> parse_str / parse_file
  -> Scene { lead, sections } + Diagnostic
  -> check / trace / routes / fmt / compile --target web
```

parse はエラーで中断せず、解釈できた範囲の `Scene` と検出したすべての `Diagnostic` を常に両方返します（SPEC 6.1）。

### 2. parse と検査・変換を分離する

parse は「読む」責務だけを持ち、実行状態を持ちません。

意味論検査（check）、経路の再現・探索（trace / routes）、整形（fmt）、StoryBundle 生成（compile）は、それぞれ独立したモジュールとして parse の結果を利用します。同じ `Scene` 構造を、静的検査にも経路探索にも変換にも再利用できます。

### 3. 表示と意味を分離する

tsumugai は、描画、音声再生、UI、アニメーションを担当しません。

代わりに、検証結果（`Diagnostic`）や経路情報（`TraceResult` / `RoutesResult`）、外部フロントエンド向けの `StoryBundle` JSON（`compile --target web`）を出力します。

```text
tsumugai: 何が起こりうるか・シナリオとして正しいかを決める
ホスト側（例: arikoi の Svelte 製 player）: どう見せるかを決める
```

tsumugai は npm ライブラリとして配布せず、CLI サブプロセス + JSON（`--format json` は stdout、`compile --output` はファイル）でホスト側と疎結合します（連携方法の詳細は [ARCHITECTURE.md](ARCHITECTURE.md) 8章）。

### 4. シナリオを読める構造にする

tsumugai の価値は、単に複雑な分岐を扱えることではありません。

構造、経路、実行順序を、人間が追跡可能な形にすることを重視します。

そのため、`trace`（1経路の再現）、`routes`（全分岐探索）、構造化された `Diagnostic` を中核機能として扱います。

### 5. 決定的な出力にする

同一の Markdown 入力からは、常に同じ結果を返すことを目指します。

`check` / `trace` / `routes` / `fmt` / `compile` のいずれも、乱数や実行時刻に依存する出力を持ちません。`compile` が生成する `storyBuildId` も、bundle の内容から決定的に計算した値です（[API.md](API.md) 6.5章）。これにより CI での Golden JSON 比較や、LLM への再現手順の提示がしやすくなります。

現行の Markdown 記法（SPEC.md）には変数・条件分岐に相当する構文はまだありません。追加する場合は、決定性を保ったまま「状態」をどう明示するかの設計が別途必要です。

## 分岐に対する考え方

tsumugai は「分岐が多いこと」自体を価値とは見なしません。

重要なのは、差分に意味があることです。

そのため、無意味な分岐爆発やコンプリート不能な列挙ではなく、到達可能性が保証された、追跡しやすい分岐構造を目指します（`routes` の到達可能性検査、`check` の `unreachable-section` 検出はこの考え方に基づきます）。

## 検証と修正支援

tsumugai は問題を検出するだけでなく、可能な範囲で「どこを、なぜ、どう直せばよいか」を返します。

`Diagnostic`（[`src/scenario/diagnostic.rs`](../src/scenario/diagnostic.rs)）は、次の情報を持ちます。

- `rule_id`
- `severity`
- `message`
- `span`
- `related_spans`
- `suggestion`

これは人間の修正作業だけでなく、LLM にシナリオを再修正させるための構造化フィードバックとしても利用できます。

## 全分岐探索とエンディング検証

tsumugai は、ユーザー操作を伴わずにシナリオを探索する `routes` コマンドを実装済みです。

`routes` は、次の検証を行います。

- 全分岐の探索（選択肢のすべての項目を辿る DFS）
- 到達可能なエンディングの確認（`reached_endings`）
- 到達不能なエンディング・シーンの検出（`unreached_endings` / `unreachable_scenes`）
- 循環の検出（`circular-route`、error 扱い）
- 経路数・深度の上限超過の検出（`route-limit-exceeded` / `route-max-depth-exceeded`、warning 扱い）

エンディングは単なるファイル末尾ではなく、`<!-- ending: id -->` という明示的な意味単位として扱います（SPEC 4.5章）。

条件によって選べない選択肢の検出は、現行記法に条件分岐がないため未実装です。

## LLM との関係

tsumugai は LLM 時代を前提とします。

そのため、Markdown ベース、人間可読、構造が単純、検証可能、ログ可能であることを重視します。

目的は LLM にシナリオを書かせることだけではありません。生成物を検証し、安全に扱えるようにすることです。

LLM は分岐、ラベル、エンディング整合性を崩しやすいので、tsumugai は生成物を検証する決定的な CLI として機能します。

## 設計上の優先順位

tsumugai は、形式的な Clean Architecture や深いオブジェクト階層そのものを目的にしません。

必要な境界は保ちつつ、次の性質を優先します。

- 明示的な `State`（`Scene` / `CheckResult` / `TraceResult` / `RoutesResult` / `CompileResult` などの具体的な型）
- enum 中心（`Block` / `BundleStep` / `RouteEnd` など）
- 純関数に近い実装（parse / check / trace / routes / compile はいずれも入出力が明確な関数）
- 単純な責務分離（`src/scenario/` 配下の各モジュールが 1 つの責務を持つ）
- 決定的な実行
- 小さく安定した外部契約（[API.md](API.md) 8章の互換性ポリシー）

## tsumugai の外部境界

tsumugai は npm ライブラリや他言語バインディングとして配布しません。

外部フロントエンド（arikoi の Svelte 製 player 等）とは、CLI サブプロセス + JSON（stdout / `compile --output`）で疎結合します。`check` / `trace` / `routes` / `fmt` の `--format json` は stdout に JSON を返すだけで、ファイルは書き出しません。ファイル出力を行うのは `compile --output` （StoryBundle JSON）と `fmt --write` （整形済み Markdown）だけです。

```text
シナリオ Markdown（別リポジトリ）
  -> tsumugai check / trace / routes --format json（CLI サブプロセス、stdout に JSON）
  -> tsumugai compile --target web --output <path>（CLI サブプロセス、StoryBundle JSON をファイル出力）
  -> ホスト側（表示・入力・セーブロード・アセット配信を担当）
```

将来、他のホスト（CUI・別エンジン等）向けの出力形式が必要になった場合も、この「CLI + JSON」という境界を保つ方針です。

## 将来像

tsumugai は、単なる個人用ツールではなく、ノベルゲーム制作における意味の共通基盤を目指します。

具体的には、複数ホストへの対応、既存 DSL からの移行、LLM 生成物検証、長編シナリオ管理を視野に入れます。ただし、現時点で具体的な実装計画があるのは `compile --target web`（arikoi 向け、実装済み）のみで、他のホスト向け出力は構想段階です。

## 一言で言うと

tsumugai は、ノベルゲームシナリオを「実行されるテキスト」ではなく、「検証可能な意味構造」として扱うための静的検査・経路検証 CLI です。
