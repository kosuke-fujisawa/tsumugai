//! 推測整形（tsumugai fmt, SPEC 7章）
//!
//! よくある書き方のパターンを**決定的なルールベース**で v1 記法へ整形する。
//! LLM 等による自由な構成推測は行わない（それは外部 LLM の役割で、
//! tsumugai はその結果を check で検証する側に立つ）。
//!
//! 黙って書き換えないため、変換は 1 件ずつ [`FmtChange`]（どの行を・
//! どのルールで・どう変えたか）として蓄積し、まとめてから適用する。
//! 確信が持てない箇所は変換せず、check と同じ [`Diagnostic`] 形式で報告する。
//!
//! 認識するパターン（SPEC 7.1）:
//! - `fmt-missing-frontmatter`: front matter がなければファイル名から補う
//! - `fmt-legacy`: 旧記法の一部（SAY / LABEL / JUMP / WAIT / `[c]` /
//!   ENDING・END / BRANCH）を確定的に変換する。それ以外の旧記法
//!   （SET / MODIFY / JUMP_IF / SHOW_IMAGE / PLAY_* / CLEAR_LAYER /
//!   `:::` ブロック等）は変換せず `legacy-command` として報告する
//! - `fmt-kagi-dialogue`: `名前「本文」` → `名前: 本文`
//! - `fmt-paren-dialogue`: `名前（本文）` → `名前: （本文）`
//!   （話者が characters.yaml に宣言済みの場合のみ）
//! - `fmt-linkless-choice`: リンクのないリスト（`・` リストを含む）を、
//!   全項目が H2 見出しに一致する場合のみ選択肢リストへ変換する

use super::characters::{Characters, find_characters_file, load_characters};
use super::diagnostic::{Diagnostic, Severity};
use super::parse;
use super::project::file_level;
use super::slugify;
use std::path::{Path, PathBuf};

/// fmt の 1 件の変更（実際に書き換えた行）
#[derive(Debug, Clone, PartialEq)]
pub struct FmtChange {
    pub rule_id: &'static str,
    /// 変換前ファイルでの開始行（1-origin）。先頭への挿入は 1
    pub line: usize,
    /// 変換前のテキスト（複数行は `\n` 区切り、挿入のみの場合は空文字列）
    pub before: String,
    /// 変換後のテキスト（削除のみの場合は空文字列）
    pub after: String,
}

/// fmt の結果。入出力エラーも [`Diagnostic`] にして返す（infallible）
#[derive(Debug)]
pub struct FmtResult {
    pub path: PathBuf,
    pub original: String,
    pub formatted: String,
    pub changes: Vec<FmtChange>,
    pub diagnostics: Vec<Diagnostic>,
}

impl FmtResult {
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// ファイルを読み込んで整形する（SPEC 7章）。
///
/// パスがディレクトリ・存在しない・読めない場合も panic や Err にせず、
/// `io-error` の Diagnostic を持つ [`FmtResult`] を返す
pub fn fmt_path(path: &Path) -> FmtResult {
    if path.is_dir() {
        return FmtResult {
            path: path.to_path_buf(),
            original: String::new(),
            formatted: String::new(),
            changes: Vec::new(),
            diagnostics: vec![file_level(
                "io-error",
                Severity::Error,
                path,
                format!(
                    "{} はディレクトリです。fmt は整形するシーンファイル（.md）を 1 つ指定してください",
                    path.display()
                ),
            )],
        };
    }
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return FmtResult {
                path: path.to_path_buf(),
                original: String::new(),
                formatted: String::new(),
                changes: Vec::new(),
                diagnostics: vec![file_level(
                    "io-error",
                    Severity::Error,
                    path,
                    format!("{} を読み込めません: {}", path.display(), e),
                )],
            };
        }
    };
    let characters = find_characters_file(path).and_then(|p| load_characters(&p).ok());
    fmt_str(&source, path, characters.as_ref())
}

/// 文字列を整形する。`characters` は `fmt-paren-dialogue` の話者宣言判定に使う
pub fn fmt_str(source: &str, path: &Path, characters: Option<&Characters>) -> FmtResult {
    let trailing_newline = source.ends_with('\n');
    let lines: Vec<&str> = source.lines().collect();
    let mut edits: Vec<Edit> = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    let scan_start = match front_matter_end(&lines) {
        Some(end) => end,
        None => {
            edits.push(Edit {
                start: 0,
                end: 0,
                new_lines: vec![
                    "---".to_string(),
                    format!("id: {}", derive_id(path)),
                    "---".to_string(),
                    String::new(),
                ],
                rule_id: "fmt-missing-frontmatter",
            });
            0
        }
    };

    let headings = collect_headings(&lines, scan_start);

    let mut i = scan_start;
    while i < lines.len() {
        let trimmed = lines[i].trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if trimmed == "[c]" {
            edits.push(Edit::replace(i, Vec::new(), "fmt-legacy"));
            i += 1;
            continue;
        }

        if let Some((name, rest)) = bracket_command(trimmed) {
            match try_legacy(name, rest, &lines, i) {
                Some(edit) => {
                    i = edit.end;
                    edits.push(edit);
                }
                None => {
                    if let Some((message, suggestion)) = parse::legacy_command(trimmed) {
                        diagnostics.push(
                            Diagnostic::error("legacy-command", path, i + 1, message)
                                .with_suggestion(suggestion),
                        );
                    }
                    i += 1;
                }
            }
            continue;
        }

        if trimmed.starts_with(":::") && trimmed != ":::" {
            if let Some((message, suggestion)) = parse::legacy_command(trimmed) {
                diagnostics.push(
                    Diagnostic::error("legacy-command", path, i + 1, message)
                        .with_suggestion(suggestion),
                );
            }
            i += 1;
            continue;
        }

        if let Some((speaker, body)) = match_kagi(trimmed) {
            edits.push(Edit::replace(
                i,
                vec![format!("{speaker}: {body}")],
                "fmt-kagi-dialogue",
            ));
            i += 1;
            continue;
        }

        if let Some((speaker, body)) = match_paren(trimmed)
            && characters.is_some_and(|c| c.contains(&speaker))
        {
            edits.push(Edit::replace(
                i,
                vec![format!("{speaker}: （{body}）")],
                "fmt-paren-dialogue",
            ));
            i += 1;
            continue;
        }

        if let Some(block) = list_block(&lines, i) {
            match convert_linkless_choice(&block, &headings) {
                ListOutcome::Converted(new_lines) => {
                    edits.push(Edit {
                        start: block.start,
                        end: block.end,
                        new_lines,
                        rule_id: "fmt-linkless-choice",
                    });
                }
                ListOutcome::Unmatched(missing) => {
                    diagnostics.push(Diagnostic::warning(
                        "fmt-linkless-choice",
                        path,
                        block.start + 1,
                        format!(
                            "リンクのないリストですが、見出しに一致しない項目があるため選択肢に変換できません（{}）。見出しを追加するか、リンク付きで書き直してください",
                            missing.join("、")
                        ),
                    ));
                }
                ListOutcome::HasLinks | ListOutcome::Empty => {}
            }
            i = block.end;
            continue;
        }

        i += 1;
    }

    let formatted_lines = apply_edits(&lines, &edits);
    let mut formatted = formatted_lines.join("\n");
    if trailing_newline && !formatted.is_empty() {
        formatted.push('\n');
    }

    let changes: Vec<FmtChange> = edits
        .iter()
        .map(|edit| FmtChange {
            rule_id: edit.rule_id,
            line: edit.start + 1,
            before: lines[edit.start..edit.end].join("\n"),
            after: edit.new_lines.join("\n"),
        })
        .collect();

    diagnostics.sort_by_key(|d| d.span.as_ref().map_or(0, |s| s.line));

    FmtResult {
        path: path.to_path_buf(),
        original: source.to_string(),
        formatted,
        changes,
        diagnostics,
    }
}

// ------------------------------------------------------------------- Edit

/// 原本の行範囲 `[start, end)` を `new_lines` に置き換える 1 件の変更
struct Edit {
    start: usize,
    end: usize,
    new_lines: Vec<String>,
    rule_id: &'static str,
}

impl Edit {
    fn replace(line: usize, new_lines: Vec<String>, rule_id: &'static str) -> Self {
        Self {
            start: line,
            end: line + 1,
            new_lines,
            rule_id,
        }
    }
}

fn apply_edits(lines: &[&str], edits: &[Edit]) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0;
    for edit in edits {
        while idx < edit.start {
            out.push(lines[idx].to_string());
            idx += 1;
        }
        out.extend(edit.new_lines.iter().cloned());
        idx = edit.end;
    }
    while idx < lines.len() {
        out.push(lines[idx].to_string());
        idx += 1;
    }
    out
}

// ---------------------------------------------------- fmt-missing-frontmatter

fn front_matter_end(lines: &[&str]) -> Option<usize> {
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim_end() == "---")
        .map(|(i, _)| i + 1)
}

/// ファイル名（拡張子なし）からシーン ID を補う
fn derive_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("scene")
        .to_string()
}

// -------------------------------------------------------- fmt-kagi/paren-dialogue

/// `名前「本文」` の全角かぎ括弧セリフ（行全体がこの形の場合のみ）
fn match_kagi(line: &str) -> Option<(String, String)> {
    match_bracket_pair(line, '「', '」')
}

/// `名前（本文）` の丸括弧の内心（行全体がこの形の場合のみ）
fn match_paren(line: &str) -> Option<(String, String)> {
    match_bracket_pair(line, '（', '）')
}

fn match_bracket_pair(line: &str, open: char, close: char) -> Option<(String, String)> {
    let open_idx = line.find(open)?;
    if !line.ends_with(close) {
        return None;
    }
    let speaker = &line[..open_idx];
    if speaker.is_empty() || speaker.chars().any(char::is_whitespace) {
        return None;
    }
    let body_start = open_idx + open.len_utf8();
    let body_end = line.len() - close.len_utf8();
    if body_end < body_start {
        return None;
    }
    let body = &line[body_start..body_end];
    if body.contains(open) || body.contains(close) {
        return None; // 入れ子は対象外（確信が持てない）
    }
    Some((speaker.to_string(), body.to_string()))
}

// ---------------------------------------------------------- fmt-linkless-choice

struct Heading {
    text: String,
    anchor: String,
}

fn collect_headings(lines: &[&str], from: usize) -> Vec<Heading> {
    lines[from..]
        .iter()
        .filter_map(|line| {
            let text = line.trim_start().strip_prefix("## ")?.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let anchor = slugify(&text);
            Some(Heading { text, anchor })
        })
        .collect()
}

struct ListBlock {
    start: usize,
    end: usize,
    items: Vec<String>,
}

/// マーカー統一のリストブロックを 1 つ切り出す（`-`/`*`/`+` または `・`）。
/// 一致しなければ None（このリストの先頭ではない）
fn list_block(lines: &[&str], start: usize) -> Option<ListBlock> {
    let marker = list_item_marker(lines[start].trim())?;
    let mut items = Vec::new();
    let mut end = start;
    while end < lines.len() {
        let trimmed = lines[end].trim();
        if trimmed.is_empty() {
            break;
        }
        let Some(m) = list_item_marker(trimmed) else {
            break;
        };
        if m.marker_char != marker.marker_char {
            break;
        }
        items.push(m.label.to_string());
        end += 1;
    }
    Some(ListBlock { start, end, items })
}

struct ListItem<'a> {
    marker_char: char,
    label: &'a str,
}

/// 行がリスト項目なら (マーカー文字, ラベル文字列) を返す
fn list_item_marker(trimmed: &str) -> Option<ListItem<'_>> {
    if let Some(rest) = trimmed.strip_prefix('・') {
        let label = rest.trim();
        return (!label.is_empty()).then_some(ListItem {
            marker_char: '・',
            label,
        });
    }
    for marker in ['-', '*', '+'] {
        if let Some(rest) = trimmed.strip_prefix(marker)
            && let Some(label) = rest.strip_prefix(' ')
        {
            let label = label.trim();
            if !label.is_empty() {
                return Some(ListItem {
                    marker_char: marker,
                    label,
                });
            }
        }
    }
    None
}

enum ListOutcome {
    /// 全項目が見出しに一致し、選択肢リストへ変換した
    Converted(Vec<String>),
    /// 一致しない項目があった（見つからなかった項目のラベル一覧）
    Unmatched(Vec<String>),
    /// 項目にすでにリンクが含まれている（fmt の対象外）
    HasLinks,
    Empty,
}

fn convert_linkless_choice(block: &ListBlock, headings: &[Heading]) -> ListOutcome {
    if block.items.is_empty() {
        return ListOutcome::Empty;
    }
    if block.items.iter().any(|item| item.contains("](")) {
        return ListOutcome::HasLinks;
    }
    let mut new_lines = Vec::with_capacity(block.items.len());
    let mut missing = Vec::new();
    for item in &block.items {
        let matched = headings
            .iter()
            .find(|h| h.text == *item || h.anchor == slugify(item));
        match matched {
            Some(h) => new_lines.push(format!("- [{item}](#{})", h.anchor)),
            None => missing.push(item.clone()),
        }
    }
    if missing.is_empty() {
        ListOutcome::Converted(new_lines)
    } else {
        ListOutcome::Unmatched(missing)
    }
}

// -------------------------------------------------------------------- fmt-legacy

/// `[NAME ...]` 形式の旧記法コマンド行を検出する。
/// v1 のリンク（`[label](target)`）と誤認しないよう、`]` の直後に
/// 何か（`(target)` 等）が続く場合は対象外にする
fn bracket_command(trimmed: &str) -> Option<(&str, &str)> {
    let inner = trimmed.strip_prefix('[')?;
    let close = inner.find(']')?;
    let content = &inner[..close];
    let after = inner[close + 1..].trim();
    if !after.is_empty() {
        return None;
    }
    let name_len = content
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || *c == '_')
        .count();
    if name_len == 0 {
        return None;
    }
    let (name, rest) = content.split_at(name_len);
    if !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    Some((name, rest.trim()))
}

/// `key=value` トークンから指定キーの値を取り出す（末尾のカンマは無視）
fn param(rest: &str, key: &str) -> Option<String> {
    rest.split_whitespace().find_map(|tok| {
        let (k, v) = tok.split_once('=')?;
        (k == key).then(|| v.trim_end_matches(',').trim().to_string())
    })
}

/// 確定的に変換できる旧記法だけを Edit にする。変換できなければ None を返し、
/// 呼び出し側が `legacy-command` の Diagnostic にフォールバックする
fn try_legacy(name: &str, rest: &str, lines: &[&str], i: usize) -> Option<Edit> {
    match name {
        "SAY" => try_say(rest, lines, i),
        "LABEL" => {
            let label = param(rest, "name")?;
            Some(Edit::replace(i, vec![format!("## {label}")], "fmt-legacy"))
        }
        "JUMP" => {
            let label = param(rest, "label")?;
            let anchor = slugify(&label);
            Some(Edit::replace(
                i,
                vec![format!("[{label}](#{anchor})")],
                "fmt-legacy",
            ))
        }
        "WAIT" => Some(Edit::replace(i, Vec::new(), "fmt-legacy")),
        "ENDING" | "END" => {
            let id = param(rest, "id")?;
            Some(Edit::replace(
                i,
                vec![format!("<!-- ending: {id} -->")],
                "fmt-legacy",
            ))
        }
        "BRANCH" => try_branch(rest, i),
        _ => None,
    }
}

/// `[SAY speaker=X]` + 次の非空行のテキスト → `X: テキスト`
fn try_say(rest: &str, lines: &[&str], i: usize) -> Option<Edit> {
    let speaker = param(rest, "speaker")?;
    let mut j = i + 1;
    while j < lines.len() {
        let l = lines[j].trim();
        if l.is_empty() {
            j += 1;
            continue;
        }
        if l.starts_with('#') || l.starts_with("<!--") || l == "[c]" || bracket_command(l).is_some()
        {
            return None; // 次に来るのがテキストだと確信できない
        }
        return Some(Edit {
            start: i,
            end: j + 1,
            new_lines: vec![format!("{speaker}: {l}")],
            rule_id: "fmt-legacy",
        });
    }
    None
}

/// `[BRANCH choice=a label=x, choice=b label=y]` → リンク付き選択肢リスト
fn try_branch(rest: &str, i: usize) -> Option<Edit> {
    let mut choices = Vec::new();
    let mut labels = Vec::new();
    for tok in rest.split_whitespace() {
        let Some((k, v)) = tok.split_once('=') else {
            continue;
        };
        let v = v.trim_end_matches(',').trim().to_string();
        match k {
            "choice" => choices.push(v),
            "label" => labels.push(v),
            _ => {}
        }
    }
    if choices.is_empty() || choices.len() != labels.len() {
        return None; // 対応が取れない場合は変換しない
    }
    let new_lines = choices
        .iter()
        .zip(labels.iter())
        .map(|(choice, label)| format!("- [{choice}](#{})", slugify(label)))
        .collect();
    Some(Edit {
        start: i,
        end: i + 1,
        new_lines,
        rule_id: "fmt-legacy",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fmt(source: &str) -> FmtResult {
        fmt_str(source, Path::new("scene.md"), None)
    }

    fn rule_ids(result: &FmtResult) -> Vec<&'static str> {
        result.changes.iter().map(|c| c.rule_id).collect()
    }

    // ------------------------------------------------------- 変更不要なもの

    #[test]
    fn 既に正しい記法は変更されない() {
        let source = "---\nid: t\n---\n\n# タイトル\n\nあゆみ: こんにちは。\n";
        let result = fmt(source);
        assert_eq!(result.formatted, source);
        assert!(result.changes.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    // ------------------------------------------------- fmt-missing-frontmatter

    #[test]
    fn front_matterがなければファイル名からid補う() {
        let result = fmt_str("# タイトル\n\n本文。\n", Path::new("spring_001.md"), None);
        assert_eq!(rule_ids(&result), vec!["fmt-missing-frontmatter"]);
        assert!(result.formatted.starts_with("---\nid: spring_001\n---\n\n"));
        assert!(result.formatted.ends_with("# タイトル\n\n本文。\n"));
    }

    // ------------------------------------------------------- fmt-kagi-dialogue

    #[test]
    fn かぎ括弧セリフはコロン記法になる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\nあゆみ「こんにちは。」\n");
        assert_eq!(rule_ids(&result), vec!["fmt-kagi-dialogue"]);
        assert!(result.formatted.contains("あゆみ: こんにちは。\n"));
        assert_eq!(result.changes[0].before, "あゆみ「こんにちは。」");
        assert_eq!(result.changes[0].after, "あゆみ: こんにちは。");
    }

    // ------------------------------------------------------ fmt-paren-dialogue

    #[test]
    fn 宣言済み話者の丸括弧は内心セリフになる() {
        let mut entries = std::collections::BTreeMap::new();
        entries.insert("あゆみ".to_string(), serde_yaml::Value::Null);
        let characters = Characters {
            path: PathBuf::from("characters.yaml"),
            entries,
        };
        let result = fmt_str(
            "---\nid: t\n---\n\n# t\n\nあゆみ（少し照れている）\n",
            Path::new("t.md"),
            Some(&characters),
        );
        assert_eq!(rule_ids(&result), vec!["fmt-paren-dialogue"]);
        assert!(result.formatted.contains("あゆみ: （少し照れている）\n"));
    }

    #[test]
    fn 未宣言話者の丸括弧は変換されない() {
        let result = fmt("---\nid: t\n---\n\n# t\n\nあゆみ（少し照れている）\n");
        assert!(result.changes.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    // ------------------------------------------------------ fmt-linkless-choice

    #[test]
    fn 全項目が見出しに一致すれば選択肢リストになる() {
        let result =
            fmt("---\nid: t\n---\n\n# t\n\n・走る\n・歩く\n\n## 走る\n\nA\n\n## 歩く\n\nB\n");
        assert_eq!(rule_ids(&result), vec!["fmt-linkless-choice"]);
        assert!(
            result
                .formatted
                .contains("- [走る](#走る)\n- [歩く](#歩く)\n")
        );
    }

    #[test]
    fn 一致しない項目があれば変換せず警告になる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n・走る\n・歩く\n\n## 走る\n\nA\n");
        assert!(result.changes.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].rule_id, "fmt-linkless-choice");
        assert!(result.diagnostics[0].message.contains("歩く"));
    }

    #[test]
    fn リンク済みのリストは対象外() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n- [走る](#a)\n- 歩く\n");
        assert!(result.changes.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    // -------------------------------------------------------------- fmt-legacy

    #[test]
    fn sayコマンドと次行の本文が結合される() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n[SAY speaker=あゆみ]\nこんにちは。\n");
        assert_eq!(rule_ids(&result), vec!["fmt-legacy"]);
        assert!(result.formatted.contains("あゆみ: こんにちは。\n"));
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn labelはh2見出しになる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n[LABEL name=go_left]\n");
        assert_eq!(rule_ids(&result), vec!["fmt-legacy"]);
        assert!(result.formatted.contains("## go_left\n"));
    }

    #[test]
    fn jumpはリンクだけの段落になる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n[JUMP label=end]\n");
        assert_eq!(rule_ids(&result), vec!["fmt-legacy"]);
        assert!(result.formatted.contains("[end](#end)\n"));
    }

    #[test]
    fn waitと開き括弧のcは削除される() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n本文。\n\n[WAIT 1.0s]\n\n[c]\n");
        assert_eq!(rule_ids(&result), vec!["fmt-legacy", "fmt-legacy"]);
        assert!(!result.formatted.contains("WAIT"));
        assert!(!result.formatted.contains("[c]"));
    }

    #[test]
    fn endingとendはコメント記法になる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n[ENDING id=good_end]\n");
        assert_eq!(rule_ids(&result), vec!["fmt-legacy"]);
        assert!(result.formatted.contains("<!-- ending: good_end -->\n"));
    }

    #[test]
    fn branchは選択肢リストに展開される() {
        let result = fmt(
            "---\nid: t\n---\n\n# t\n\n[BRANCH choice=Left label=go_left, choice=Right label=go_right]\n",
        );
        assert_eq!(rule_ids(&result), vec!["fmt-legacy"]);
        assert!(
            result
                .formatted
                .contains("- [Left](#go_left)\n- [Right](#go_right)\n")
        );
    }

    #[test]
    fn setは変換せずlegacy_command診断になる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n[SET name=flag value=1]\n");
        assert!(result.changes.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].rule_id, "legacy-command");
    }

    #[test]
    fn fenced旧記法は変換せずlegacy_command診断になる() {
        let result = fmt("---\nid: t\n---\n\n# t\n\n:::choices\n- 走る @run\n:::\n");
        assert!(result.changes.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.rule_id == "legacy-command")
        );
    }

    // ------------------------------------------------------------ 実物サンプル

    #[test]
    fn branch_mdフィクスチャ全体を変換できる() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/branch.md");
        let result = fmt_path(&path);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert!(result.formatted.starts_with("---\nid: branch\n---\n\n"));
        assert!(result.formatted.contains("Guide: Choose your path.\n"));
        assert!(
            result
                .formatted
                .contains("- [Left](#go_left)\n- [Right](#go_right)\n")
        );
        assert!(result.formatted.contains("## go_left\n"));
        assert!(result.formatted.contains("Guide: You went left.\n"));
        assert!(result.formatted.contains("[end](#end)\n"));
        assert!(result.formatted.contains("## go_right\n"));
        assert!(result.formatted.contains("Guide: You went right.\n"));
        assert!(result.formatted.contains("## end\n"));
    }

    /// examples/fmt/README.md のレビュー材料。before → fmt の結果が
    /// after と一致することを固定し、変換の決定性を保証する（#86 完了条件）
    #[test]
    fn examples_fmtのbeforeを整形するとafterと一致する() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/fmt");
        let result = fmt_path(&dir.join("before.md"));
        let expected =
            std::fs::read_to_string(dir.join("after.md")).expect("after.md を読み込めない");
        assert_eq!(result.formatted, expected);
    }
}
