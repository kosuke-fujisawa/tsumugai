import { readFileSync } from "node:fs";
import { commentPath } from "./lib.mjs";

const token = process.env.GITHUB_TOKEN;
const repository = process.env.GITHUB_REPOSITORY;
const prNumber = process.env.PR_NUMBER;
const body = readFileSync(commentPath, "utf8");
const marker = "<!-- ai-review-bot -->";

if (!token || !repository || !prNumber) {
  throw new Error("GITHUB_TOKEN、GITHUB_REPOSITORY、PR_NUMBER が必要です。");
}

const apiBase = "https://api.github.com";
const headers = {
  Authorization: `Bearer ${token}`,
  Accept: "application/vnd.github+json",
  "X-GitHub-Api-Version": "2022-11-28",
  "Content-Type": "application/json",
};

async function request(path, options = {}) {
  const response = await fetch(`${apiBase}${path}`, { ...options, headers });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(`GitHub API error: ${response.status} ${JSON.stringify(data)}`);
  }
  return data;
}

const comments = await request(`/repos/${repository}/issues/${prNumber}/comments`);
const existing = comments.find((comment) => typeof comment.body === "string" && comment.body.includes(marker));

if (existing) {
  await request(`/repos/${repository}/issues/comments/${existing.id}`, {
    method: "PATCH",
    body: JSON.stringify({ body }),
  });
} else {
  await request(`/repos/${repository}/issues/${prNumber}/comments`, {
    method: "POST",
    body: JSON.stringify({ body }),
  });
}
