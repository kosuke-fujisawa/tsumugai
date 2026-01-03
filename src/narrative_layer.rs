//! Narrative layer for converting runtime output to player-friendly events
//!
//! This module provides functions to convert raw runtime Output into
//! NarrativeEvent types suitable for display in a CUI player.

use crate::types::{
    narrative::{ChoiceOption, NarrativeEvent},
    output::Output,
    state::State,
    Ast,
};

/// Convert runtime output to narrative events
///
/// Priority order:
/// 1. Choices (if available)
/// 2. Dialogue/Narration (if available)
/// 3. Effects (if available)
/// 4. End (if at end of AST and no output)
pub fn output_to_events(output: &Output, state: &State, ast: &Ast) -> Vec<NarrativeEvent> {
    let mut events = Vec::new();

    // Add choices (highest priority)
    if !output.choices.is_empty() {
        let choices = output
            .choices
            .iter()
            .map(|c| ChoiceOption {
                id: c.id.clone(),
                label: c.label.clone(),
            })
            .collect();
        events.push(NarrativeEvent::choices(choices));
    }

    // Add dialogue/narration
    for line in &output.lines {
        events.push(NarrativeEvent::dialogue(
            line.speaker.clone(),
            line.text.clone(),
        ));
    }

    // Add effects
    for effect in &output.effects {
        events.push(NarrativeEvent::effect(
            effect.tag.clone(),
            effect.opts.clone(),
        ));
    }

    // If nothing was generated, check if we're at the end
    if events.is_empty() {
        if state.waiting_for_choice {
            // Still waiting for a choice, return empty
            // The player should not advance
        } else if state.pc >= ast.len() {
            // At end of program
            events.push(NarrativeEvent::end());
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ast::AstNode, output::Output, state::State, Ast};
    use std::collections::HashMap;

    #[test]
    fn test_dialogue_event() {
        let mut output = Output::new();
        output.add_line(Some("Alice".to_string()), "Hello!".to_string());

        let state = State::new();
        // Create a non-empty AST so pc < ast.len()
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 1);
        match &events[0] {
            NarrativeEvent::Dialogue { speaker, text } => {
                assert_eq!(speaker.as_ref().unwrap(), "Alice");
                assert_eq!(text, "Hello!");
            }
            _ => panic!("Expected Dialogue event"),
        }
    }

    #[test]
    fn test_choices_event() {
        let mut output = Output::new();
        output.add_choice("choice_0".to_string(), "Option A".to_string());
        output.add_choice("choice_1".to_string(), "Option B".to_string());

        let state = State::new();
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 1);
        match &events[0] {
            NarrativeEvent::Choices { choices } => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0].id, "choice_0");
                assert_eq!(choices[0].label, "Option A");
            }
            _ => panic!("Expected Choices event"),
        }
    }

    #[test]
    fn test_end_event() {
        let output = Output::new();
        let state = State::new();
        // Empty AST means pc=0 >= ast.len()=0
        let ast = Ast::new(vec![], HashMap::new());

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NarrativeEvent::End));
    }

    #[test]
    fn test_effect_event() {
        let mut output = Output::new();
        output.add_effect(
            "show_image".to_string(),
            Some(serde_json::json!({"layer": "bg", "name": "room.png"})),
        );

        let state = State::new();
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 1);
        match &events[0] {
            NarrativeEvent::Effect { kind, data } => {
                assert_eq!(kind, "show_image");
                assert!(data.is_some());
            }
            _ => panic!("Expected Effect event"),
        }
    }

    #[test]
    fn test_multiple_dialogue_lines() {
        let mut output = Output::new();
        output.add_line(Some("Alice".to_string()), "First line".to_string());
        output.add_line(Some("Bob".to_string()), "Second line".to_string());
        output.add_line(None, "Narration".to_string());

        let state = State::new();
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 3);
        
        match &events[0] {
            NarrativeEvent::Dialogue { speaker, text } => {
                assert_eq!(speaker.as_ref().unwrap(), "Alice");
                assert_eq!(text, "First line");
            }
            _ => panic!("Expected Dialogue event"),
        }
        
        match &events[2] {
            NarrativeEvent::Dialogue { speaker, text } => {
                assert!(speaker.is_none());
                assert_eq!(text, "Narration");
            }
            _ => panic!("Expected Dialogue event"),
        }
    }

    #[test]
    fn test_multiple_effects() {
        let mut output = Output::new();
        output.add_effect(
            "show_image".to_string(),
            Some(serde_json::json!({"layer": "bg"})),
        );
        output.add_effect(
            "play_bgm".to_string(),
            Some(serde_json::json!({"name": "theme.mp3"})),
        );

        let state = State::new();
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 2);
        
        assert!(matches!(&events[0], NarrativeEvent::Effect { .. }));
        assert!(matches!(&events[1], NarrativeEvent::Effect { .. }));
    }

    #[test]
    fn test_choices_and_dialogue_mixed() {
        let mut output = Output::new();
        output.add_choice("choice_0".to_string(), "Option A".to_string());
        output.add_line(Some("Alice".to_string()), "Choose wisely".to_string());

        let state = State::new();
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        
        // Choices should come first (highest priority)
        assert!(events.len() >= 2);
        assert!(matches!(&events[0], NarrativeEvent::Choices { .. }));
        assert!(matches!(&events[1], NarrativeEvent::Dialogue { .. }));
    }

    #[test]
    fn test_empty_output_waiting_for_choice() {
        let output = Output::new();
        let mut state = State::new();
        state.waiting_for_choice = true;
        
        let ast = Ast::new(vec![], HashMap::new());

        let events = output_to_events(&output, &state, &ast);
        // Should return empty when waiting for choice
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_empty_output_mid_scenario() {
        let output = Output::new();
        let state = State::new();
        
        // Non-empty AST so pc < ast.len()
        let ast = Ast::new(
            vec![
                AstNode::Say {
                    speaker: "Test".to_string(),
                    text: "Test".to_string(),
                },
                AstNode::Say {
                    speaker: "Test2".to_string(),
                    text: "Test2".to_string(),
                },
            ],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        // Should return empty (no end event)
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_all_event_types_combined() {
        let mut output = Output::new();
        output.add_choice("choice_0".to_string(), "Option 1".to_string());
        output.add_line(Some("Alice".to_string()), "What do you choose?".to_string());
        output.add_effect("show_image".to_string(), None);

        let state = State::new();
        let ast = Ast::new(
            vec![AstNode::Say {
                speaker: "Test".to_string(),
                text: "Test".to_string(),
            }],
            HashMap::new(),
        );

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 3);
        
        // Order: Choices, Dialogue, Effects
        assert!(matches!(&events[0], NarrativeEvent::Choices { .. }));
        assert!(matches!(&events[1], NarrativeEvent::Dialogue { .. }));
        assert!(matches!(&events[2], NarrativeEvent::Effect { .. }));
    }

    #[test]
    fn test_end_event_at_ast_end() {
        let output = Output::new();
        let mut state = State::new();
        state.pc = 10; // Beyond AST length
        
        let ast = Ast::new(vec![], HashMap::new()); // Empty AST (len = 0)

        let events = output_to_events(&output, &state, &ast);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NarrativeEvent::End));
    }

    #[test]
    fn test_choice_option_creation() {
        use crate::types::narrative::ChoiceOption;
        
        let choice = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Test label".to_string(),
        };
        
        assert_eq!(choice.id, "choice_0");
        assert_eq!(choice.label, "Test label");
    }
}
