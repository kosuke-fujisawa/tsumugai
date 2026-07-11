# AIレビュー設定

このリポジトリでは、PR作成・更新時に `.github/workflows/ai-review.yml` が起動し、PR差分をLLMでレビューします。

## 必須設定

Repository secrets に以下を追加します。

- `OPENAI_API_KEY`: OpenAI APIキー

Repository variables は任意です。

- `AI_REVIEW_MODEL`: 使用するモデル。未設定時は `gpt-5-mini`

## 動作

1. PR差分と `AGENTS.md`、`.github/ai-review-instructions.md` を収集する。
2. OpenAI Responses APIにレビューを依頼する。
3. 同一リポジトリ内PRの場合、PRコメントを作成または更新する。
4. 入力・出力・コメント本文を `ai-review` artifact に保存する。

`OPENAI_API_KEY` が未設定の場合、レビューはスキップされます。

## ローカル確認

```bash
node --test scripts/ai-review/*.test.mjs
node scripts/ai-review/collect-input.mjs
node scripts/ai-review/review.mjs
```

APIキーなしで `review.mjs` を実行した場合、スキップ結果が `tmp/ai-review/` に生成されます。
