開発分担　開発ワークフロー

1. 要求仕様を詰める（ChatGPT）
    出力：SPEC.md（I/O契約・異常系・不変条件・SPEC-ID）
2. 要求仕様を満たすようにテストを書く（赤で入れる）
    振る舞い命名＋[SPEC-ID]を必ず付与
3. 実装依頼（Claude Codeへ）
    ChatGPTが指示テンプレ生成（“指定ファイルのみ・Unified diff・#[allow]禁止・最小変更”）
4. コードレビュー、PR作成
5. CI & Botレビュー（PR上） /review CodeRabbit Codex ChatGPT
    fmt / clippy -D warnings / test
    Block: テスト削除/編集、SPEC-ID不整合
    Botロール分担でコメント
6. 必要ならテスト追加→ 3へ再ループ
    追加分は必ず赤で入れる（TDD継続）
7. 対応すべきものがなくなれば Merge

大きめの改修後
ChatGPT/Gemini CLI/Codex でリファクタリング観点でのレビューを実施
方針にしたがってリファクタリングを実施
ドキュメントへの反映が必要な場合、Gemini CLIで実施

・ChatGPT
　要件・仕様検討、リファクタリング方針決定、PRレビュー
・Claude Code
　実装・PR時セルフレビュー（/review）
・CodeRabbit
　PRレビュー（細かな指摘中心）
・Codex
　PRレビュー
・Gemini CLI
　ドキュメンテーション
