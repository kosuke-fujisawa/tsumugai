import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildDiffArgs,
  buildReviewMarkdown,
  parseReviewJson,
  shouldSkipReview,
  truncateText,
} from "./lib.mjs";

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

test("parseReviewJsonは高確信度の重大指摘だけを最大3件残す", () => {
  const findings = [
    ...Array.from({ length: 6 }, (_, index) => ({
      severity: "medium",
      confidence: "high",
      file: "src/example.js",
      line: index + 1,
      title: `指摘${index + 1}`,
      body: "再現可能な問題です。",
    })),
    {
      severity: "high",
      confidence: "medium",
      file: "src/uncertain.js",
      line: 1,
      title: "確信度不足",
      body: "推測を含みます。",
    },
    {
      severity: "low",
      confidence: "high",
      file: "src/style.js",
      line: 1,
      title: "軽微な指摘",
      body: "動作には影響しません。",
    },
  ];

  const result = parseReviewJson(JSON.stringify({ status: "completed", findings }));

  assert.equal(result.findings.length, 3);
  assert.ok(result.findings.every((finding) => finding.confidence === "high"));
  assert.ok(result.findings.every((finding) => finding.severity === "medium"));
});

test("buildDiffArgsは少ない文脈で自動生成物と文書を除外する", () => {
  const args = buildDiffArgs("origin/main...HEAD");

  assert.ok(args.includes("--unified=20"));
  assert.ok(args.includes("origin/main...HEAD"));
  assert.ok(args.includes(":(exclude,glob)**/*.md"));
  assert.ok(args.includes(":(exclude,glob)**/package-lock.json"));
  assert.ok(args.includes(":(exclude,glob)**/dist/**"));
  assert.ok(args.includes(":(exclude,glob)**/*.png"));
});

test("shouldSkipReviewは対象diffが空の場合だけスキップする", () => {
  assert.equal(shouldSkipReview({ diff: "\n  " }), true);
  assert.equal(shouldSkipReview({ diff: "+const enabled = true;" }), false);
});
