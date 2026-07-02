//! tsumugai — Markdown ノベルシナリオのセマンティックランタイム
//!
//! # モジュール構成
//! - [`scenario`] : v1 記法（SPEC.md）のシーンモデルとパーサー
//! - [`parser`]   : 旧記法（v0）Markdown DSL → AST（#76〜#78 の移行完了後に撤去予定）
//! - [`analyzer`] : AST の静的検証
//! - [`runtime`]  : AST → IR コンパイル + ステップ実行
//! - [`player`]   : CUI プレイヤー（参照実装）
//!
//! # 典型的な使い方
//! ```no_run
//! use tsumugai::{parser, runtime};
//! use tsumugai::types::state::State;
//!
//! let markdown = "...";
//! let ast = parser::parse(markdown).unwrap();
//! let program = runtime::compile(&ast);
//! let (state, output) = runtime::step(State::new(), &program, None);
//! ```

pub mod analyzer;
pub mod parser;
pub mod player;
pub mod runtime;
pub mod scenario;
pub mod types;
