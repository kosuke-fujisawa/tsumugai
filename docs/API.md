# API — tsumugai `scenario` モジュールの契約

この文書は、tsumugai の公開 API（`src/scenario/`。CLI の `check` / `trace` / `routes` / `fmt` はこの薄いラッパー）の契約を定義します。

tsumugai は独立したシナリオ制作 CLI であり、ゲームエンジンにライブラリとして組み込んで対話ループを回す設計（旧 Core ⇄ Host 契約）はありません。ホスト側（arikoi 等）とは CLI 境界（ファイル入出力 + 終了コード + JSON）で接続します。データフローは [SPEC.md](../SPEC.md) 1章を参照してください。

---

## 1. シーンモデル

```rust
pub struct Scene {
    pub path: PathBuf,
    pub id: Option<String>,
    pub title: Option<String>,
    pub background: Option<String>,
    pub bgm: Option<String>,
    pub lead: Vec<Block>,       // front matter 直後〜最初の H2 まで
    pub sections: Vec<Section>, // H2 で区切られたセクション
}

pub struct Section {
    pub heading: String,
    pub anchor: String, // slugify(heading)
    pub line: usize,
    pub blocks: Vec<Block>,
}

pub enum Block {
    Narration { text: String, line: usize },
    Dialogue { speaker: String, text: String, line: usize },
    Choices { items: Vec<ChoiceItem>, line: usize },
    Jump { label: String, target: LinkTarget, line: usize },
    Ending { id: String, line: usize },
}
```

`parse_str(source, path)` / `parse_file(path)` が [`Parsed`]（`scene` + `diagnostics`）を返します。エラーで中断せず、解釈できた範囲の `Scene` と検出したすべての `Diagnostic` を常に両方返します（SPEC 6.1）。

---

## 2. Diagnostic（構造化エラー・警告）

```rust
pub struct Diagnostic {
    pub rule_id: &'static str,
    pub severity: Severity,      // Error | Warning
    pub message: String,
    pub file: PathBuf,
    pub span: Option<Span>,          // { line: usize }
    pub related_spans: Vec<Span>,
    pub suggestion: Option<String>,  // 機械的に適用できる書き換え例
}
```

ルール一覧は [SPEC.md 6章](../SPEC.md) と [DIAGNOSTIC.md](DIAGNOSTIC.md) が正です。

---

## 3. check（静的検査）

```rust
let result = scenario::check_path(path, &CheckOptions::default());
// result: CheckResult { files: Vec<PathBuf>, diagnostics: Vec<Diagnostic> }
result.has_errors(); // bool
```

入出力エラーも `io-error` の Diagnostic にして返す（infallible）。詳細と JSON / SARIF スキーマは [CLI_OUTPUT.md](CLI_OUTPUT.md)。

---

## 4. trace（経路の再現）

```rust
let result = scenario::trace_path(path, &TraceOptions { choices: vec![1, 3], ..Default::default() });
// result: TraceResult { file, check: CheckResult, trace: Option<Trace> }
```

実行前に check と同じ検査を行い、error があれば `trace` は `None` になる（SPEC 6.1）。詳細は [TRACE.md](TRACE.md)。

---

## 5. routes（全分岐探索）

```rust
let result = scenario::routes_path(path, &RoutesOptions::default());
// result: RoutesResult { file, check: CheckResult, report: Option<RoutesReport> }
```

詳細は [ROUTES.md](ROUTES.md)。

---

## 6. fmt（推測整形）

```rust
let result = scenario::fmt_path(path);
// result: FmtResult { path, original, formatted, changes: Vec<FmtChange>, diagnostics }
```

変換はすべて決定的なルールベース。黙って書き換えず、変更点は `changes` に 1 件ずつ記録する。確信が持てない箇所は変換せず `diagnostics` に積む（SPEC 7章）。実例は [examples/fmt/README.md](../examples/fmt/README.md)。

---

## 7. JSON 出力

`render_json` / `render_trace_json` / `render_routes_json` / `render_fmt_json` / `render_sarif` が機械向け出力を生成する。スキーマは [CLI_OUTPUT.md](CLI_OUTPUT.md) が正。

---

## 8. 互換性の考え方

- **後方互換の変更（許容）**：新しい `rule_id` の追加、構造体への任意（Option）フィールド追加
- **破壊的変更（要調整）**：既存 `rule_id` の意味変更・削除、JSON スキーマの必須フィールド変更

---

## 9. Roadmap

実装済み（`scenario` モジュール）:

- `tsumugai check --format json|sarif`
- `tsumugai trace --choices`（[TRACE.md](TRACE.md)）
- `tsumugai routes`（[ROUTES.md](ROUTES.md)）
- `tsumugai fmt --write`（SPEC 7章）

未実装:

- `tsumugai compile --target renpy` — Ren'Py 変換（#79）
