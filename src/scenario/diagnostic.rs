//! v1 記法パイプライン用の構造化 Diagnostic
//!
//! SPEC.md 6章の Diagnostic ルールに対応する。`file` を持ち、複数ファイル
//! 入力（プロジェクト検査）に対応する。
//! 「Diagnostic は学習教材である」（SPEC 6.1）に従い、どこが（file/span）・
//! なぜ（message）・どう直すか（suggestion または message 内の案内）を持つ。

use serde::Serialize;
use std::path::PathBuf;

/// 深刻度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// エラー：シナリオとして解釈・変換できない
    Error,
    /// 警告：解釈はできるが意図と違う可能性が高い
    Warning,
}

/// ソース上の位置（1-origin）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Span {
    pub line: usize,
    /// 列位置（1-origin）。分かる場合のみ Some（#150）
    pub column: Option<usize>,
}

/// 検出した 1 件の問題
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostic {
    /// SPEC.md 6章のルール ID（例: "broken-link"）
    pub rule_id: &'static str,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub span: Option<Span>,
    pub related_spans: Vec<Span>,
    /// 機械的に適用できる書き換え例。構成できない場合は message 内で案内する
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(
        rule_id: &'static str,
        file: &std::path::Path,
        line: usize,
        message: String,
    ) -> Self {
        Self {
            rule_id,
            severity: Severity::Error,
            message,
            file: file.to_path_buf(),
            span: Some(Span { line, column: None }),
            related_spans: Vec::new(),
            suggestion: None,
        }
    }

    pub fn warning(
        rule_id: &'static str,
        file: &std::path::Path,
        line: usize,
        message: String,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(rule_id, file, line, message)
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_related(mut self, line: usize) -> Self {
        self.related_spans.push(Span { line, column: None });
        self
    }

    /// 主 span に列位置（1-origin）を付与する（#150）
    pub fn with_column(mut self, column: usize) -> Self {
        if let Some(span) = &mut self.span {
            span.column = Some(column);
        }
        self
    }
}
