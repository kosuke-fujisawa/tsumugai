//! Output from runtime execution

use serde::{Deserialize, Serialize};

/// Result of a single step execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Output {
    /// Lines of dialogue or text to display
    pub lines: Vec<Line>,
    /// Available choices for user selection
    pub choices: Vec<Choice>,
    /// Effect tags for UI layer (sound, images, etc.)
    pub effects: Vec<EffectTag>,
    /// Whether the state can be saved at this point
    pub can_save: bool,
}

impl Output {
    /// Create empty output
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            choices: Vec::new(),
            effects: Vec::new(),
            can_save: true,
        }
    }

    /// Add a line of dialogue
    pub fn add_line(&mut self, speaker: Option<String>, text: String) {
        self.lines.push(Line { speaker, text });
    }

    /// Add a choice option
    pub fn add_choice(&mut self, id: String, label: String) {
        self.choices.push(Choice { id, label });
    }

    /// Add an effect tag
    pub fn add_effect(&mut self, tag: String, opts: Option<serde_json::Value>) {
        self.effects.push(EffectTag { tag, opts });
    }

    /// Check if there are any choices available
    pub fn has_choices(&self) -> bool {
        !self.choices.is_empty()
    }

    /// Check if there are any lines to display
    pub fn has_lines(&self) -> bool {
        !self.lines.is_empty()
    }

    /// Check if there are any effects
    pub fn has_effects(&self) -> bool {
        !self.effects.is_empty()
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

/// A line of dialogue or narrative text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Line {
    /// Speaker name (None for narrative text)
    pub speaker: Option<String>,
    /// The text content
    pub text: String,
}

/// A choice option for user selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    /// Unique identifier for this choice
    pub id: String,
    /// Display text for the choice
    pub label: String,
}

/// An effect tag for the UI layer to handle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectTag {
    /// Effect type identifier
    pub tag: String,
    /// Optional parameters for the effect
    pub opts: Option<serde_json::Value>,
}
