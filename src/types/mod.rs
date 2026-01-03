//! Core types for the tsumugai library
//!
//! This module contains the fundamental types that form the public API:
//! - AST: Abstract syntax tree representation of parsed scenarios
//! - State: Runtime state including program counter and variables
//! - Event: External events like user choices
//! - Output: Results of step execution
//! - Directive: Represents "what happens next" in the scenario

pub mod ast;
pub mod debug;
pub mod directive;
pub mod display_step;
pub mod event;
pub mod narrative;
pub mod output;
pub mod state;

pub use ast::Ast;
pub use directive::Directive;
pub use display_step::{ChoiceItem, DisplayStep, Effects};
pub use event::Event;
pub use narrative::{ChoiceOption, NarrativeEvent};
pub use output::Output;
pub use state::State;
