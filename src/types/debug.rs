//! Debug trace types for the interactive debugger
//!
//! This module provides types for capturing "narrative meaning" events
//! during scenario execution, which the debugger UI consumes to present
//! story progression without exposing internal implementation details
//! (like PC, AST indices, or `__skip_*` labels).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::state::State;

/// A debug trace event representing a meaningful narrative action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebugTraceEvent {
    /// Entered a new scene
    EnterScene { name: String },

    /// Dialogue line spoken
    Dialogue {
        speaker: Option<String>,
        text: String,
        click_wait: bool,
    },

    /// Choices presented to the user
    PresentChoices { choices: Vec<ChoiceItem> },

    /// User selected a choice
    SelectChoice { choice_id: String, label: String },

    /// Variable value changed
    EffectSetVar {
        name: String,
        before: serde_json::Value,
        after: serde_json::Value,
    },

    /// Flag value changed (boolean)
    EffectSetFlag {
        name: String,
        before: bool,
        after: bool,
    },

    /// Jump occurred
    Jump { to: String, reason: JumpReason },

    /// Warning detected during execution
    Warning {
        code: String,
        message: String,
        location: LocationHint,
    },
}

/// A choice option item for presentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChoiceItem {
    pub id: String,
    pub label: String,
    pub condition: Option<String>,
}

/// Reason for a jump (for "why are we here?" display)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JumpReason {
    /// Sequential execution (no explicit jump)
    Sequential,

    /// Jump due to choice selection
    Choice { choice_id: String },

    /// Jump due to when condition
    When { expr: String },

    /// Explicit GOTO command
    Goto,
}

/// Location hint for errors/warnings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocationHint {
    pub scene: Option<String>,
    pub line: Option<usize>,
}

/// Snapshot of state for undo functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub state: State,
    pub scene: Option<String>,
    pub note: String,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(state: State, scene: Option<String>, note: String) -> Self {
        Self { state, scene, note }
    }
}

/// Debugger state managing history and scene navigation
#[derive(Debug, Clone)]
pub struct DebuggerState {
    /// Current runtime state
    pub state: State,

    /// History of snapshots for undo
    pub history: Vec<Snapshot>,

    /// Scene name to AST index mapping
    pub scene_index: HashMap<String, usize>,

    /// Current scene name (for display)
    pub current_scene: Option<String>,

    /// Last jump reason (for "why are we here?" display)
    pub last_reason: Option<JumpReason>,

    /// Pending choices (when waiting for user selection)
    pub pending_choices: Vec<ChoiceItem>,
}

impl DebuggerState {
    /// Create a new debugger state
    pub fn new(state: State, scene_index: HashMap<String, usize>) -> Self {
        Self {
            state,
            history: Vec::new(),
            scene_index,
            current_scene: None,
            last_reason: None,
            pending_choices: Vec::new(),
        }
    }

    /// Take a snapshot of the current state
    pub fn snapshot(&mut self, note: String) {
        let snapshot = Snapshot::new(self.state.clone(), self.current_scene.clone(), note);
        self.history.push(snapshot);
    }

    /// Restore from the last snapshot (undo)
    pub fn restore_snapshot(&mut self) -> Result<(), String> {
        if let Some(snapshot) = self.history.pop() {
            self.state = snapshot.state;
            self.current_scene = snapshot.scene;
            Ok(())
        } else {
            Err("No snapshot to restore".to_string())
        }
    }

    /// Jump to a scene by name
    pub fn jump_to_scene(&mut self, scene_name: &str) -> Result<(), String> {
        if let Some(&pc) = self.scene_index.get(scene_name) {
            self.state.pc = pc;
            self.current_scene = Some(scene_name.to_string());
            Ok(())
        } else {
            Err(format!("Scene '{}' not found", scene_name))
        }
    }

    /// Get list of all scene names
    pub fn get_scenes(&self) -> Vec<String> {
        let mut scenes: Vec<_> = self.scene_index.keys().cloned().collect();
        scenes.sort();
        scenes
    }
}
