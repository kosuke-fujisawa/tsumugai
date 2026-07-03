//! v1 記法（SPEC.md）のシーンモデル・パーサー・検査
//!
//! 一般 Markdown 準拠のシナリオ記法 v1 を解析して [`Scene`] を構築し
//! （[`parse_str`] / [`parse_file`]）、プロジェクト全体の意味論検査
//! （[`check_path`]）と検査結果の出力（[`render_human`] / [`render_json`] /
//! [`render_sarif`]）を提供する。
//! 旧 `parser`（括弧コマンド記法）は runtime の移行完了（#77〜#78）
//! まで並存し、その後撤去される。
//!
//! # 設計方針（SPEC 6.1）
//! - パースはエラーで中断しない。解釈できた範囲の [`Scene`] と、
//!   検出したすべての [`Diagnostic`] を常に両方返す
//! - 位置情報（行番号）を全ブロック・全 Diagnostic に持たせる
//!
//! # 典型的な使い方
//! ```
//! use tsumugai::scenario;
//!
//! let source = "---\nid: demo\n---\n\n# タイトル\n\n主人公: こんにちは。\n";
//! let parsed = scenario::parse_str(source, std::path::Path::new("demo.md"));
//! assert!(parsed.diagnostics.is_empty());
//! assert_eq!(parsed.scene.id.as_deref(), Some("demo"));
//! ```

mod anchor;
mod characters;
mod check;
mod diagnostic;
mod parse;
mod project;
mod report;
#[cfg(test)]
mod tests;
mod trace;

pub use anchor::{percent_decode, slugify};
pub use characters::{Characters, find_characters_file, load_characters};
pub use check::{CheckOptions, CheckResult, check_path};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use parse::{FrontMatterSpans, Parsed, parse_file, parse_str};
pub use report::{render_human, render_json, render_sarif, render_trace_human, render_trace_json};
pub use trace::{Trace, TraceChoice, TraceEnd, TraceOptions, TraceResult, TraceStep, trace_path};

use serde::Serialize;
use std::path::PathBuf;

/// 1 ファイル = 1 シーン（SPEC 3章）
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Scene {
    /// シナリオファイルのパス
    pub path: PathBuf,
    /// front matter の `id`（欠落時は None + missing-scene-id）
    pub id: Option<String>,
    /// H1 の表示用タイトル
    pub title: Option<String>,
    /// front matter の `background`（ファイルからの相対パス）
    pub background: Option<String>,
    /// front matter の `bgm`（ファイルからの相対パス）
    pub bgm: Option<String>,
    /// front matter 直後から最初の H2 までのリード部
    pub lead: Vec<Block>,
    /// H2 セクション（分岐先）
    pub sections: Vec<Section>,
}

/// H2 で区切られたセクション（SPEC 3.2）
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    /// 見出しテキスト
    pub heading: String,
    /// 導出したアンカー名（SPEC 3.2 の slug 規則）
    pub anchor: String,
    /// 見出しの行番号（1-origin）
    pub line: usize,
    pub blocks: Vec<Block>,
}

/// 本文ブロック（SPEC 4章）
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// 通常の段落（SPEC 4.1）
    Narration { text: String, line: usize },
    /// `名前: 本文` 形式の段落（SPEC 4.2）
    Dialogue {
        speaker: String,
        text: String,
        line: usize,
    },
    /// リンクのみを項目とするリスト（SPEC 4.3）。ここで入力待ちになる
    Choices { items: Vec<ChoiceItem>, line: usize },
    /// リンク 1 つだけの段落（SPEC 4.4）
    Jump {
        label: String,
        target: LinkTarget,
        line: usize,
    },
    /// `<!-- ending: id -->`（SPEC 4.5）。ここで実行終了
    Ending { id: String, line: usize },
}

/// 選択肢 1 項目
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChoiceItem {
    pub label: String,
    pub target: LinkTarget,
    pub line: usize,
}

/// 選択肢・ジャンプの飛び先（SPEC 4.3）
///
/// `#anchor` / `file.md` / `file.md#anchor` の 3 形式。
/// `file` が None なら同一ファイル内、`anchor` が None なら
/// 参照先ファイルのリード部先頭（front matter 直後）に着地する。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LinkTarget {
    pub file: Option<String>,
    pub anchor: Option<String>,
}
