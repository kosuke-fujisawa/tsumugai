//! CUI player implementation for tsumugai
//!
//! This module provides a state history manager and player logic
//! for running scenarios in a terminal interface.

use crate::{
    runtime,
    types::{display_step::DisplayStep, display_step::Effects, event::Event, Ast, State},
};

/// Manages state history for undo functionality
#[derive(Debug, Clone)]
pub struct StateHistory {
    /// Stack of previous states
    history: Vec<State>,
    /// Maximum number of states to keep
    max_size: usize,
}

impl StateHistory {
    /// Create a new state history with a maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            history: Vec::new(),
            max_size,
        }
    }

    /// Push a state snapshot onto the history
    pub fn push(&mut self, state: State) {
        self.history.push(state);

        // Keep only the most recent states
        if self.history.len() > self.max_size {
            self.history.remove(0);
        }
    }

    /// Pop the most recent state from history
    ///
    /// Returns None if history is empty
    pub fn pop(&mut self) -> Option<State> {
        self.history.pop()
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Get the current history depth
    pub fn depth(&self) -> usize {
        self.history.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.history.clear();
    }
}

impl Default for StateHistory {
    fn default() -> Self {
        // Default to keeping last 100 states
        Self::new(100)
    }
}

/// Result of advancing the player session
#[derive(Debug, Clone)]
pub enum PlayerResult {
    /// A display step to show to the user, with effects
    Step {
        display_step: DisplayStep,
        effects: Effects,
    },
    /// Scenario has ended
    Ended,
}

/// A player session for running a scenario
pub struct PlayerSession {
    ast: Ast,
    state: State,
    history: StateHistory,
    display_history: Vec<(Option<DisplayStep>, Effects)>,
}

impl PlayerSession {
    /// Create a new player session
    pub fn new(ast: Ast) -> Self {
        Self {
            ast,
            state: State::new(),
            history: StateHistory::default(),
            display_history: vec![(None, Effects::new())], // Start with empty initial state
        }
    }

    /// Advance to the next DisplayStep
    ///
    /// This will execute until the next meaningful display unit
    pub fn next(&mut self) -> PlayerResult {
        // Save current state to history before advancing
        self.history.push(self.state.clone());

        // Execute to next display step
        let (new_state, display_step, effects) =
            runtime::step_to_next_display(self.state.clone(), &self.ast, None);
        self.state = new_state;

        // Check if we got a display step
        match display_step {
            Some(step) => {
                // Save to history
                self.display_history.push((Some(step.clone()), effects.clone()));
                PlayerResult::Step {
                    display_step: step,
                    effects,
                }
            }
            None => {
                // No display step means we've reached the end
                PlayerResult::Ended
            }
        }
    }

    /// Make a choice
    ///
    /// This should be called when the player selects a choice
    pub fn choose(&mut self, choice_index: usize) -> PlayerResult {
        // Save current state to history before advancing
        self.history.push(self.state.clone());

        // Create choice event
        let event = Event::Choice {
            id: format!("choice_{}", choice_index),
        };

        // Execute to next display step with choice event
        let (new_state, display_step, effects) =
            runtime::step_to_next_display(self.state.clone(), &self.ast, Some(event));
        self.state = new_state;

        // Check if we got a display step
        match display_step {
            Some(step) => {
                // Save to history
                self.display_history.push((Some(step.clone()), effects.clone()));
                PlayerResult::Step {
                    display_step: step,
                    effects,
                }
            }
            None => {
                // No display step means we've reached the end
                PlayerResult::Ended
            }
        }
    }

    /// Undo the last action
    ///
    /// Returns the display step from the restored state, or None if no history available
    pub fn undo(&mut self) -> Option<(Option<DisplayStep>, Effects)> {
        if let Some(previous_state) = self.history.pop() {
            self.state = previous_state;
            // Pop the current display (we don't need it anymore)
            self.display_history.pop();
            // Return the display from the restored state
            self.display_history.last().cloned()
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Check if the scenario has ended
    pub fn is_ended(&self) -> bool {
        self.state.pc >= self.ast.len()
    }

    /// Get the current state (for debugging)
    pub fn current_state(&self) -> &State {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ast::AstNode;
    use std::collections::HashMap;

    #[test]
    fn test_push_and_pop() {
        let mut history = StateHistory::new(10);
        let state1 = State::new();
        let mut state2 = State::new();
        state2.pc = 5;

        history.push(state1.clone());
        history.push(state2.clone());

        assert_eq!(history.depth(), 2);
        assert!(history.can_undo());

        let popped = history.pop().unwrap();
        assert_eq!(popped.pc, 5);

        let popped = history.pop().unwrap();
        assert_eq!(popped.pc, 0);

        assert!(!history.can_undo());
    }

    #[test]
    fn test_max_size() {
        let mut history = StateHistory::new(3);

        for i in 0..5 {
            let mut state = State::new();
            state.pc = i;
            history.push(state);
        }

        // Should only keep the last 3
        assert_eq!(history.depth(), 3);

        let popped = history.pop().unwrap();
        assert_eq!(popped.pc, 4);

        let popped = history.pop().unwrap();
        assert_eq!(popped.pc, 3);

        let popped = history.pop().unwrap();
        assert_eq!(popped.pc, 2);

        assert!(!history.can_undo());
    }

    #[test]
    fn test_clear() {
        let mut history = StateHistory::new(10);
        let state = State::new();

        history.push(state.clone());
        history.push(state.clone());

        assert_eq!(history.depth(), 2);

        history.clear();

        assert_eq!(history.depth(), 0);
        assert!(!history.can_undo());
    }

    #[test]
    fn test_player_session_simple() {
        // Create a simple scenario with two dialogues
        let nodes = vec![
            AstNode::Say {
                speaker: "Alice".to_string(),
                text: "Hello!".to_string(),
            },
            AstNode::Say {
                speaker: "Bob".to_string(),
                text: "Hi!".to_string(),
            },
        ];
        let ast = Ast::new(nodes, HashMap::new());

        let mut session = PlayerSession::new(ast);

        // First next should return the first dialogue
        match session.next() {
            PlayerResult::Step { display_step, .. } => {
                match display_step {
                    DisplayStep::Dialogue { speaker, text } => {
                        assert_eq!(speaker, "Alice");
                        assert_eq!(text, "Hello!");
                    }
                    _ => panic!("Expected Dialogue, got {:?}", display_step),
                }
            }
            PlayerResult::Ended => panic!("Should not end yet"),
        }

        assert_eq!(session.current_state().pc, 1);

        // Second next should return the second dialogue
        match session.next() {
            PlayerResult::Step { display_step, .. } => {
                match display_step {
                    DisplayStep::Dialogue { speaker, text } => {
                        assert_eq!(speaker, "Bob");
                        assert_eq!(text, "Hi!");
                    }
                    _ => panic!("Expected Dialogue, got {:?}", display_step),
                }
            }
            PlayerResult::Ended => {
                eprintln!("Session ended at second step. PC: {}", session.current_state().pc);
                panic!("Should not end yet at second step");
            }
        }

        assert_eq!(session.current_state().pc, 2);

        // Third next should end (pc >= ast.len())
        match session.next() {
            PlayerResult::Ended => {
                // Expected
            }
            PlayerResult::Step { .. } => panic!("Should have ended"),
        }
    }

    #[test]
    fn test_player_session_undo() {
        let nodes = vec![
            AstNode::Say {
                speaker: "Alice".to_string(),
                text: "First".to_string(),
            },
            AstNode::Say {
                speaker: "Alice".to_string(),
                text: "Second".to_string(),
            },
        ];
        let ast = Ast::new(nodes, HashMap::new());

        let mut session = PlayerSession::new(ast);

        // Can't undo at start
        assert!(!session.can_undo());

        // Advance once
        let _ = session.next();
        assert_eq!(session.current_state().pc, 1);

        // Now we can undo
        assert!(session.can_undo());

        // Undo
        let result = session.undo();
        assert!(result.is_some());
        assert_eq!(session.current_state().pc, 0);
    }
}
