//! Public API types - External contracts for tsumugai library
//!
//! This module contains all public types that external users depend on.
//! These types form the stable contract and should be changed with care.

use serde::{Deserialize, Serialize};

/// Next action that the host should take after processing directives
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NextAction {
    /// Continue to next step immediately
    Next,
    /// Wait for user input (Enter key)
    WaitUser,
    /// Wait for user to choose from options
    WaitBranch,
    /// Story execution is finished
    Halt,
}

/// A single directive to execute (play sound, show text, etc.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "args")]
#[non_exhaustive]
pub enum Directive {
    /// Display spoken text
    Say { speaker: String, text: String },
    /// Show image on specified layer
    ShowImage { layer: String, path: Option<String> },
    /// Play background music
    PlayBgm { path: Option<String> },
    /// Wait for specified duration
    Wait { seconds: f32 },
    /// Present choices to user
    Branch { choices: Vec<String> },
    /// Clear specified image layer
    ClearLayer { layer: String },
    /// Set game variable
    SetVar { name: String, value: String },
    /// Jump to label marker
    JumpTo { label: String },
    /// Play sound effect
    PlaySe { path: Option<String> },
    /// Play movie/video
    PlayMovie { path: Option<String> },
    /// Label reached (for navigation tracking)
    ReachedLabel { label: String },
}

/// Result of a single step execution
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StepResult {
    /// What action the host should take next
    pub next: NextAction,
    /// Directives to execute in order
    pub directives: Vec<Directive>,
}

/// Error types returned by the public API
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    /// Parse error with location information
    #[error("parse error at {line}:{column}: {message}")]
    Parse {
        line: usize,
        column: usize,
        message: String,
    },
    /// Invalid operation or state
    #[error("invalid operation: {0}")]
    Invalid(String),
    /// Engine execution error
    #[error("engine error: {0}")]
    Engine(String),
    /// I/O related error
    #[error("I/O error: {0}")]
    Io(String),
}

impl ApiError {
    /// Create a parse error with location
    pub fn parse(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            column,
            message: message.into(),
        }
    }

    /// Create an invalid operation error
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    /// Create an engine error
    pub fn engine(message: impl Into<String>) -> Self {
        Self::Engine(message.into())
    }

    /// Create an I/O error
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }
}