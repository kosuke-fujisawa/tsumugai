//! Runtime execution engine for story scenarios
//!
//! This module provides the step function that executes AST nodes and manages state.

use crate::types::{
    ast::{Ast, AstNode},
    event::Event,
    output::Output,
    state::State,
};

#[cfg(test)]
mod tests;

/// Execute a single step of the story
///
/// Takes the current state, AST, and optional event, returns updated state and output
pub fn step(mut state: State, ast: &Ast, event: Option<Event>) -> (State, Output) {
    let mut output = Output::new();

    // Handle events first
    if let Some(event) = event {
        let event_handled = handle_event(&mut state, &event, &mut output, ast);
        if event_handled && !output.choices.is_empty() {
            // If we handled an event AND generated new choices, return early
            return (state, output);
        }
    }

    // If we're waiting for a choice, don't advance until choice is made
    if state.waiting_for_choice && output.choices.is_empty() {
        return (state, output);
    }

    // Execute current instruction
    loop {
        if state.pc >= ast.len() {
            // End of program
            break;
        }

        let node = match ast.get_node(state.pc) {
            Some(node) => node,
            None => break,
        };

        let should_continue = execute_node(&mut state, node, &mut output, ast);

        if !should_continue {
            break;
        }
    }

    (state, output)
}

fn handle_event(state: &mut State, event: &Event, output: &mut Output, ast: &Ast) -> bool {
    match event {
        Event::Choice { id } => {
            if state.waiting_for_choice {
                // Extract choice index from ID (e.g., "choice_0" -> 0)
                if let Some(index_str) = id.strip_prefix("choice_") {
                    if let Ok(choice_index) = index_str.parse::<usize>() {
                        if choice_index < state.pending_choices.len() {
                            // Get the target label for this choice
                            let target_label = &state.pending_choices[choice_index];

                            // Jump to the target label
                            if let Some(target_pc) = ast.get_label_index(target_label) {
                                state.pc = target_pc;
                            }

                            // Clear the choice state
                            state.waiting_for_choice = false;
                            state.pending_choices.clear();

                            return true; // Event was handled
                        }
                    }
                }
            }
            false
        }
        Event::Continue => {
            // Just continue execution
            false
        }
        Event::Save { slot: _ } => {
            // Save handling would be done at a higher level
            output.add_effect("save".to_string(), None);
            true
        }
        Event::Load { slot: _ } => {
            // Load handling would be done at a higher level
            output.add_effect("load".to_string(), None);
            true
        }
    }
}

fn execute_node(state: &mut State, node: &AstNode, output: &mut Output, ast: &Ast) -> bool {
    match node {
        AstNode::Say { speaker, text } => {
            output.add_line(Some(speaker.clone()), text.clone());
            state.pc += 1;
            false // Stop here, wait for user input
        }
        AstNode::ShowImage { layer, name } => {
            output.add_effect(
                "show_image".to_string(),
                Some(serde_json::json!({
                    "layer": layer,
                    "name": name
                })),
            );
            state.pc += 1;
            false // Stop here to allow UI to process effect
        }
        AstNode::PlayBgm { name } => {
            output.add_effect(
                "play_bgm".to_string(),
                Some(serde_json::json!({
                    "name": name
                })),
            );
            state.pc += 1;
            false // Stop here to allow UI to process effect
        }
        AstNode::PlaySe { name } => {
            output.add_effect(
                "play_se".to_string(),
                Some(serde_json::json!({
                    "name": name
                })),
            );
            state.pc += 1;
            false // Stop here to allow UI to process effect
        }
        AstNode::PlayMovie { name } => {
            output.add_effect(
                "play_movie".to_string(),
                Some(serde_json::json!({
                    "name": name
                })),
            );
            state.pc += 1;
            false // Stop here to allow UI to process effect
        }
        AstNode::Wait { seconds } => {
            output.add_effect(
                "wait".to_string(),
                Some(serde_json::json!({
                    "seconds": seconds
                })),
            );
            state.pc += 1;
            false // Stop here
        }
        AstNode::Branch { choices } => {
            for choice in choices {
                output.add_choice(choice.id.clone(), choice.label.clone());
            }
            state.waiting_for_choice = true;
            state.pending_choices = choices.iter().map(|c| c.target.clone()).collect();
            state.pc += 1;
            false // Stop here, wait for choice
        }
        AstNode::Jump { label } => {
            if let Some(target_pc) = ast.get_label_index(label) {
                state.pc = target_pc;
            } else {
                // Should not happen if validation passed
                state.pc += 1;
            }
            true // Continue
        }
        AstNode::JumpIf {
            var,
            cmp,
            value,
            label,
        } => {
            match state.check_condition(var, cmp, value) {
                Ok(true) => {
                    if let Some(target_pc) = ast.get_label_index(label) {
                        state.pc = target_pc;
                    } else {
                        state.pc += 1;
                    }
                }
                Ok(false) => {
                    state.pc += 1;
                }
                Err(_) => {
                    // Error in condition evaluation, just continue
                    state.pc += 1;
                }
            }
            true // Continue
        }
        AstNode::Set { name, value } => {
            state.set_var(name.clone(), value.clone());
            state.pc += 1;
            true // Continue
        }
        AstNode::Modify { name, op, value } => {
            let _ = state.modify_var(name, op.clone(), value);
            state.pc += 1;
            true // Continue
        }
        AstNode::Label { name } => {
            state.last_label = Some(name.clone());
            output.add_effect(
                "label".to_string(),
                Some(serde_json::json!({
                    "name": name
                })),
            );
            state.pc += 1;
            true // Continue
        }
        AstNode::ClearLayer { layer } => {
            output.add_effect(
                "clear_layer".to_string(),
                Some(serde_json::json!({
                    "layer": layer
                })),
            );
            state.pc += 1;
            true // Continue
        }
    }
}
