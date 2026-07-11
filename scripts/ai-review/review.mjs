import { writeFileSync } from "node:fs";
import {
  buildReviewMarkdown,
  commentPath,
  extractResponseText,
  inputPath,
  parseReviewJson,
  readJson,
  resultPath,
  writeJson,
} from "./lib.mjs";

const apiKey = process.env.OPENAI_API_KEY;
const model = process.env.AI_REVIEW_MODEL || "gpt-5-mini";
const input = readJson(inputPath);

if (!apiKey) {
  const result = {
    status: "skipped",
    reason: "OPENAI_API_KEY が設定されていません。Repository secrets に OPENAI_API_KEY を追加してください。",
    findings: [],
  };
  writeJson(resultPath, result);
  writeFileSync(commentPath, buildReviewMarkdown(result));
  process.exit(0);
}

const systemPrompt = `あなたはGitHub Pull Requestの自動レビュアーです。
日本語で回答してください。
PR差分に直接関係する、再現性のある指摘だけを返してください。
重大なバグ、セキュリティ、データ破壊、テスト不足を優先してください。
確信度が低い指摘、好みのスタイル指摘、差分外の設計論は返さないでください。`;

const userPrompt = `以下のPR差分をレビューしてください。

## レビュー方針
${input.reviewInstructions}

## AGENTS.md
${input.agentInstructions.map((item) => `### ${item.file}\n${item.content}`).join("\n\n")}

## PR情報
${JSON.stringify(input.pullRequest, null, 2)}

## 注意
diffTruncated=${input.diffTruncated}

## Diff
\`\`\`diff
${input.diff}
\`\`\``;

const schema = {
  type: "object",
  additionalProperties: false,
  required: ["status", "findings"],
  properties: {
    status: { type: "string", enum: ["completed"] },
    findings: {
      type: "array",
      maxItems: 20,
      items: {
        type: "object",
        additionalProperties: false,
        required: ["severity", "file", "line", "title", "body"],
        properties: {
          severity: { type: "string", enum: ["critical", "high", "medium", "low"] },
          file: { type: "string" },
          line: { type: ["integer", "null"] },
          title: { type: "string" },
          body: { type: "string" },
        },
      },
    },
  },
};

const response = await fetch("https://api.openai.com/v1/responses", {
  method: "POST",
  headers: {
    Authorization: `Bearer ${apiKey}`,
    "Content-Type": "application/json",
  },
  body: JSON.stringify({
    model,
    input: [
      { role: "system", content: [{ type: "input_text", text: systemPrompt }] },
      { role: "user", content: [{ type: "input_text", text: userPrompt }] },
    ],
    text: {
      format: {
        type: "json_schema",
        name: "review_result",
        schema,
        strict: true,
      },
    },
  }),
});

const data = await response.json();
if (!response.ok) {
  throw new Error(`OpenAI API error: ${response.status} ${JSON.stringify(data)}`);
}

const result = parseReviewJson(extractResponseText(data));
writeJson(resultPath, result);
writeFileSync(commentPath, buildReviewMarkdown(result));
