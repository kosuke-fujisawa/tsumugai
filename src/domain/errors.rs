//! Domain errors - Business logic errors

use crate::domain::value_objects::{LabelName, VariableName};
use thiserror::Error;

/// Domain-specific errors that represent business rule violations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
    #[error("Invalid command index {index}, maximum is {max}")]
    InvalidCommandIndex { index: usize, max: usize },

    #[error("Undefined label '{label}' referenced at line {line}")]
    UndefinedLabel { label: LabelName, line: usize },

    #[error("Duplicate label '{label}' defined at line {line}")]
    DuplicateLabel { label: LabelName, line: usize },

    #[error("Variable '{variable}' not found")]
    VariableNotFound { variable: VariableName },

    #[error("Type mismatch for variable '{variable}': expected {expected}, got {actual}")]
    VariableTypeMismatch {
        variable: VariableName,
        expected: String,
        actual: String,
    },

    #[error("Invalid scenario: {reason}")]
    InvalidScenario { reason: String },

    #[error("Execution state error: {reason}")]
    ExecutionState { reason: String },

    #[error("Business rule violation: {rule}")]
    BusinessRuleViolation { rule: String },
}

impl DomainError {
    pub fn variable_not_found(variable: impl Into<VariableName>) -> Self {
        Self::VariableNotFound {
            variable: variable.into(),
        }
    }

    pub fn undefined_label(label: impl Into<LabelName>, line: usize) -> Self {
        Self::UndefinedLabel {
            label: label.into(),
            line,
        }
    }

    pub fn invalid_scenario(reason: impl Into<String>) -> Self {
        Self::InvalidScenario {
            reason: reason.into(),
        }
    }

    pub fn execution_state_error(reason: impl Into<String>) -> Self {
        Self::ExecutionState {
            reason: reason.into(),
        }
    }

    pub fn business_rule_violation(rule: impl Into<String>) -> Self {
        Self::BusinessRuleViolation {
            rule: rule.into(),
        }
    }
}