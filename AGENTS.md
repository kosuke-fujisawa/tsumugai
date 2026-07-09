# AGENTS.md

この文書は、Codex などの LLM エージェントが `tsumugai` を開発するときに参照する行動指針です。

tsumugai の開発方針・責務分離・コード設計の制約・品質ゲートなどの正本は `CLAUDE.md` です。AGENTS.md との内容の重複によるドキュメント間の食い違いを避けるため、本ドキュメントでは共通方針を `CLAUDE.md` への参照に寄せ、Codex 固有の運用（TODO 管理）のみをここに記載します。

作業を始める前に、必ず `CLAUDE.md` を読んでください。特に次の節は毎回の作業に直結します。

- 最優先方針
- 現在の責務分離（`parse` / `check` / `trace` / `routes` / `fmt` / `report` の構成）
- コード設計の制約
- 変更時に必要なレビュー材料
- Diagnostic 方針
- CLI / JSON / ログ方針
- テストと品質ゲート
- ドキュメント更新
- 最終報告の方針

> **注記（2026-07-08）**
> TypeScript / Svelte / Vite への全面移行（[epic #99](https://github.com/kosuke-fujisawa/tsumugai/issues/99)）は不採用が確定しました。Rust 向けの設計制約・責務分離は、引き続き `src/` 以下の現行実装にそのまま適用されます。`compile --target web` コマンド（[#128](https://github.com/kosuke-fujisawa/tsumugai/issues/128)）を追加済みで、arikoi 側の Svelte 製 player が読み込む StoryBundle JSON を出力できます。

## TODO 管理方針

作業候補はリポジトリ直下の `TODO.md` で管理します（`../claude_code/TODO.md` ではありません）。

ユーザーは `TODO.md` の `Inbox` に自由にタスクを追加してかまいません。Codex は作業開始時に `TODO.md` を確認し、必要に応じてタスクへ ID を付け、`Backlog` / `Doing` / `Done` / `Dropped` を整理します。

タスクを完了した場合は、`Done` に移し、完了日、変更内容、確認したコマンドやレビュー材料を簡潔に残してください。タスクを削除せず、対応しない判断をした場合は `Dropped` に理由を残してください。
