//! 基本型定義
//!
//! - [`ast`]   : AST（構文木）の型定義
//! - [`state`] : 実行状態

pub mod ast;
pub mod state;

pub use ast::Ast;
pub use state::State;
