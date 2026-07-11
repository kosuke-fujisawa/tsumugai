import { readFileSync, writeFileSync } from "node:fs";
import {
  buildDiffArgs,
  inputPath,
  listTrackedFiles,
  readTextIfExists,
  runGit,
  truncateText,
  writeJson,
} from "./lib.mjs";

const maxDiffChars = Number(process.env.AI_REVIEW_MAX_DIFF_CHARS || 30_000);
const baseRef = process.env.GITHUB_BASE_REF ? `origin/${process.env.GITHUB_BASE_REF}` : "HEAD^";

let diff = "";
try {
  diff = runGit(buildDiffArgs(`${baseRef}...HEAD`));
} catch {
  diff = runGit(buildDiffArgs("HEAD^...HEAD"));
}

const agentFiles = listTrackedFiles(["AGENTS.md", "**/AGENTS.md"]).slice(0, 3);
const agentInstructions = agentFiles.map((file) => ({
  file,
  content: truncateText(readFileSync(file, "utf8"), 8_000).text,
}));

const reviewInstructions = truncateText(
  readTextIfExists(".github/ai-review-instructions.md"),
  8_000,
).text;
const truncatedDiff = truncateText(diff, maxDiffChars);

writeJson(inputPath, {
  pullRequest: {
    number: process.env.PR_NUMBER || "",
    baseRef: process.env.GITHUB_BASE_REF || "",
    headRef: process.env.GITHUB_HEAD_REF || "",
  },
  reviewInstructions,
  agentInstructions,
  diff: truncatedDiff.text,
  diffTruncated: truncatedDiff.truncated,
});

writeFileSync("tmp/ai-review/diff.patch", `${truncatedDiff.text}\n`);
