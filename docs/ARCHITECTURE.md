# ARCHITECTURE — tsumugai

tsumugai は単一の Rust crate（`src/`）で構成される、Markdown シナリオの静的検査・経路検証・整形・StoryBundle 生成を行う CLI です。対話的に進行させる実行エンジン（ランタイム）は持ちません。

---

## 1. 全体像（責務の境界）

```text
Markdown（v1 記法、SPEC.md）
  -> scenario::parse_str / parse_file
  -> Scene { lead, sections: Vec<Section>, ... }（+ Diagnostic 列）
  -> scenario::check_path / trace_path / routes_path / fmt_path / compile_path
  -> CheckResult / TraceResult / RoutesResult / FmtResult / CompileResult
  -> scenario::render_human / render_json / render_sarif / render_trace_* / render_routes_* / render_fmt_* / StoryBundle JSON
```

- **parse**（`src/scenario/parse.rs`）: 1 ファイル = 1 [`Scene`] に変換する。実行状態を持たず、エラーで中断しない
- **check**（`check.rs`）: リンク切れ・話者・到達可能性などプロジェクト横断の意味論検査
- **trace / routes**（`trace.rs` / `routes.rs`）: SPEC 5章の実行モデルに基づく経路再現・全分岐探索
- **compile**（`compile.rs`）: check 相当の検査を通過したプロジェクトを StoryBundle JSON に変換する（`--target web`）
- **fmt**（`fmt.rs`）: よくある書き方を決定的ルールで v1 記法へ整形する（SPEC 7章）
- **report**（`report.rs`）: 各結果の human / JSON / SARIF 出力

表示、音声再生、UI、アセットロードは tsumugai の責務ではありません。旧 v0 記法向けの `parser` / `analyzer` / `runtime` / `player` / `types` モジュールは撤去済みです（#93）。

---

## 2. ディレクトリ構成

```text
tsumugai/
├─ src/
│  ├─ main.rs          # CLI エントリーポイント（引数パースとコマンド分岐のみ）
│  ├─ lib.rs            # scenario モジュールの再公開
│  └─ scenario/
│     ├─ mod.rs          # Scene / Block / Section 等の IR 定義、公開 API の再エクスポート
│     ├─ parse.rs        # Markdown → Scene + Diagnostic
│     ├─ project.rs      # 複数ファイルの読み込み・リンク解決（check / routes / compile が共有）
│     ├─ characters.rs   # characters.yaml の探索・読み込み
│     ├─ check.rs        # プロジェクト横断の意味論検査
│     ├─ exec.rs         # trace / routes が共有する実行位置（Cursor）とナビゲーション
│     ├─ trace.rs        # 1 経路の実行再現（--choices）
│     ├─ routes.rs       # 全分岐探索（到達可能性・循環・エンディング到達検証）
│     ├─ compile.rs      # StoryBundle JSON 生成（--target web）
│     ├─ fmt.rs          # 推測整形
│     ├─ diagnostic.rs   # 構造化 Diagnostic（rule_id / severity / message / span / suggestion）
│     └─ report.rs       # human / JSON / SARIF 出力
├─ tests/
│  ├─ check_test.rs / trace_test.rs / routes_test.rs / compile_test.rs  # 統合テスト
│  └─ fixtures/                                                        # ケースごとのミニプロジェクト・Golden JSON
├─ examples/
│  ├─ spring/            # 全ブロック種別を含む仕様網羅サンプル
│  └─ fmt/               # fmt の整形前後サンプル
├─ docs/{CONCEPT,ARCHITECTURE,API,CLI_OUTPUT,TRACE,ROUTES,DEVELOPMENT_WORKFLOW,REVIEW_GUIDE}.md
└─ .github/workflows/ci.yml
```

---

## 3. データフロー（詳細）

1. **Parse**: Markdown 1 ファイルを `Scene { lead, sections: Vec<Section> }` + `Diagnostic` 列に変換する。エラーで中断しない（SPEC 6.1）
2. **Load**: `check` / `routes` / `compile` は `project.rs` が提供する共通の読み込み規則（ディレクトリ走査、またはリンクで辿れる `.md` の閉包）で複数ファイルをロードする
3. **Check**: プロジェクト横断の意味論検査（リンク解決、アセット実在、話者宣言、到達可能性）を行う。他コマンドはこの検査を通過した場合のみ先へ進む
4. **Trace / Routes**: check 通過後、実行位置（`Cursor { scene, seg, block }`）を SPEC 5章の規則で進める。`trace` は `--choices` で指定した 1 経路を、`routes` はすべての分岐を DFS で網羅する
5. **Compile**: check 通過後、読み込んだ `Scene` 群を `StoryBundle`（`scenes[].steps[]` + `assets[]`）にコンパイルする。jump / choice の飛び先はソース表記ではなく `{ sceneId, stepIndex }` に解決済みで持たせる
6. **Fmt**: よくある書き方を決定的ルールで v1 記法に整形する。確信が持てない箇所は変換せず `Diagnostic` に積む（SPEC 7章）
7. **Report**: 各結果を human / JSON / SARIF 形式に変換する（`report.rs`）

---

## 4. 決定性

同一の Markdown 入力からは、常に同じ結果を返します。乱数や実行時刻に依存する出力はありません。`compile` が生成する `storyBuildId` も、bundle の内容から決定的に計算した値です（FNV-1a、[API.md](API.md) 6.5章）。これにより CI での Golden JSON 比較や、LLM への再現手順の提示がしやすくなります。

---

## 5. テスト戦略

- **統合テスト**（`tests/*.rs`）: ライブラリ関数（`check_path` 等）を直接呼び出し、`tests/fixtures/` 配下のミニプロジェクトで各ルールを 1 対 1 で検証する
- **Golden JSON**（`tests/fixtures/compile/golden/*.json` 等）: 意図しない出力変化を検出する
- **CLI プロセステスト**（`compile_test.rs`）: `CARGO_BIN_EXE_tsumugai` で実バイナリを起動し、exit code とファイル生成有無を確認する
- **doctest**: `src/scenario/mod.rs` / `src/lib.rs` のコード例を `cargo test` で検証する

CLI 引数パース自体（`main.rs`）は薄く保ち、原則としてテストしない。ロジックは `scenario` モジュールの関数に置き、そちらをテスト対象にする。

---

## 6. CI（`.github/workflows/ci.yml`）

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `compile` コマンドのスモークテスト（`examples/spring` から StoryBundle JSON を生成できることを確認）

---

## 7. 互換性ポリシー

- **後方互換の変更（許容）**: 新しい `rule_id` の追加、構造体への任意（Option）フィールド追加
- **破壊的変更（要調整）**: 既存 `rule_id` の意味変更・削除、JSON 出力の必須フィールド変更

詳細は [API.md](API.md) 8章を参照。

---

## 8. tsumugai の外部境界

tsumugai は npm ライブラリや他言語バインディングとして配布しません。外部フロントエンド（arikoi の Svelte 製 player 等）とは、CLI サブプロセス + JSON ファイルで疎結合します。

```text
arikoi/scenarios/*.md（別リポジトリ）
  -> tsumugai check / trace / routes / compile --target web（CLI サブプロセス）
  -> Diagnostic JSON / StoryBundle JSON
  -> arikoi（表示・入力・セーブロード・Web 配布を担当）
```

---

## 9. よくある判断

- 演出 DSL（フェード等）を tsumugai に足す？ → **No**（ホスト側の責務）
- アセットの実描画・再生を tsumugai で？ → **No**（存在チェックのみ行う）
- 変数・条件分岐を足す？ → 現行 v1 記法にはまだない。追加する場合は SPEC.md の改訂と、`compile.rs` の `BundleStep` への `set_variable` 相当の追加が必要になる
- 仕様を変える？ → 挙動を変える変更では、入力例・出力例・Diagnostic 例・テストケースのいずれかを追加・更新し、Rust コードを読まなくても変更点が分かるようにする（CLAUDE.md 参照）
