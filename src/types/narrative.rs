//! Narrative event types for player mode
//!
//! This module provides types for converting runtime output into
//! human-readable narrative events suitable for CUI player mode.

use serde::{Deserialize, Serialize};

/// A narrative event representing what should be presented to the player
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NarrativeEvent {
    /// Dialogue or narration text
    Dialogue {
        speaker: Option<String>,
        text: String,
    },
    /// Player choices
    Choices { choices: Vec<ChoiceOption> },
    /// Visual or audio effect (images, BGM, SE, etc.)
    Effect {
        kind: String,
        data: Option<serde_json::Value>,
    },
    /// End of scenario
    End,
}

/// A single choice option
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChoiceOption {
    /// Unique identifier for this choice (e.g., "choice_0")
    pub id: String,
    /// Display text for the choice
    pub label: String,
}

impl NarrativeEvent {
    /// Create a dialogue event
    pub fn dialogue(speaker: Option<String>, text: String) -> Self {
        Self::Dialogue { speaker, text }
    }

    /// Create a choices event
    pub fn choices(choices: Vec<ChoiceOption>) -> Self {
        Self::Choices { choices }
    }

    /// Create an effect event
    pub fn effect(kind: String, data: Option<serde_json::Value>) -> Self {
        Self::Effect { kind, data }
    }

    /// Create an end event
    pub fn end() -> Self {
        Self::End
    }
}
