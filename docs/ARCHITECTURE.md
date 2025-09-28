# ARCHITECTURE — tsumugai

tsumugaiは、目的の異なる2つのアーキテクチャを提供します。

- **簡易アーキテクチャ (Facade)**: 手軽さと即時性を重視したAPI。
- **コアアーキテクチャ (Layered)**: 拡張性と保守性を重視した階層化API。

---

## 1. アーキテクチャの選択ガイド

| 観点 | 簡易アーキテクチャ (Facade) | コアアーキテクチャ (Layered) |
|:---|:---|:---|
| **主な目的** | 手軽な利用、迅速なプロトタイピング | 拡張性、保守性、長期的な開発 |
| **設計** | パーサーとランタイムを直接組み合わせたシンプルな構成 | DDDに基づく3層のクリーンアーキテクチャ |
| **API** | `facade::Facade` 構造体 | `application::engine::StoryEngine` |
| **状態管理** | 呼び出し側で `State` オブジェクトを保持 | `StoryEngine` が内部で状態をカプセル化 |
| **データフロー** | `Markdown -> AST -> Runtime -> Output` | `Markdown -> IR -> Engine -> StepResult` |
| **こんな時に** | 個人開発、小規模ゲーム、ツールへの組み込み | チーム開発、大規模ゲーム、複雑な独自仕様 |

---

## 2. 簡易アーキテクチャ (Facade) 詳細

`facade`モジュールを介して提供される、シンプルな構成です。

- **`parser`**: MarkdownテキストをAST (Abstract Syntax Tree) に変換します。
- **`runtime`**: ASTと現在の状態(`State`)を受け取り、次の状態と`Output`（描画すべき内容）を返します。
- **`types`**: `AST`, `State`, `Output`, `Event`など、簡易アーキテクチャで使われるデータ構造を定義します。
- **`facade`**: これらを統合し、`Facade::new(markdown)` や `Facade::next(&mut state)` のような簡単なインターフェースを提供します。

## 3. コアアーキテクチャ (Layered) 詳細

# ARCHITECTURE — tsumugai

> 目的：**Markdown の台本**を逐次解釈し、**決定的な StepResult(JSON)** を返す“コア”を提供する。  
> 表示・音・演出はホスト（例：Tauri+Svelte）側の責務。

---

## 1. 全体像（責務の境界）

```markdown
.md（tsumugai 記法）
│ parse
▼
[Core/Rust] ── Engine.step()/choose() ──> StepResult(JSON)
▲
│（契約：Directive/NextAction を固定）
[Host] Tauri/Svelte/Bevy/CLI など
```

- **Core**（このリポの中心）
  - 入力：UTF-8 Markdown（`[COMMAND key=value]` 形式）
  - 出力：`StepResult { next, directives[] }`（**決定的**）
  - 非責務：描画・再生、アセット存在確認、演出DSL、LLMプロンプト生成

- **Host / Samples**（限定同梱）
  - `examples/cui_runner/`（CUI の最小上映装置）
  - `hosts/tauri_svelte/`（参照 GUI。画像/BGMの受け渡しのみ）
  - 目的は **契約確認** と **導線提示**。機能は最小限に留める。

---

## 2. ディレクトリ構成（最小）

```markdown
tsumugai/
├─ crates/core/
│ ├─ src/
│ │ ├─ lib.rs # 公開API
│ │ ├─ domain/ # 台本構文・状態遷移（純粋ロジック）
│ │ ├─ application/ # Engine（ユースケース編成）
│ │ ├─ parse/ # Markdown → IR
│ │ └─ engine/ # StepResult/Directive の生成
│ └─ tests/ # ユニット／DFS／ゴールデン
├─ examples/
│ ├─ strange_encounter.md
│ ├─ dump.rs # .md → JSON ダンプ
│ └─ cui_runner/
│ └─ main.rs
├─ hosts/tauri_svelte/ # 参照GUI（任意・最小）
├─ docs/{ARCHITECTURE.md, API.md}
├─ schemas/stepresult.schema.json
└─ .github/workflows/ci.yml
```

---

## 3. 設計優先順位（抜粋）

- **TDD / テストファースト**：赤→緑→リファクタ。DFS で分岐網羅、`--dump` で決定性を比較。
- **DDD**：ドメイン＝台本構文と逐次解釈。演出はドメイン外。Domain に I/O を入れない。
- **クリーンアーキ**：依存は内向きのみ。`StepResult/Directive/NextAction` を契約として固定。
- **DRY / Tidy First**：重複は3回目で抽象化。構造変更と挙動変更は別コミット。

---

## 4. データフロー（詳細）

1. **Parse**：`.md` をトークン化 → 構文規則で IR（中間表現）へ  
2. **Plan**：ジャンプ先・ラベル索引を構築（未定義は警告）  
3. **Run**：`Engine.step()` が現在位置を 1 ステップ進め、`StepResult` を生成  
4. **Branch**：`NextAction::WaitBranch` なら `Engine.choose(index)` を待つ  
5. **Halt**：終端で `NextAction::Halt`

> **決定性**：同一入力 `.md` は同一 `StepResult` 列を生む。CI でゴールデン比較。

---

## 5. 互換性ポリシー

- **破壊変更禁止**：既存 `Directive` の意味変更／フィールド削除／`NextAction` 既存値の変更  
- **後方互換で許容**：新 `Directive` 追加、可オプショナルなフィールド追加  
- **変更時手順**：**先に** `docs/API.md` と `schemas/*.json` を更新 → コード → ゴールデン更新

---

## 6. テスト戦略

- **ユニット**：パーサ（正常/異常）、待機、分岐、未定義ラベル  
- **DFS網羅**：深さ上限・ループ検出。各ルートで `StepResult` 整合を検査  
- **ゴールデン**：`examples/dump.rs` の JSON と `tests/golden/*.json` を厳密比較  
- **Lint**：未定義ラベル／未解決アセット／重複ラベル等を検出（失敗で赤）

---

## 7. CI（最低条件）

- `cargo fmt --check` / `cargo clippy -D warnings` / `cargo test`  
- `dump` 出力とゴールデン比較  
- Hosts は別ジョブ／任意。Core を重くしない

---

## 8. 例外とログ

- エラーには **行/列番号 + 修正候補** を含める  
- 未解決アセットや未定義ラベル：**警告**で継続（Core を止めない）

---

## 9. よくある判断

- フェード等の演出DSLを Core に足す？ → **No**（Host の辞書へ）  
- アセット存在確認を Core で？ → **No**（Host の責務）  
- 仕様を変える？ → **API.md/Schema を先に更新 → テスト赤 → 実装**
