//! Public contracts - Stable interfaces for external consumers
//! 
//! This module defines the contracts that external users of the library
//! depend on. These should remain stable across versions.

use crate::domain::{services::ExecutionDirective, value_objects::Choice};

/// Result of a story execution step - the main contract for library users
#[derive(Debug, Clone, PartialEq)]
pub enum StepResult {
    /// Continue immediately with the given directive
    Continue(ExecutionDirective),
    /// Wait for user input after showing the directive
    WaitForUser(ExecutionDirective),
    /// Wait for user to choose from the given options
    WaitForChoice(Vec<Choice>),
    /// Story execution is finished
    Finished,
}

/// Next action type for step results
#[derive(Debug, Clone, PartialEq)]
pub enum NextAction {
    /// Continue to next step immediately
    Continue,
    /// Wait for user input
    WaitForUser,
    /// Wait for user choice selection
    WaitForChoice,
    /// Story is finished
    Finished,
}

/// Directive list for a step
#[derive(Debug, Clone, PartialEq)]
pub struct StepDirectives {
    /// The primary action directive
    pub next_action: NextAction,
    /// Execution directives to process
    pub directives: Vec<ExecutionDirective>,
    /// Available choices (if next_action is WaitForChoice)
    pub choices: Option<Vec<Choice>>,
}

impl StepResult {
    /// Get the next action type
    pub fn next_action(&self) -> NextAction {
        match self {
            StepResult::Continue(_) => NextAction::Continue,
            StepResult::WaitForUser(_) => NextAction::WaitForUser,
            StepResult::WaitForChoice(_) => NextAction::WaitForChoice,
            StepResult::Finished => NextAction::Finished,
        }
    }

    /// Extract directives from the step result
    pub fn directives(&self) -> Vec<ExecutionDirective> {
        match self {
            StepResult::Continue(directive) => vec![directive.clone()],
            StepResult::WaitForUser(directive) => vec![directive.clone()],
            StepResult::WaitForChoice(_) => vec![],
            StepResult::Finished => vec![],
        }
    }

    /// Extract choices if this is a choice step
    pub fn choices(&self) -> Option<&Vec<Choice>> {
        match self {
            StepResult::WaitForChoice(choices) => Some(choices),
            _ => None,
        }
    }

    /// Convert to structured directive format
    pub fn to_step_directives(&self) -> StepDirectives {
        StepDirectives {
            next_action: self.next_action(),
            directives: self.directives(),
            choices: self.choices().cloned(),
        }
    }
}

/// Error types that can be returned to library users
#[derive(Debug, thiserror::Error)]
pub enum StoryEngineError {
    #[error("Domain error: {message}")]
    Domain { message: String },

    #[error("Repository error: {message}")]
    Repository { message: String },

    #[error("Parsing error: {message}")]
    Parsing { message: String },

    #[error("Invalid path '{path}': {reason}")]
    InvalidPath {
        path: std::path::PathBuf,
        reason: String,
    },

    #[error("IO error: {message}")]
    Io { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

impl StoryEngineError {
    /// Create a domain error
    pub fn domain(message: impl Into<String>) -> Self {
        Self::Domain { message: message.into() }
    }

    /// Create a repository error
    pub fn repository(message: impl Into<String>) -> Self {
        Self::Repository { message: message.into() }
    }

    /// Create a parsing error
    pub fn parsing(message: impl Into<String>) -> Self {
        Self::Parsing { message: message.into() }
    }

    /// Create an IO error
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io { message: message.into() }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration { message: message.into() }
    }
}