//! Directive type representing "what happens next" in the scenario
//!
//! Directives are data structures that represent the next action to be taken,
//! but are not executed by the engine. They are returned to the caller for processing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents "what happens next" in the scenario
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Directive {
    /// Display spoken text
    Say {
        speaker: Option<String>,
        text: String,
    },
    /// Present a choice to the user
    Choice {
        id: String,
        label: String,
        condition: Option<String>,
    },
    /// Set a flag value
    SetFlag { name: String, value: bool },
    /// Effect hint - just passes through without interpretation
    EffectHint {
        name: String,
        args: HashMap<String, String>,
    },
    /// Warning message with location information
    Warning {
        message: String,
        line: usize,
        column: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_say_creation() {
        let directive = Directive::Say {
            speaker: Some("Alice".to_string()),
            text: "Hello".to_string(),
        };

        match directive {
            Directive::Say { speaker, text } => {
                assert_eq!(speaker, Some("Alice".to_string()));
                assert_eq!(text, "Hello".to_string());
            }
            _ => panic!("Expected Say directive"),
        }
    }

    #[test]
    fn directive_choice_with_condition() {
        let directive = Directive::Choice {
            id: "choice_0".to_string(),
            label: "Go right".to_string(),
            condition: Some("can_go_right".to_string()),
        };

        match directive {
            Directive::Choice {
                id,
                label,
                condition,
            } => {
                assert_eq!(id, "choice_0");
                assert_eq!(label, "Go right");
                assert_eq!(condition, Some("can_go_right".to_string()));
            }
            _ => panic!("Expected Choice directive"),
        }
    }

    #[test]
    fn directive_warning() {
        let directive = Directive::Warning {
            message: "Unused condition".to_string(),
            line: 10,
            column: 5,
        };

        match directive {
            Directive::Warning {
                message,
                line,
                column,
            } => {
                assert_eq!(message, "Unused condition");
                assert_eq!(line, 10);
                assert_eq!(column, 5);
            }
            _ => panic!("Expected Warning directive"),
        }
    }
}
