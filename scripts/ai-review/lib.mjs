import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

export const outputDir = "tmp/ai-review";
export const inputPath = `${outputDir}/input.json`;
export const resultPath = `${outputDir}/result.json`;
export const commentPath = `${outputDir}/comment.md`;

export function ensureParentDir(path) {
  mkdirSync(dirname(path), { recursive: true });
}

export function readTextIfExists(path) {
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

export function writeJson(path, value) {
  ensureParentDir(path);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

export function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

export function truncateText(text, maxChars) {
  if (text.length <= maxChars) {
    return { text, truncated: false };
  }

  return {
    text: `${text.slice(0, maxChars)}\n\n[truncated: ${text.length - maxChars} chars omitted]`,
    truncated: true,
  };
}

export function runGit(args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

export function listTrackedFiles(patterns) {
  try {
    return runGit(["ls-files", ...patterns])
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

export function buildReviewMarkdown(result) {
  const marker = "<!-- ai-review-bot -->";
  const findings = Array.isArray(result.findings) ? result.findings : [];
  const skipped = result.status === "skipped";

  if (skipped) {
    return `${marker}\n## AIレビュー\n\nレビューはスキップされました。\n\n理由: ${result.reason || "不明"}\n`;
  }

  if (findings.length === 0) {
    return `${marker}\n## AIレビュー\n\n重大な指摘は見つかりませんでした。\n\n対象: PR差分のみ\n`;
  }

  const body = findings
    .map((finding, index) => {
      const location = finding.file ? `${finding.file}${finding.line ? `:${finding.line}` : ""}` : "場所未指定";
      return `${index + 1}. **${finding.severity}** ${finding.title}\n   - 場所: \`${location}\`\n   - 内容: ${finding.body}`;
    })
    .join("\n\n");

  return `${marker}\n## AIレビュー\n\n${body}\n`;
}

export function extractResponseText(data) {
  if (typeof data.output_text === "string") {
    return data.output_text;
  }

  const chunks = [];
  for (const output of data.output || []) {
    for (const content of output.content || []) {
      if (typeof content.text === "string") {
        chunks.push(content.text);
      }
    }
  }
  return chunks.join("\n");
}

export function parseReviewJson(text) {
  const parsed = JSON.parse(text);
  return {
    status: parsed.status || "completed",
    findings: Array.isArray(parsed.findings) ? parsed.findings : [],
  };
}
