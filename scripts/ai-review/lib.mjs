import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

export const outputDir = "tmp/ai-review";
export const inputPath = `${outputDir}/input.json`;
export const resultPath = `${outputDir}/result.json`;
export const commentPath = `${outputDir}/comment.md`;

const excludedDiffPathspecs = [
  ":(exclude,glob)**/*.md",
  ":(exclude,glob)**/*.txt",
  ":(exclude,glob)**/*.lock",
  ":(exclude,glob)**/package-lock.json",
  ":(exclude,glob)**/yarn.lock",
  ":(exclude,glob)**/pnpm-lock.yaml",
  ":(exclude,glob)**/Package.resolved",
  ":(exclude,glob)**/dist/**",
  ":(exclude,glob)**/build/**",
  ":(exclude,glob)**/DerivedData/**",
  ":(exclude,glob)**/*.min.js",
  ":(exclude,glob)**/*.map",
  ":(exclude,glob)**/*.png",
  ":(exclude,glob)**/*.jpg",
  ":(exclude,glob)**/*.jpeg",
  ":(exclude,glob)**/*.gif",
  ":(exclude,glob)**/*.webp",
  ":(exclude,glob)**/*.svg",
  ":(exclude,glob)**/*.pdf",
  ":(exclude,glob)**/*.zip",
  ":(exclude,glob).github/workflows/ai-review.yml",
  ":(exclude,glob)scripts/ai-review/**",
];

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

export function buildDiffArgs(range) {
  return [
    "diff",
    "--unified=20",
    "--find-renames",
    "--diff-filter=ACDMRT",
    range,
    "--",
    ".",
    ...excludedDiffPathspecs,
  ];
}

export function shouldSkipReview(input) {
  return typeof input?.diff !== "string" || input.diff.trim().length === 0;
}

export async function collectPaginatedItems(loadPage) {
  const perPage = 100;
  const items = [];

  for (let page = 1; ; page += 1) {
    const batch = await loadPage(page, perPage);
    if (!Array.isArray(batch)) {
      throw new TypeError("ページ取得結果は配列である必要があります。");
    }

    items.push(...batch);
    if (batch.length < perPage) {
      return items;
    }
  }
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
    return `${marker}
## AIレビュー

レビューはスキップされました。

理由: ${result.reason || "不明"}
`;
  }

  if (findings.length === 0) {
    return `${marker}
## AIレビュー

重大な指摘は見つかりませんでした。

対象: PR差分のみ
`;
  }

  const body = findings
    .map((finding, index) => {
      const location = finding.file ? `${finding.file}${finding.line ? `:${finding.line}` : ""}` : "場所未指定";
      return `${index + 1}. **${finding.severity}** ${finding.title}
   - 場所: \`${location}\`
   - 内容: ${finding.body}`;
    })
    .join("\n\n");

  return `${marker}
## AIレビュー

${body}
`;
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
  const findings = Array.isArray(parsed.findings) ? parsed.findings : [];
  const allowedSeverities = new Set(["critical", "high", "medium"]);

  return {
    status: parsed.status || "completed",
    findings: findings
      .filter(
        (finding) =>
          finding &&
          finding.confidence === "high" &&
          allowedSeverities.has(finding.severity),
      )
      .slice(0, 3),
  };
}
