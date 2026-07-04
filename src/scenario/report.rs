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
use super::trace::{TraceEnd, TraceResult, TraceStep};
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
        .and_then(|lines| span.line.checked_sub(1).and_then(|i| lines.get(i)))
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

// ---------------------------------------------------------- trace 人間向け

/// trace の人間向け出力（SPEC 5.1）
///
/// 実行前検査が error のときは check とまったく同じ出力になる（SPEC 6.1:
/// どのコマンドから入っても同じ指摘に到達できる）。warning は経路の前に
/// 見せてから経路を表示する。
pub fn render_trace_human(result: &TraceResult) -> String {
    let Some(trace) = &result.trace else {
        return render_human(&result.check);
    };
    let mut out = String::new();
    if !result.check.diagnostics.is_empty() {
        out.push_str(&render_human(&result.check));
        out.push('\n');
    }
    let _ = writeln!(out, "=== Trace: {} ===", result.file.display());
    for step in &trace.steps {
        render_trace_step(&mut out, step);
    }
    out.push('\n');
    render_trace_end(&mut out, trace);
    out
}

fn render_trace_step(out: &mut String, step: &TraceStep) {
    match step {
        TraceStep::SceneEnter {
            file,
            id,
            title,
            background,
            bgm,
        } => {
            let id = id.as_deref().unwrap_or("(id なし)");
            let title = title.as_deref().unwrap_or("(タイトルなし)");
            let _ = writeln!(out, "\n▶ シーン {id}「{title}」 ({})", file.display());
            if let Some(background) = background {
                let _ = writeln!(out, "      background: {background}");
            }
            if let Some(bgm) = bgm {
                let _ = writeln!(out, "      bgm: {bgm}");
            }
        }
        TraceStep::SectionEnter { line, heading, .. } => {
            let _ = writeln!(out, "  ── セクション「{heading}」（{line} 行目）");
        }
        TraceStep::Narration { line, text, .. } => {
            let _ = writeln!(out, "  {line:>4}| {text}");
        }
        TraceStep::Dialogue {
            line,
            speaker,
            text,
            ..
        } => {
            let _ = writeln!(out, "  {line:>4}| {speaker}: {text}");
        }
        TraceStep::Choice {
            line,
            options,
            selected,
            ..
        } => {
            let _ = writeln!(out, "  {line:>4}| 選択肢:");
            for (i, option) in options.iter().enumerate() {
                let _ = writeln!(
                    out,
                    "          {}. [{}]({})",
                    i + 1,
                    option.label,
                    option.target
                );
            }
            match selected {
                Some(n) => {
                    let label = &options[n - 1].label;
                    let _ = writeln!(out, "        → {n} を選択「{label}」");
                }
                None => {
                    let _ = writeln!(out, "        → （入力待ちで停止）");
                }
            }
        }
        TraceStep::Jump {
            line,
            label,
            target,
            ..
        } => {
            let _ = writeln!(out, "  {line:>4}| ジャンプ → [{label}]({target})");
        }
        TraceStep::Ending { line, id, .. } => {
            let _ = writeln!(out, "  {line:>4}| エンディング: {id}");
        }
    }
}

fn render_trace_end(out: &mut String, trace: &super::trace::Trace) {
    match &trace.end {
        TraceEnd::Ending { id } => {
            let _ = writeln!(out, "結果: エンディング「{id}」に到達しました");
        }
        TraceEnd::EndOfFile => {
            let _ = writeln!(
                out,
                "結果: ファイル末尾に到達して終了しました（明示的なエンディングなし）"
            );
        }
        TraceEnd::AwaitingChoice => {
            let mut example: Vec<String> = trace
                .choices_requested
                .iter()
                .map(usize::to_string)
                .collect();
            example.push("1".to_string());
            let _ = writeln!(
                out,
                "結果: 選択肢の入力待ちで停止しました。--choices に選択番号を足すと先へ進めます（例: --choices {}）",
                example.join(",")
            );
        }
        TraceEnd::InvalidChoice { given, available } => {
            let _ = writeln!(
                out,
                "結果: エラー: 選択番号 {given} はこの選択肢にありません。1〜{available} から選んでください"
            );
        }
        TraceEnd::Truncated { max_steps } => {
            let _ = writeln!(
                out,
                "結果: エラー: {max_steps} ステップを超えたため打ち切りました。ジャンプが同じ場所を回り続けていないか確認してください"
            );
        }
    }
    if trace.choices_used > 0 {
        let used: Vec<String> = trace.choices_requested[..trace.choices_used]
            .iter()
            .map(usize::to_string)
            .collect();
        let _ = writeln!(out, "入力した選択肢: --choices {}", used.join(","));
    }
    if trace.choices_used < trace.choices_requested.len() {
        let unused: Vec<String> = trace.choices_requested[trace.choices_used..]
            .iter()
            .map(usize::to_string)
            .collect();
        let _ = writeln!(
            out,
            "未使用の選択番号: {}（実行が終了したため使われませんでした）",
            unused.join(",")
        );
    }
}

// ------------------------------------------------------------ trace JSON

/// trace の機械向け JSON 出力（docs/CLI_OUTPUT.md のスキーマ）。
/// check の JSON の上位互換で、`file` と `trace` が加わる。
/// 実行前検査が error・入出力エラーのときも同じ形式を保つ（`trace` が null）
pub fn render_trace_json(result: &TraceResult) -> String {
    let value = json!({
        "status": if result.has_errors() { "error" } else { "ok" },
        "file": result.file,
        "files": result.check.files,
        "error_count": result.check.error_count(),
        "warning_count": result.check.warning_count(),
        "diagnostics": result.check.diagnostics,
        "trace": result.trace,
    });
    serde_json::to_string_pretty(&value).expect("JSON のシリアライズは失敗しない")
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

#[cfg(test)]
mod tests {
    use super::rule_summary;

    /// SPEC 6章のルール表（error 12種 + warning 12種）+ CLI レベルの io-error。
    /// ルールを追加したら SPEC → この一覧 → rule_summary の順に更新する
    const ALL_RULE_IDS: [&str; 25] = [
        "missing-scene-id",
        "invalid-frontmatter",
        "duplicate-scene-id",
        "duplicate-anchor",
        "empty-anchor",
        "invalid-h1",
        "broken-link",
        "invalid-choice-item",
        "empty-choice-label",
        "missing-asset",
        "legacy-command",
        "invalid-characters-file",
        "undefined-character",
        "implicit-fallthrough",
        "missing-title",
        "linkless-list",
        "inline-link",
        "setext-heading",
        "unsupported-element",
        "missing-characters-file",
        "unreachable-section",
        "deep-heading",
        "unknown-frontmatter-key",
        "unknown-directive",
        "io-error",
    ];

    #[test]
    fn 全ルールにsarif用の個別説明がある() {
        let fallback = rule_summary("__unknown_rule__");
        for rule_id in ALL_RULE_IDS {
            assert_ne!(
                rule_summary(rule_id),
                fallback,
                "{rule_id} の説明が rule_summary に未登録"
            );
        }
    }
}
