//! check 結果の出力形式
//!
//! - [`render_human`]: SPEC 6.1 の例に準拠した rustc 風の人間向け出力
//! - [`render_json`]: CI・LLM デバッグ依頼向けの安定 JSON
//! - [`render_sarif`]: GitHub Code Scanning に取り込める SARIF 2.1.0
//!
//! いずれもエラーの有無にかかわらず同じ形式で出力する（SPEC 6.1 /
//! docs/CLI_OUTPUT.md）。

use super::check::CheckResult;
use super::diagnostic::{Diagnostic, Severity};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------- 人間向け出力

/// rustc 風の人間向け出力（SPEC 6.1 のフォーマット例に準拠）
pub fn render_human(result: &CheckResult) -> String {
    let mut out = String::new();
    let mut sources: HashMap<PathBuf, Option<Vec<String>>> = HashMap::new();
    for diag in &result.diagnostics {
        render_one(&mut out, diag, &mut sources);
        out.push('\n');
    }
    if result.diagnostics.is_empty() {
        let _ = writeln!(
            out,
            "✓ 問題は見つかりませんでした。（{} ファイルを検査）",
            result.files.len()
        );
    } else {
        let _ = writeln!(
            out,
            "エラー: {}件  警告: {}件（{} ファイルを検査）",
            result.error_count(),
            result.warning_count(),
            result.files.len()
        );
    }
    out
}

fn render_one(
    out: &mut String,
    diag: &Diagnostic,
    sources: &mut HashMap<PathBuf, Option<Vec<String>>>,
) {
    let severity = severity_word(diag.severity);
    let _ = writeln!(out, "{severity}[{}]: {}", diag.rule_id, diag.message);

    let Some(span) = &diag.span else {
        let _ = writeln!(out, "  --> {}", diag.file.display());
        if let Some(suggestion) = &diag.suggestion {
            let _ = writeln!(out, "   = help: {}", indent_continuation(suggestion, 3));
        }
        return;
    };

    let width = span.line.to_string().len().max(2);
    let _ = writeln!(
        out,
        "{:width$}--> {}:{}",
        "",
        diag.file.display(),
        span.line
    );
    let source_line = sources
        .entry(diag.file.clone())
        .or_insert_with(|| {
            std::fs::read_to_string(&diag.file)
                .ok()
                .map(|s| s.lines().map(String::from).collect())
        })
        .as_ref()
        .and_then(|lines| lines.get(span.line - 1))
        .cloned();
    if let Some(text) = source_line {
        let _ = writeln!(out, "{:width$} |", "");
        let _ = writeln!(out, "{:width$} | {text}", span.line);
        let _ = writeln!(out, "{:width$} |", "");
    }
    if let Some(suggestion) = &diag.suggestion {
        let _ = writeln!(
            out,
            "{:width$} = help: {}",
            "",
            indent_continuation(suggestion, width + 10)
        );
    }
    if !diag.related_spans.is_empty() {
        let lines: Vec<String> = diag
            .related_spans
            .iter()
            .map(|s| s.line.to_string())
            .collect();
        let _ = writeln!(
            out,
            "{:width$} = note: 関連する行: {}",
            "",
            lines.join("、")
        );
    }
}

/// 複数行の suggestion を出力の桁に合わせて折り返す
fn indent_continuation(text: &str, indent: usize) -> String {
    text.replace('\n', &format!("\n{:indent$}", ""))
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

// ------------------------------------------------------------------- JSON

/// 機械向け JSON 出力（docs/CLI_OUTPUT.md のスキーマ）
pub fn render_json(result: &CheckResult) -> String {
    let value = json!({
        "status": if result.has_errors() { "error" } else { "ok" },
        "files": result.files,
        "error_count": result.error_count(),
        "warning_count": result.warning_count(),
        "diagnostics": result.diagnostics,
    });
    serde_json::to_string_pretty(&value).expect("JSON のシリアライズは失敗しない")
}

// ------------------------------------------------------------------ SARIF

/// SARIF 2.1.0 出力（GitHub Code Scanning 取り込み用）
pub fn render_sarif(result: &CheckResult) -> String {
    let mut rule_ids: Vec<&str> = result.diagnostics.iter().map(|d| d.rule_id).collect();
    rule_ids.sort_unstable();
    rule_ids.dedup();
    let rules: Vec<Value> = rule_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "shortDescription": { "text": rule_summary(id) },
                "helpUri": "https://github.com/kosuke-fujisawa/tsumugai/blob/main/SPEC.md",
            })
        })
        .collect();
    let results: Vec<Value> = result.diagnostics.iter().map(sarif_result).collect();
    let value = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "tsumugai",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/kosuke-fujisawa/tsumugai",
                    "rules": rules,
                }
            },
            "results": results,
        }],
    });
    serde_json::to_string_pretty(&value).expect("JSON のシリアライズは失敗しない")
}

fn sarif_result(diag: &Diagnostic) -> Value {
    let mut text = diag.message.clone();
    if let Some(suggestion) = &diag.suggestion {
        text.push_str("\n提案:\n");
        text.push_str(suggestion);
    }
    json!({
        "ruleId": diag.rule_id,
        "level": severity_word(diag.severity),
        "message": { "text": text },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": { "uri": sarif_uri(&diag.file) },
                "region": { "startLine": diag.span.as_ref().map_or(1, |s| s.line) },
            }
        }],
    })
}

/// SARIF の artifactLocation.uri（`/` 区切り、`./` なし）
fn sarif_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    s.strip_prefix("./").unwrap_or(&s).to_string()
}

/// SARIF の rules 向けの短い説明（SPEC 6章のルール表に対応）
fn rule_summary(rule_id: &str) -> &'static str {
    match rule_id {
        "missing-scene-id" => "front matter に id がない",
        "invalid-frontmatter" => "front matter の YAML が解析できない、または値が文字列でない",
        "duplicate-scene-id" => "シーン ID がプロジェクト内で重複している",
        "duplicate-anchor" => "同一ファイル内で H2 アンカー名が重複している",
        "empty-anchor" => "H2 見出しから導出したアンカー名が空になる",
        "invalid-h1" => "H1 が複数ある、またはファイル先頭以外にある",
        "broken-link" => "選択肢・ジャンプのリンク先が解決できない",
        "invalid-choice-item" => "選択肢リストにリンク以外の項目が混在している",
        "empty-choice-label" => "選択肢のリンクテキストが空",
        "missing-asset" => "background / bgm のパスが実在しない",
        "legacy-command" => "旧記法（v0）のコマンドが使われている",
        "invalid-characters-file" => "characters.yaml が読み込めない、または形式が正しくない",
        "undefined-character" => "characters.yaml に宣言されていない話者",
        "implicit-fallthrough" => "セクション末尾が ending・ジャンプ・選択肢のいずれでもない",
        "missing-title" => "H1 タイトルがない",
        "linkless-list" => "リンクを 1 つも含まないリスト",
        "inline-link" => "本文中のインラインリンク",
        "setext-heading" => "setext 形式の見出し",
        "unsupported-element" => "v1 で意味を定義していない Markdown 要素",
        "missing-characters-file" => "characters.yaml が見つからない",
        "unreachable-section" => "どこからも到達しないセクション",
        "deep-heading" => "H3 以深の見出し",
        "unknown-frontmatter-key" => "front matter の未知キー",
        "unknown-directive" => "HTML コメントの未知の制御キー",
        "io-error" => "ファイルの読み込みに失敗した（記法ではなく環境の問題）",
        _ => "tsumugai check の診断",
    }
}
