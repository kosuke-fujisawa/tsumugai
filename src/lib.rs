//! tsumugai — Markdown シナリオ制作 CLI
//!
//! # モジュール構成
//! - [`scenario`] : v1 記法（SPEC.md）のシーンモデル・パーサー・検査・整形
//!
//! # 典型的な使い方
//! ```
//! use tsumugai::scenario;
//!
//! let source = "---\nid: demo\n---\n\n# タイトル\n\n主人公: こんにちは。\n";
//! let parsed = scenario::parse_str(source, std::path::Path::new("demo.md"));
//! assert!(parsed.diagnostics.is_empty());
//! ```

pub mod scenario;
