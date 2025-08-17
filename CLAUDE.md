# CLAUDE.md — tsumugai 開発ガイドライン

このファイルは、Claude Code（claude.ai/code）が **tsumugai** リポジトリで作業する際のガードレールです。  
目標は「**Markdown 台本 → 決定的な StepResult(JSON)**」のコア純度を保ちつつ、最小のサンプル/参照GUIで動作確認を回せる状態を維持すること。

---

## 0. 前提（責務の境界）

- **Core（このリポの中心）**  
  - 入力：`.md`（UTF-8, tsumugai 記法）  
  - 出力：`StepResult { next: NextAction, directives: Vec<Directive> }`（**決定的**）  
  - 非責務：描画/再生、アセット存在確認、演出（フェード等）、LLMプロンプト生成

- **Samples / Hosts（限定的に同梱してよい）**  
  - `examples/cui_runner/`：CUI 実行サンプル（Enter進行・分岐入力）  
  - `hosts/tauri_svelte/`：参照GUI（最小の上映装置。画像/BGMの受け渡しのみ）  
  - 目的：**契約確認と導線提示のみ**。機能は最小。

---

## 1. 設計優先順位

1) **TDD / テストファースト**  
- 単位は「1行の解釈→StepResult」。失敗テストから書く。  
- 種別：ユニット（パーサ/分岐/待機/未定義ラベル）、分岐DFS、ゴールデン比較（`--dump`）、Lint（未定義ラベル・未解決アセット）。

2) **DDD（ドメイン駆動設計）**  
- ドメイン＝台本構文と逐次解釈。演出はドメイン外。  
- Domain は I/O を持たない。Application が Engine の組み立て役。

3) **クリーンアーキテクチャ（依存方向の固定）**  
Domain ← Application ← (Infrastructure)
↑
examples/, hosts/（外側）

- 公開契約は `StepResult / Directive / NextAction`。破壊変更禁止。

4) **DRY / Tidy First**  
- 重複は3回目で抽象化。  
- 「整える→直す」。構造変更と挙動変更は別コミット/別PR。

---

## 2. Claude が行ってよいタスク

- `crates/core/src/**` の実装・テスト・ドキュメント更新  
- パーサ/エンジン/IR/エラーハンドリングの改善  
- `examples/` の追加・更新（`cui_runner/` を含む）  
- `hosts/tauri_svelte/` の最小実装・維持（重依存は追加しない）  
- `examples/dump.rs` と **ゴールデン比較**テストの整備  
- `docs/ARCHITECTURE.md`, `docs/API.md`, `schemas/stepresult.schema.json` の更新

### 行ってはいけない
- 公開契約（`StepResult/Directive/NextAction`）の**破壊変更**  
- `hosts/tauri_svelte/` に重い依存（動画コーデック等）を追加  
- `assets/` に大容量アセット追加（CC0の最小素材のみ許可）  
- CI を GUI ビルドで重くする変更（GUIは別ジョブ/任意）

---

## 3. Confirm-First Protocol（実装前の確認）

**以下が埋まるまで実装しない。質問のみ返す。**

【Clarify Request】

目的（観測値）: 例) dumpの決定性をCIで100%検知

受け入れ条件(3点以上): ①ログ/キー ②可視確認 ③テスト名

変更スコープ: 対象/非対象/壊してよい既存挙動

ルーブリック根拠: CLAUDE.md §(章) / 既存 ADR への言及

**Proceed OK（実装開始合図）**
【Proceed OK – 要件充足】

目的数値:

受け入れ条件:

スコープ:

ルーブリック根拠:
→ 実装に進みます（次メッセージで差分とテスト提示）

---

## 4. レビュー運用（Debate-First）

- PR に `needs-debate` ラベルが付いている間、**コードを変更しない**。  
- 指摘には JSON で応答（立場/根拠/影響/代替案/合意条件/質問/自己評価）。  
- 合意後に `approved-to-apply` を付け、初めて修正コミットを作成。

（JSONスキーマは簡略版でOK。長文の弁論は不要、**代替案の列挙**を重視。）

---

## 5. Issue テンプレ

- **[目的]** 観測可能な成果（例：`tests/golden/*.json` と `--dump` が完全一致する）  
- **[範囲]** 対象/非対象/壊してよい既存挙動  
- **[受け入れ条件]** 少なくとも3点（ログ/可視/テスト名）  
- **[ルーブリック根拠]** CLAUDE.md §X.Y / ADR 参照

---

## 6. CI ガード

- Core（必須）: `fmt` / `clippy -D warnings` / `cargo test` / **ゴールデン比較**  
- Hosts（任意・別ジョブ）: lint + 最小起動（失敗しても Core を止めない設定可）

---

## 7. サンプル/参照GUIの方針

- **目的**：Core 契約の確認と導線。演出過多にしない。  
- **受け入れ条件**：  
  - CUI：Enter 進行／分岐入力／未解決アセットは警告表示で継続  
  - GUI：`StepResult` を素直に反映（画像/BGMの受け渡し・分岐ボタン・待機）  
- **禁止**：Core の API を都合で曲げない。辞書/演出は Host 側に閉じる。

---

## 8. 変更管理・コミット規律

- 構造変更（rename/move/整形）と挙動変更（ロジック/意味）は**同一コミットに含めない**。  
- 小さく頻繁に、メッセージは `feat(parser): WAIT "1.5s" support` のように機能＋要点で。  
- 失敗したら即Revert。テストは緑を維持。

---

## 9. エラーハンドリング

- 例外は **行/列番号 + 最良候補** を含める（修正ガイド）。  
- 未定義ラベル/未解決アセットは **警告** として継続（Coreが止まらない）。

---

## 10. 便利リンク（索引）

- Core入口：`crates/core/src/lib.rs`  
- パーサ：`crates/core/src/parse/**`  
- 実行器：`crates/core/src/engine/**`  
- 公開型：`crates/core/src/public.rs`（`StepResult/Directive/NextAction`）  
- 例：`examples/strange_encounter.md`  
- CUI：`examples/cui_runner/main.rs`  
- ダンプ：`examples/dump.rs`  
- スキーマ：`schemas/stepresult.schema.json`  
- CI：`.github/workflows/ci.yml`

---

## 11. 完了報告

**完全完了（継続タスクなし）の合図**は次のテンプレで報告し、決め台詞は使わなくてよい。

```markdown
## 実行完了
### 変更内容
- …

### 次のステップ
- （あれば記載、なければ「なし」）
```