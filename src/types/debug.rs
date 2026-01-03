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
        let snapshot = Snapshot::new(
            self.state.clone(),
            self.current_scene.clone(),
            note,
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    // Snapshot tests
    #[test]
    fn test_snapshot_creation() {
        let state = State::new();
        let snapshot = Snapshot::new(state.clone(), Some("scene1".to_string()), "Test snapshot".to_string());
        
        assert_eq!(snapshot.state.pc, state.pc);
        assert_eq!(snapshot.scene, Some("scene1".to_string()));
        assert_eq!(snapshot.note, "Test snapshot");
    }

    #[test]
    fn test_snapshot_without_scene() {
        let state = State::new();
        let snapshot = Snapshot::new(state.clone(), None, "Initial state".to_string());
        
        assert_eq!(snapshot.scene, None);
        assert_eq!(snapshot.note, "Initial state");
    }

    // DebuggerState tests
    #[test]
    fn test_debugger_state_creation() {
        let state = State::new();
        let mut scene_index = HashMap::new();
        scene_index.insert("intro".to_string(), 0);
        scene_index.insert("chapter1".to_string(), 10);
        
        let debugger = DebuggerState::new(state, scene_index.clone());
        
        assert_eq!(debugger.state.pc, 0);
        assert_eq!(debugger.history.len(), 0);
        assert_eq!(debugger.scene_index.len(), 2);
        assert_eq!(debugger.current_scene, None);
        assert_eq!(debugger.last_reason, None);
        assert_eq!(debugger.pending_choices.len(), 0);
    }

    #[test]
    fn test_debugger_snapshot() {
        let state = State::new();
        let scene_index = HashMap::new();
        let mut debugger = DebuggerState::new(state, scene_index);
        
        debugger.snapshot("First snapshot".to_string());
        assert_eq!(debugger.history.len(), 1);
        
        debugger.snapshot("Second snapshot".to_string());
        assert_eq!(debugger.history.len(), 2);
    }

    #[test]
    fn test_debugger_restore_snapshot() {
        let mut state = State::new();
        state.pc = 5;
        
        let scene_index = HashMap::new();
        let mut debugger = DebuggerState::new(state, scene_index);
        
        // Take snapshot at pc=5
        debugger.snapshot("At pc=5".to_string());
        
        // Advance state
        debugger.state.pc = 10;
        assert_eq!(debugger.state.pc, 10);
        
        // Restore
        let result = debugger.restore_snapshot();
        assert!(result.is_ok());
        assert_eq!(debugger.state.pc, 5);
    }

    #[test]
    fn test_debugger_restore_snapshot_empty() {
        let state = State::new();
        let scene_index = HashMap::new();
        let mut debugger = DebuggerState::new(state, scene_index);
        
        let result = debugger.restore_snapshot();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No snapshot to restore");
    }

    #[test]
    fn test_debugger_jump_to_scene() {
        let state = State::new();
        let mut scene_index = HashMap::new();
        scene_index.insert("intro".to_string(), 0);
        scene_index.insert("chapter1".to_string(), 10);
        scene_index.insert("ending".to_string(), 20);
        
        let mut debugger = DebuggerState::new(state, scene_index);
        
        let result = debugger.jump_to_scene("chapter1");
        assert!(result.is_ok());
        assert_eq!(debugger.state.pc, 10);
        assert_eq!(debugger.current_scene, Some("chapter1".to_string()));
    }

    #[test]
    fn test_debugger_jump_to_nonexistent_scene() {
        let state = State::new();
        let scene_index = HashMap::new();
        let mut debugger = DebuggerState::new(state, scene_index);
        
        let result = debugger.jump_to_scene("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_debugger_get_scenes() {
        let state = State::new();
        let mut scene_index = HashMap::new();
        scene_index.insert("chapter3".to_string(), 30);
        scene_index.insert("chapter1".to_string(), 10);
        scene_index.insert("intro".to_string(), 0);
        
        let debugger = DebuggerState::new(state, scene_index);
        let scenes = debugger.get_scenes();
        
        // Should be sorted alphabetically
        assert_eq!(scenes.len(), 3);
        assert_eq!(scenes[0], "chapter1");
        assert_eq!(scenes[1], "chapter3");
        assert_eq!(scenes[2], "intro");
    }

    #[test]
    fn test_debugger_get_scenes_empty() {
        let state = State::new();
        let scene_index = HashMap::new();
        let debugger = DebuggerState::new(state, scene_index);
        
        let scenes = debugger.get_scenes();
        assert_eq!(scenes.len(), 0);
    }

    // DebugTraceEvent tests
    #[test]
    fn test_debug_trace_event_enter_scene() {
        let event = DebugTraceEvent::EnterScene {
            name: "chapter1".to_string(),
        };
        
        match event {
            DebugTraceEvent::EnterScene { name } => {
                assert_eq!(name, "chapter1");
            }
            _ => panic!("Expected EnterScene variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_dialogue() {
        let event = DebugTraceEvent::Dialogue {
            speaker: Some("Alice".to_string()),
            text: "Hello!".to_string(),
            click_wait: true,
        };
        
        match event {
            DebugTraceEvent::Dialogue { speaker, text, click_wait } => {
                assert_eq!(speaker, Some("Alice".to_string()));
                assert_eq!(text, "Hello!");
                assert!(click_wait);
            }
            _ => panic!("Expected Dialogue variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_present_choices() {
        let choices = vec![
            ChoiceItem {
                id: "choice_0".to_string(),
                label: "Yes".to_string(),
                condition: None,
            },
        ];
        
        let event = DebugTraceEvent::PresentChoices {
            choices: choices.clone(),
        };
        
        match event {
            DebugTraceEvent::PresentChoices { choices: c } => {
                assert_eq!(c.len(), 1);
                assert_eq!(c[0].id, "choice_0");
            }
            _ => panic!("Expected PresentChoices variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_select_choice() {
        let event = DebugTraceEvent::SelectChoice {
            choice_id: "choice_0".to_string(),
            label: "Attack".to_string(),
        };
        
        match event {
            DebugTraceEvent::SelectChoice { choice_id, label } => {
                assert_eq!(choice_id, "choice_0");
                assert_eq!(label, "Attack");
            }
            _ => panic!("Expected SelectChoice variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_effect_set_var() {
        let event = DebugTraceEvent::EffectSetVar {
            name: "score".to_string(),
            before: serde_json::json!(100),
            after: serde_json::json!(150),
        };
        
        match event {
            DebugTraceEvent::EffectSetVar { name, before, after } => {
                assert_eq!(name, "score");
                assert_eq!(before, serde_json::json!(100));
                assert_eq!(after, serde_json::json!(150));
            }
            _ => panic!("Expected EffectSetVar variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_effect_set_flag() {
        let event = DebugTraceEvent::EffectSetFlag {
            name: "found_key".to_string(),
            before: false,
            after: true,
        };
        
        match event {
            DebugTraceEvent::EffectSetFlag { name, before, after } => {
                assert_eq!(name, "found_key");
                assert!(!before);
                assert!(after);
            }
            _ => panic!("Expected EffectSetFlag variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_jump() {
        let event = DebugTraceEvent::Jump {
            to: "ending".to_string(),
            reason: JumpReason::Goto,
        };
        
        match event {
            DebugTraceEvent::Jump { to, reason } => {
                assert_eq!(to, "ending");
                assert_eq!(reason, JumpReason::Goto);
            }
            _ => panic!("Expected Jump variant"),
        }
    }

    #[test]
    fn test_debug_trace_event_warning() {
        let event = DebugTraceEvent::Warning {
            code: "W001".to_string(),
            message: "Undefined variable".to_string(),
            location: LocationHint {
                scene: Some("chapter1".to_string()),
                line: Some(42),
            },
        };
        
        match event {
            DebugTraceEvent::Warning { code, message, location } => {
                assert_eq!(code, "W001");
                assert_eq!(message, "Undefined variable");
                assert_eq!(location.scene, Some("chapter1".to_string()));
                assert_eq!(location.line, Some(42));
            }
            _ => panic!("Expected Warning variant"),
        }
    }

    // JumpReason tests
    #[test]
    fn test_jump_reason_sequential() {
        let reason = JumpReason::Sequential;
        assert_eq!(reason, JumpReason::Sequential);
    }

    #[test]
    fn test_jump_reason_choice() {
        let reason = JumpReason::Choice {
            choice_id: "choice_0".to_string(),
        };
        
        match reason {
            JumpReason::Choice { choice_id } => {
                assert_eq!(choice_id, "choice_0");
            }
            _ => panic!("Expected Choice variant"),
        }
    }

    #[test]
    fn test_jump_reason_when() {
        let reason = JumpReason::When {
            expr: "flag == true".to_string(),
        };
        
        match reason {
            JumpReason::When { expr } => {
                assert_eq!(expr, "flag == true");
            }
            _ => panic!("Expected When variant"),
        }
    }

    #[test]
    fn test_jump_reason_goto() {
        let reason = JumpReason::Goto;
        assert_eq!(reason, JumpReason::Goto);
    }

    // LocationHint tests
    #[test]
    fn test_location_hint_with_scene_and_line() {
        let location = LocationHint {
            scene: Some("intro".to_string()),
            line: Some(10),
        };
        
        assert_eq!(location.scene, Some("intro".to_string()));
        assert_eq!(location.line, Some(10));
    }

    #[test]
    fn test_location_hint_without_scene() {
        let location = LocationHint {
            scene: None,
            line: Some(5),
        };
        
        assert_eq!(location.scene, None);
        assert_eq!(location.line, Some(5));
    }

    #[test]
    fn test_location_hint_without_line() {
        let location = LocationHint {
            scene: Some("ending".to_string()),
            line: None,
        };
        
        assert_eq!(location.scene, Some("ending".to_string()));
        assert_eq!(location.line, None);
    }

    // ChoiceItem tests (debug module)
    #[test]
    fn test_choice_item_with_condition() {
        let choice = ChoiceItem {
            id: "choice_0".to_string(),
            label: "Use key".to_string(),
            condition: Some("has_key == true".to_string()),
        };
        
        assert_eq!(choice.id, "choice_0");
        assert_eq!(choice.label, "Use key");
        assert_eq!(choice.condition, Some("has_key == true".to_string()));
    }

    #[test]
    fn test_choice_item_without_condition() {
        let choice = ChoiceItem {
            id: "choice_1".to_string(),
            label: "Continue".to_string(),
            condition: None,
        };
        
        assert_eq!(choice.condition, None);
    }

    // Serialization tests
    #[test]
    fn test_debug_trace_event_serialization() {
        let event = DebugTraceEvent::EnterScene {
            name: "test".to_string(),
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: DebugTraceEvent = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_jump_reason_serialization() {
        let reason = JumpReason::Choice {
            choice_id: "choice_0".to_string(),
        };
        
        let serialized = serde_json::to_string(&reason).unwrap();
        let deserialized: JumpReason = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(reason, deserialized);
    }

    #[test]
    fn test_snapshot_serialization() {
        let state = State::new();
        let snapshot = Snapshot::new(state, Some("scene1".to_string()), "Test".to_string());
        
        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: Snapshot = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(snapshot.scene, deserialized.scene);
        assert_eq!(snapshot.note, deserialized.note);
    }
}
