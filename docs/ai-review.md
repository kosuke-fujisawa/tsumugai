# AIレビュー設定

同一リポジトリ内の非ドラフトPRを作成・更新すると、`.github/workflows/ai-review.yml` が共有Action `kosuke-fujisawa/ai-review-action@v1` を実行します。

## 必須設定

- Repository secret `OPENAI_API_KEY`: OpenAI APIキー
- Repository variable `AI_REVIEW_MODEL`: 任意。未設定時は `gpt-5-mini`

## 動作

1. PR差分をファイル単位で配分し、`AGENTS.md`、固有レビュー方針、GitHub check-runsを収集する。
2. 削除シンボルについて、HEADに残る参照を `git grep` で検証する。
3. OpenAI Responses APIへレビューを依頼する。
4. 実在する場所・検証コマンド・実行経路・反証結果を提示できる指摘だけをコメントする。
5. 入力・出力・コメント本文を `ai-review` artifactへ保存する。

差分予算は最大120,000文字です。高確信度の `critical`、`high`、`medium` だけを最大3件返します。

## 共通基盤

実装とテストの正本は [kosuke-fujisawa/ai-review-action](https://github.com/kosuke-fujisawa/ai-review-action) です。このリポジトリにはWorkflowとプロジェクト固有のレビュー方針だけを置きます。
