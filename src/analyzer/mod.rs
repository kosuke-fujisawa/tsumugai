//! 静的解析モジュール
//!
//! AST を検査して警告・エラーを返す。
//! 実行せずに問題を検出できることを目的とする。
//!
//! # 検査内容
//! - 未定義ラベルへのジャンプ
//! - 到達不能なラベル
//! - 選択肢に対応するラベルが存在しない

use crate::types::ast::{Ast, AstNode};
use serde::Serialize;
use std::collections::HashSet;

/// 検査レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// エラー：シナリオが正しく動作しない
    Error,
    /// 警告：動作はするが見直しを推奨
    Warning,
    /// 情報：参考情報
    Info,
}

/// ソースコード上の位置（行・列は 1-origin）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

/// 検査で発見した1件の問題（Diagnostic）
///
/// `rule_id` でルール種別を機械的に識別できる。
/// `span` はパーサーが行番号を記録したノードに対して設定される（1-origin）。
/// `suggestion` があれば修正方法をユーザーに提示する。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Issue {
    pub rule_id: &'static str,
    pub level: Level,
    pub message: String,
    pub span: Option<Span>,
    pub related_spans: Vec<Span>,
    pub suggestion: Option<String>,
}

/// `--json` フラグ用の出力型
#[derive(Debug, Serialize)]
pub struct CheckJsonOutput {
    /// "ok" または "error"
    pub status: &'static str,
    pub error_count: usize,
    pub warning_count: usize,
    pub issues: Vec<Issue>,
}

impl CheckJsonOutput {
    /// パースエラーを JSON 出力に変換する
    pub fn parse_error(message: String) -> Self {
        Self {
            status: "error",
            error_count: 1,
            warning_count: 0,
            issues: vec![Issue {
                rule_id: "parse_error",
                level: Level::Error,
                message,
                span: None,
                related_spans: vec![],
                suggestion: None,
            }],
        }
    }
}

impl From<&AnalysisResult> for CheckJsonOutput {
    fn from(result: &AnalysisResult) -> Self {
        Self {
            status: if result.has_errors() { "error" } else { "ok" },
            error_count: result.error_count(),
            warning_count: result.warning_count(),
            issues: result.issues.clone(),
        }
    }
}

impl Issue {
    fn error(rule_id: &'static str, msg: impl Into<String>) -> Self {
        Self {
            rule_id,
            level: Level::Error,
            message: msg.into(),
            span: None,
            related_spans: vec![],
            suggestion: None,
        }
    }

    fn warning(rule_id: &'static str, msg: impl Into<String>) -> Self {
        Self {
            rule_id,
            level: Level::Warning,
            message: msg.into(),
            span: None,
            related_spans: vec![],
            suggestion: None,
        }
    }

    fn info(rule_id: &'static str, msg: impl Into<String>) -> Self {
        Self {
            rule_id,
            level: Level::Info,
            message: msg.into(),
            span: None,
            related_spans: vec![],
            suggestion: None,
        }
    }

    fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    fn with_span(mut self, span: Option<Span>) -> Self {
        self.span = span;
        self
    }
}

/// 解析結果
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub issues: Vec<Issue>,
}

impl AnalysisResult {
    fn new() -> Self {
        Self { issues: Vec::new() }
    }

    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == Level::Error)
    }

    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == Level::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == Level::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == Level::Info)
            .count()
    }
}

/// AST を解析して問題を返す
pub fn analyze(ast: &Ast) -> AnalysisResult {
    let mut result = AnalysisResult::new();
    let defined_labels: HashSet<&str> = ast.labels.keys().map(|s| s.as_str()).collect();

    check_label_references(ast, &defined_labels, &mut result);
    check_reachability(ast, &defined_labels, &mut result);
    check_empty_branches(ast, &mut result);

    result
}

fn node_span(ast: &Ast, idx: usize) -> Option<Span> {
    ast.node_lines
        .get(idx)
        .map(|&line| Span { line, column: 1 })
}

/// ジャンプ先ラベルの存在確認
fn check_label_references(ast: &Ast, defined: &HashSet<&str>, result: &mut AnalysisResult) {
    for (idx, node) in ast.nodes.iter().enumerate() {
        let span = node_span(ast, idx);
        match node {
            AstNode::Jump { label } if !defined.contains(label.as_str()) => {
                result.issues.push(
                    Issue::error(
                        "undefined_label",
                        format!("未定義ラベル '{}' へのジャンプが存在します", label),
                    )
                    .with_span(span)
                    .with_suggestion(format!(
                        "'[LABEL name={}]' を追加するか、ジャンプ先を修正してください",
                        label
                    )),
                );
            }
            AstNode::JumpIf { label, .. } if !defined.contains(label.as_str()) => {
                result.issues.push(
                    Issue::error(
                        "undefined_label",
                        format!("未定義ラベル '{}' への条件ジャンプが存在します", label),
                    )
                    .with_span(span)
                    .with_suggestion(format!(
                        "'[LABEL name={}]' を追加するか、ジャンプ先を修正してください",
                        label
                    )),
                );
            }
            AstNode::Branch { choices } => {
                for choice in choices {
                    if !defined.contains(choice.target.as_str()) {
                        result.issues.push(
                            Issue::error(
                                "undefined_label",
                                format!(
                                    "選択肢 '{}' のジャンプ先ラベル '{}' が存在しません",
                                    choice.label, choice.target
                                ),
                            )
                            .with_span(span.clone())
                            .with_suggestion(format!(
                                "'[LABEL name={}]' を追加するか、選択肢のラベル参照を修正してください",
                                choice.target
                            )),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

/// 到達不能なラベルの検出
fn check_reachability(ast: &Ast, _defined: &HashSet<&str>, result: &mut AnalysisResult) {
    // 参照されているラベルを収集
    let mut referenced: HashSet<&str> = HashSet::new();
    for node in &ast.nodes {
        match node {
            AstNode::Jump { label } => {
                referenced.insert(label.as_str());
            }
            AstNode::JumpIf { label, .. } => {
                referenced.insert(label.as_str());
            }
            AstNode::Branch { choices } => {
                for c in choices {
                    referenced.insert(c.target.as_str());
                }
            }
            _ => {}
        }
    }

    // 定義されているが一度も参照されないラベルは情報として報告
    for (label, &node_idx) in &ast.labels {
        if !referenced.contains(label.as_str()) {
            let span = node_span(ast, node_idx);
            result.issues.push(
                Issue::info(
                    "unreferenced_label",
                    format!("ラベル '{}' はどこからも参照されていません", label),
                )
                .with_span(span),
            );
        }
    }
}

/// 空の選択肢チェック
fn check_empty_branches(ast: &Ast, result: &mut AnalysisResult) {
    for (idx, node) in ast.nodes.iter().enumerate() {
        if let AstNode::Branch { choices } = node {
            let span = node_span(ast, idx);
            if choices.is_empty() {
                result.issues.push(
                    Issue::error("empty_branch", "BRANCH 命令に選択肢が1つもありません")
                        .with_span(span)
                        .with_suggestion(
                            "choice=テキスト label=ラベル名 の形式で選択肢を追加してください",
                        ),
                );
            } else if choices.len() == 1 {
                result.issues.push(
                    Issue::warning(
                        "single_choice_branch",
                        "BRANCH 命令の選択肢が1つしかありません（分岐になっていません）",
                    )
                    .with_span(span),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn 正常なシナリオはクリーン() {
        let md = r#"
[SAY speaker=Alice]
こんにちは。

[BRANCH choice=はい label=yes, choice=いいえ label=no]

[LABEL name=yes]
[SAY speaker=Alice]
よかった！

[LABEL name=no]
[SAY speaker=Alice]
残念。
"#;
        let ast = parse(md).unwrap();
        let result = analyze(&ast);
        assert!(!result.has_errors());
    }

    #[test]
    fn 未定義ラベルはエラー() {
        let md = "[JUMP label=nonexistent]\n";
        // パーサーがラベル検証をするので parse が失敗する
        // ここではパーサーを通すために別の方法でテスト
        // 実際には parser::parse が先にエラーを出す
        assert!(parse(md).is_err());
    }
}
