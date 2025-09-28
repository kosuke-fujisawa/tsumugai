//! Core types for the tsumugai library
//!
//! This module contains the fundamental types that form the public API:
//! - AST: Abstract syntax tree representation of parsed scenarios
//! - State: Runtime state including program counter and variables
//! - Event: External events like user choices
//! - Output: Results of step execution

pub mod ast;
pub mod event;
pub mod output;
pub mod state;

pub use ast::Ast;
pub use event::Event;
pub use output::Output;
pub use state::State;
