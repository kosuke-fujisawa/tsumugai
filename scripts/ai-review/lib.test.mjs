import assert from "node:assert/strict";
import { test } from "node:test";
import { buildReviewMarkdown, parseReviewJson, truncateText } from "./lib.mjs";

test("truncateTextは上限を超えた文字列を切り詰める", () => {
  const result = truncateText("abcdef", 3);

  assert.equal(result.truncated, true);
  assert.match(result.text, /^abc/);
  assert.match(result.text, /truncated: 3 chars omitted/);
});

test("buildReviewMarkdownは指摘なしのコメントを生成する", () => {
  const markdown = buildReviewMarkdown({ status: "completed", findings: [] });

  assert.match(markdown, /<!-- ai-review-bot -->/);
  assert.match(markdown, /重大な指摘は見つかりませんでした/);
});

test("parseReviewJsonはfindingsがない場合に空配列へ正規化する", () => {
  const result = parseReviewJson('{"status":"completed"}');

  assert.equal(result.status, "completed");
  assert.deepEqual(result.findings, []);
});
