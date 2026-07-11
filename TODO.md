# TODO

このファイルは、tsumugai の作業候補を人間と Codex が共有するための管理表です。

ユーザーは自由にタスクを追加・編集してかまいません。Codex は作業開始時と完了時にこのファイルを確認し、必要に応じて ID、状態、メモ、完了履歴を整理します。

## 運用ルール

- 新しいタスクは `Inbox` に箇条書きで追加するだけでよい。
- Codex は `Inbox` のタスクを確認し、必要なら `Backlog` に移して ID を付ける。
- Codex が作業するタスクは `Doing` に移す。
- 完了したタスクは `Done` に移し、完了日と確認内容を書く。
- やらないと決めたタスクは削除せず、`Dropped` に移して理由を書く。
- コードを読まなくても判断できるように、タスクには可能な範囲で入力例、期待出力、確認方法を書く。

## 状態

- `Inbox`: まだ整理していない思いつき、要望、違和感
- `Backlog`: 着手可能なタスク
- `Doing`: 現在 Codex が対応中のタスク
- `Done`: 完了済みのタスク
- `Dropped`: 対応しないことを明示したタスク

## ID ルール

Codex が整理するときに、次の形式で ID を付けます。

```text
T-0001
T-0002
T-0003
```

ID は再利用しません。タスクを分割した場合は、新しい ID を作り、元タスクのメモに分割先を書きます。

## タスクの書き方

最小形:

```markdown
- [ ] シナリオチェックで未定義ラベルをもっと分かりやすく出したい
```

整理後:

```markdown
### T-0001 未定義ラベル Diagnostic の改善

- 状態: Backlog
- 種別: Diagnostic
- 目的: Rust を読まなくても、どのラベル参照が壊れているか分かるようにする
- 期待する確認材料:
  - 入力 Markdown 例
  - Diagnostic 例
  - テストケース
- メモ:
  - `rule_id`, `span`, `suggestion` を確認する
```

完了時:

```markdown
### T-0001 未定義ラベル Diagnostic の改善

- 状態: Done
- 完了日: 2026-05-17
- 確認:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- 変更:
  - 未定義ラベルの Diagnostic に修正候補を追加
  - Golden JSON を更新
```

## Inbox

自由に追記してください。Codex があとで整理します。

- [ ] 

## Backlog

## Doing

## Done

### T-0001 実行系コマンドの事前検査・読み込み処理の共通化

- 状態: Done
- 完了日: 2026-07-12
- 確認:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- 変更:
  - `trace` / `routes` / `compile` で重複していた check + project load の入口を `load_checked_project` に集約
  - 各コマンド固有の実行・探索・bundle 生成ロジックは既存モジュールに残し、影響範囲を入口処理に限定

## Dropped
