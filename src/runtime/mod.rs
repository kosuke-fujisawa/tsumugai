//! Runtime execution engine for story scenarios
//!
//! This module provides the step function that executes AST nodes and manages state.

use crate::types::{
    ast::{Ast, AstNode, Expr},
    debug::{DebugTraceEvent, JumpReason},
    display_step::{ChoiceItem, DisplayStep, Effects},
    event::Event,
    output::Output,
    state::State,
};

pub mod debug;

#[cfg(test)]
mod tests;

/// Execute a single step of the story
///
/// Takes the current state, AST, and optional event, returns updated state and output
pub fn step(state: State, ast: &Ast, event: Option<Event>) -> (State, Output) {
    step_with_debug(state, ast, event, &debug::DebugConfig::default())
}

/// Execute until the next DisplayStep
///
/// This function advances the story to the next meaningful display unit.
/// Returns the updated state, the DisplayStep (if any), and collected effects.
pub fn step_to_next_display(
    mut state: State,
    ast: &Ast,
    event: Option<Event>,
) -> (State, Option<DisplayStep>, Effects) {
    let mut effects = Effects::new();

    // Handle events first
    if let Some(ref event) = event {
        match event {
            Event::Choice { id } => {
                if state.waiting_for_choice {
                    // Extract choice index from ID (e.g., "choice_0" -> 0)
                    if let Some(index_str) = id.strip_prefix("choice_")
                        && let Ok(choice_index) = index_str.parse::<usize>()
                        && choice_index < state.pending_choices.len()
                    {
                        // Get the target label for this choice
                        let target_label = &state.pending_choices[choice_index];

                        // Jump to the target label
                        if let Some(target_pc) = ast.get_label_index(target_label) {
                            state.pc = target_pc;
                        }

                        // Clear the choice state
                        state.waiting_for_choice = false;
                        state.pending_choices.clear();
                    }
                }
            }
            Event::Continue => {
                // Just continue execution
            }
            Event::Save { slot: _ } | Event::Load { slot: _ } => {
                // Save/Load handling would be done at a higher level
                return (state, None, effects);
            }
        }
    }

    // If we're waiting for a choice, don't advance
    if state.waiting_for_choice {
        return (state, None, effects);
    }

    // Execute nodes until we hit a DisplayStep
    loop {
        if state.pc >= ast.len() {
            // End of program
            return (state, None, effects);
        }

        let node = match ast.get_node(state.pc) {
            Some(node) => node,
            None => return (state, None, effects),
        };

        match node {
            AstNode::Say { speaker, text } => {
                let display_step = if speaker.is_empty() {
                    DisplayStep::Narration { text: text.clone() }
                } else {
                    DisplayStep::Dialogue {
                        speaker: speaker.clone(),
                        text: text.clone(),
                    }
                };
                state.pc += 1;
                return (state, Some(display_step), effects);
            }
            AstNode::ShowImage { layer, name } => {
                effects.add_image(layer.clone(), name.clone());
                state.pc += 1;
                // Continue to next node
            }
            AstNode::PlayBgm { name } => {
                effects.set_bgm(name.clone());
                state.pc += 1;
                // Continue to next node
            }
            AstNode::PlaySe { name } => {
                effects.add_se(name.clone());
                state.pc += 1;
                // Continue to next node
            }
            AstNode::PlayMovie { name } => {
                effects.add_other(format!("PlayMovie: {}", name));
                state.pc += 1;
                // Continue to next node
            }
            AstNode::Wait { seconds } => {
                effects.add_other(format!("Wait: {}s", seconds));
                state.pc += 1;
                // Continue to next node
            }
            AstNode::ClearLayer { layer } => {
                effects.clear_layer(layer.clone());
                state.pc += 1;
                // Continue to next node
            }
            AstNode::Branch { choices } => {
                let choice_items: Vec<ChoiceItem> = choices
                    .iter()
                    .map(|c| ChoiceItem {
                        id: c.id.clone(),
                        label: c.label.clone(),
                        target: c.target.clone(),
                    })
                    .collect();
                let display_step = DisplayStep::ChoiceBlock {
                    choices: choice_items,
                };
                state.waiting_for_choice = true;
                state.pending_choices = choices.iter().map(|c| c.target.clone()).collect();
                state.pc += 1;
                return (state, Some(display_step), effects);
            }
            AstNode::Scene { meta } => {
                let display_step = DisplayStep::SceneBoundary {
                    scene_name: meta.name.clone(),
                };
                state.last_label = Some(meta.name.clone());
                state.pc += 1;
                return (state, Some(display_step), effects);
            }
            AstNode::Jump { label } => {
                if let Some(target_pc) = ast.get_label_index(label) {
                    state.pc = target_pc;
                } else {
                    state.pc += 1;
                }
                // Continue
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
                    Ok(false) | Err(_) => {
                        state.pc += 1;
                    }
                }
                // Continue
            }
            AstNode::Set { name, value } => {
                state.set_var(name.clone(), value.clone());
                state.pc += 1;
                // Continue
            }
            AstNode::Modify { name, op, value } => {
                let _ = state.modify_var(name, op.clone(), value);
                state.pc += 1;
                // Continue
            }
            AstNode::Label { name } => {
                state.last_label = Some(name.clone());
                state.pc += 1;
                // Continue
            }
            AstNode::Goto { target } => {
                if let Some(target_pc) = ast.get_label_index(target) {
                    state.pc = target_pc;
                } else {
                    state.pc += 1;
                }
                // Continue
            }
            AstNode::WhenBlock { condition, body } => {
                // Evaluate condition
                let condition_result = eval_expr(condition, &state).unwrap_or(false);
                if condition_result {
                    // Execute body nodes
                    for body_node in body {
                        execute_node_in_display_step(&mut state, body_node, &mut effects, ast);
                    }
                }
                state.pc += 1;
                // Continue
            }
        }
    }
}

/// Execute a node within the display step context (for WhenBlock bodies)
fn execute_node_in_display_step(
    state: &mut State,
    node: &AstNode,
    effects: &mut Effects,
    ast: &Ast,
) {
    match node {
        AstNode::ShowImage { layer, name } => {
            effects.add_image(layer.clone(), name.clone());
        }
        AstNode::PlayBgm { name } => {
            effects.set_bgm(name.clone());
        }
        AstNode::PlaySe { name } => {
            effects.add_se(name.clone());
        }
        AstNode::ClearLayer { layer } => {
            effects.clear_layer(layer.clone());
        }
        AstNode::Set { name, value } => {
            state.set_var(name.clone(), value.clone());
        }
        AstNode::Modify { name, op, value } => {
            let _ = state.modify_var(name, op.clone(), value);
        }
        AstNode::Jump { label } => {
            if let Some(target_pc) = ast.get_label_index(label) {
                state.pc = target_pc;
            }
        }
        AstNode::Goto { target } => {
            if let Some(target_pc) = ast.get_label_index(target) {
                state.pc = target_pc;
            }
        }
        _ => {
            // Other nodes are not expected in WhenBlock bodies
        }
    }
}

/// Execute a single step with debug trace events
///
/// Takes the current state, AST, and optional event, returns updated state, output, and trace events
pub fn step_with_trace(
    state: State,
    ast: &Ast,
    event: Option<Event>,
) -> (State, Output, Vec<DebugTraceEvent>) {
    let mut trace_events = Vec::new();
    let (new_state, output) = step_with_trace_internal(state, ast, event, &mut trace_events);
    (new_state, output, trace_events)
}

/// Execute a single step with debug configuration
///
/// Takes the current state, AST, optional event, and debug config, returns updated state and output
pub fn step_with_debug(
    mut state: State,
    ast: &Ast,
    event: Option<Event>,
    debug_config: &debug::DebugConfig,
) -> (State, Output) {
    let start_pc = state.pc;
    let mut output = Output::new();

    debug::log(
        debug_config,
        debug::DebugCategory::Engine,
        debug::LogLevel::Debug,
        &format!("[Step] Starting at PC={}", start_pc),
    );

    // Handle events first
    if let Some(ref event) = event {
        debug::log(
            debug_config,
            debug::DebugCategory::Flow,
            debug::LogLevel::Debug,
            &format!("[Event] Handling event: {:?}", event),
        );
        let event_handled = handle_event(&mut state, event, &mut output, ast, debug_config);
        if event_handled && !output.choices.is_empty() {
            // If we handled an event AND generated new choices, return early
            return (state, output);
        }
    }

    // If we're waiting for a choice, don't advance until choice is made
    if state.waiting_for_choice && output.choices.is_empty() {
        debug::log(
            debug_config,
            debug::DebugCategory::Flow,
            debug::LogLevel::Debug,
            "[Flow] Waiting for choice, not advancing",
        );
        return (state, output);
    }

    // Execute current instruction
    loop {
        if state.pc >= ast.len() {
            // End of program
            debug::log(
                debug_config,
                debug::DebugCategory::Engine,
                debug::LogLevel::Info,
                &format!("[Engine] Reached end of program at PC={}", state.pc),
            );
            break;
        }

        let node = match ast.get_node(state.pc) {
            Some(node) => node,
            None => break,
        };

        debug::log(
            debug_config,
            debug::DebugCategory::Engine,
            debug::LogLevel::Trace,
            &format!("[Engine] Executing PC={} node={:?}", state.pc, node),
        );

        let should_continue = execute_node(&mut state, node, &mut output, ast, debug_config);

        if !should_continue {
            break;
        }
    }

    (state, output)
}

/// Internal step function with trace event collection
fn step_with_trace_internal(
    mut state: State,
    ast: &Ast,
    event: Option<Event>,
    trace_events: &mut Vec<DebugTraceEvent>,
) -> (State, Output) {
    let mut output = Output::new();

    // Handle events first
    if let Some(ref event) = event {
        let event_handled =
            handle_event_with_trace(&mut state, event, &mut output, ast, trace_events);
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

        let should_continue =
            execute_node_with_trace(&mut state, node, &mut output, ast, trace_events);

        if !should_continue {
            break;
        }
    }

    (state, output)
}

fn handle_event_with_trace(
    state: &mut State,
    event: &Event,
    output: &mut Output,
    ast: &Ast,
    _trace_events: &mut Vec<DebugTraceEvent>,
) -> bool {
    match event {
        Event::Choice { id } => {
            if state.waiting_for_choice {
                // Extract choice index from ID (e.g., "choice_0" -> 0)
                if let Some(index_str) = id.strip_prefix("choice_")
                    && let Ok(choice_index) = index_str.parse::<usize>()
                    && choice_index < state.pending_choices.len()
                {
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

fn handle_event(
    state: &mut State,
    event: &Event,
    output: &mut Output,
    ast: &Ast,
    debug_config: &debug::DebugConfig,
) -> bool {
    match event {
        Event::Choice { id } => {
            if state.waiting_for_choice {
                // Extract choice index from ID (e.g., "choice_0" -> 0)
                if let Some(index_str) = id.strip_prefix("choice_")
                    && let Ok(choice_index) = index_str.parse::<usize>()
                    && choice_index < state.pending_choices.len()
                {
                    // Get the target label for this choice
                    let target_label = &state.pending_choices[choice_index];

                    // Jump to the target label
                    if let Some(target_pc) = ast.get_label_index(target_label) {
                        debug::log(
                            debug_config,
                            debug::DebugCategory::Flow,
                            debug::LogLevel::Debug,
                            &format!(
                                "[Branch] Choice {} selected, jumping from PC={} to PC={} (label={})",
                                choice_index, state.pc, target_pc, target_label
                            ),
                        );
                        state.pc = target_pc;
                    }

                    // Clear the choice state
                    state.waiting_for_choice = false;
                    state.pending_choices.clear();

                    return true; // Event was handled
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

fn execute_node(
    state: &mut State,
    node: &AstNode,
    output: &mut Output,
    ast: &Ast,
    debug_config: &debug::DebugConfig,
) -> bool {
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
            debug::log(
                debug_config,
                debug::DebugCategory::Flow,
                debug::LogLevel::Debug,
                &format!(
                    "[Branch] Presenting {} choices at PC={}",
                    choices.len(),
                    state.pc
                ),
            );
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
                debug::log(
                    debug_config,
                    debug::DebugCategory::Flow,
                    debug::LogLevel::Debug,
                    &format!(
                        "[Jump] Jumping from PC={} to PC={} (label={})",
                        state.pc, target_pc, label
                    ),
                );
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
                        debug::log(
                            debug_config,
                            debug::DebugCategory::Flow,
                            debug::LogLevel::Debug,
                            &format!(
                                "[JumpIf] Condition {}={:?} {:?} {} is TRUE, jumping from PC={} to PC={} (label={})",
                                var,
                                state.get_var(var),
                                cmp,
                                value,
                                state.pc,
                                target_pc,
                                label
                            ),
                        );
                        state.pc = target_pc;
                    } else {
                        state.pc += 1;
                    }
                }
                Ok(false) => {
                    debug::log(
                        debug_config,
                        debug::DebugCategory::Flow,
                        debug::LogLevel::Debug,
                        &format!(
                            "[JumpIf] Condition {}={:?} {:?} {} is FALSE, not jumping",
                            var,
                            state.get_var(var),
                            cmp,
                            value
                        ),
                    );
                    state.pc += 1;
                }
                Err(_) => {
                    // Error in condition evaluation, just continue
                    debug::log(
                        debug_config,
                        debug::DebugCategory::Flow,
                        debug::LogLevel::Warn,
                        &format!("[JumpIf] Error evaluating condition for var={}", var),
                    );
                    state.pc += 1;
                }
            }
            true // Continue
        }
        AstNode::Set { name, value } => {
            debug::log(
                debug_config,
                debug::DebugCategory::Variables,
                debug::LogLevel::Debug,
                &format!("[Set] Setting {}={}", name, value),
            );
            state.set_var(name.clone(), value.clone());
            state.pc += 1;
            true // Continue
        }
        AstNode::Modify { name, op, value } => {
            let old_value = state.get_var(name);
            let _ = state.modify_var(name, op.clone(), value);
            let new_value = state.get_var(name);
            debug::log(
                debug_config,
                debug::DebugCategory::Variables,
                debug::LogLevel::Debug,
                &format!(
                    "[Modify] {} {:?} {} : {:?} -> {:?}",
                    name, op, value, old_value, new_value
                ),
            );
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
        AstNode::Goto { target } => {
            if let Some(target_pc) = ast.get_label_index(target) {
                debug::log(
                    debug_config,
                    debug::DebugCategory::Flow,
                    debug::LogLevel::Debug,
                    &format!(
                        "[Goto] Jumping from PC={} to PC={} (target={})",
                        state.pc, target_pc, target
                    ),
                );
                state.pc = target_pc;
            } else {
                // Should not happen if validation passed
                state.pc += 1;
            }
            true // Continue
        }
        AstNode::Scene { meta } => {
            state.last_label = Some(meta.name.clone());
            output.add_effect(
                "scene".to_string(),
                Some(serde_json::json!({
                    "name": meta.name,
                    "ending": meta.ending
                })),
            );
            state.pc += 1;
            true // Continue
        }
        AstNode::WhenBlock { condition, body } => {
            // Evaluate condition
            let condition_result = eval_expr(condition, state).unwrap_or(false);
            debug::log(
                debug_config,
                debug::DebugCategory::Flow,
                debug::LogLevel::Debug,
                &format!(
                    "[WhenBlock] Condition: {:?} = {}",
                    condition, condition_result
                ),
            );

            if condition_result {
                // Execute body nodes
                for node in body {
                    execute_node(state, node, output, ast, debug_config);
                }
            }

            state.pc += 1;
            true // Continue
        }
    }
}

/// Evaluate an expression against the current state
fn eval_expr(expr: &Expr, state: &State) -> anyhow::Result<bool> {
    match expr {
        Expr::Bool(b) => Ok(*b),
        Expr::Number(_) => anyhow::bail!("Cannot use number as boolean condition"),
        Expr::String(_) => anyhow::bail!("Cannot use string as boolean condition"),
        Expr::Var(name) => {
            // Get variable value and try to interpret as boolean
            if let Some(value) = state.get_var(name) {
                match value.to_lowercase().as_str() {
                    "true" | "1" => Ok(true),
                    "false" | "0" => Ok(false),
                    _ => {
                        // Check if it's a number > 0
                        if let Ok(num) = value.parse::<i64>() {
                            Ok(num != 0)
                        } else {
                            Ok(!value.is_empty())
                        }
                    }
                }
            } else {
                // Undefined variable is treated as false
                Ok(false)
            }
        }
        Expr::Equal(left, right) => {
            let left_val = eval_expr_to_value(left, state)?;
            let right_val = eval_expr_to_value(right, state)?;
            Ok(left_val == right_val)
        }
        Expr::NotEqual(left, right) => {
            let left_val = eval_expr_to_value(left, state)?;
            let right_val = eval_expr_to_value(right, state)?;
            Ok(left_val != right_val)
        }
        Expr::LessThan(left, right) => {
            let left_num = eval_expr_to_number(left, state)?;
            let right_num = eval_expr_to_number(right, state)?;
            Ok(left_num < right_num)
        }
        Expr::LessThanOrEqual(left, right) => {
            let left_num = eval_expr_to_number(left, state)?;
            let right_num = eval_expr_to_number(right, state)?;
            Ok(left_num <= right_num)
        }
        Expr::GreaterThan(left, right) => {
            let left_num = eval_expr_to_number(left, state)?;
            let right_num = eval_expr_to_number(right, state)?;
            Ok(left_num > right_num)
        }
        Expr::GreaterThanOrEqual(left, right) => {
            let left_num = eval_expr_to_number(left, state)?;
            let right_num = eval_expr_to_number(right, state)?;
            Ok(left_num >= right_num)
        }
        Expr::And(left, right) => {
            let left_bool = eval_expr(left, state)?;
            if !left_bool {
                return Ok(false); // Short-circuit
            }
            eval_expr(right, state)
        }
        Expr::Or(left, right) => {
            let left_bool = eval_expr(left, state)?;
            if left_bool {
                return Ok(true); // Short-circuit
            }
            eval_expr(right, state)
        }
        Expr::Not(expr) => {
            let bool_val = eval_expr(expr, state)?;
            Ok(!bool_val)
        }
    }
}

/// Evaluate an expression to a value (string)
fn eval_expr_to_value(expr: &Expr, state: &State) -> anyhow::Result<String> {
    match expr {
        Expr::Bool(b) => Ok(b.to_string()),
        Expr::Number(n) => Ok(n.to_string()),
        Expr::String(s) => Ok(s.clone()),
        Expr::Var(name) => {
            if let Some(value) = state.get_var(name) {
                Ok(value)
            } else {
                Ok(String::new())
            }
        }
        _ => anyhow::bail!("Cannot evaluate complex expression to value"),
    }
}

/// Evaluate an expression to a number
fn eval_expr_to_number(expr: &Expr, state: &State) -> anyhow::Result<i64> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Var(name) => {
            if let Some(value) = state.get_var(name) {
                value
                    .parse::<i64>()
                    .map_err(|_| anyhow::anyhow!("Variable '{}' is not a number", name))
            } else {
                Ok(0)
            }
        }
        Expr::String(s) => s
            .parse::<i64>()
            .map_err(|_| anyhow::anyhow!("String '{}' is not a number", s)),
        _ => anyhow::bail!("Cannot evaluate complex expression to number"),
    }
}

/// Execute a node with trace event generation
fn execute_node_with_trace(
    state: &mut State,
    node: &AstNode,
    output: &mut Output,
    ast: &Ast,
    trace_events: &mut Vec<DebugTraceEvent>,
) -> bool {
    match node {
        AstNode::Say { speaker, text } => {
            // Generate Dialogue trace event
            trace_events.push(DebugTraceEvent::Dialogue {
                speaker: if speaker.is_empty() {
                    None
                } else {
                    Some(speaker.clone())
                },
                text: text.clone(),
                click_wait: true,
            });
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
            // Generate PresentChoices trace event
            use crate::types::debug::ChoiceItem;
            let choice_items: Vec<ChoiceItem> = choices
                .iter()
                .map(|c| ChoiceItem {
                    id: c.id.clone(),
                    label: c.label.clone(),
                    condition: c.condition.clone(),
                })
                .collect();
            trace_events.push(DebugTraceEvent::PresentChoices {
                choices: choice_items,
            });

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
                // Generate Jump trace event
                trace_events.push(DebugTraceEvent::Jump {
                    to: label.clone(),
                    reason: JumpReason::Sequential,
                });
                state.pc = target_pc;
            } else {
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
                        // Generate Jump trace event with When reason
                        trace_events.push(DebugTraceEvent::Jump {
                            to: label.clone(),
                            reason: JumpReason::When {
                                expr: format!("{} {:?} {}", var, cmp, value),
                            },
                        });
                        state.pc = target_pc;
                    } else {
                        state.pc += 1;
                    }
                }
                Ok(false) => {
                    state.pc += 1;
                }
                Err(_) => {
                    state.pc += 1;
                }
            }
            true // Continue
        }
        AstNode::Set { name, value } => {
            // Get before value
            let before = state
                .flags
                .get(name)
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Set new value
            state.set_var(name.clone(), value.clone());

            // Get after value
            let after = state
                .flags
                .get(name)
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Generate EffectSetVar trace event
            trace_events.push(DebugTraceEvent::EffectSetVar {
                name: name.clone(),
                before,
                after,
            });

            state.pc += 1;
            true // Continue
        }
        AstNode::Modify { name, op, value } => {
            // Get before value
            let before = state
                .flags
                .get(name)
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Modify value
            let _ = state.modify_var(name, op.clone(), value);

            // Get after value
            let after = state
                .flags
                .get(name)
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Generate EffectSetVar trace event
            trace_events.push(DebugTraceEvent::EffectSetVar {
                name: name.clone(),
                before,
                after,
            });

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
        AstNode::Goto { target } => {
            if let Some(target_pc) = ast.get_label_index(target) {
                // Generate Jump trace event with Goto reason
                trace_events.push(DebugTraceEvent::Jump {
                    to: target.clone(),
                    reason: JumpReason::Goto,
                });
                state.pc = target_pc;
            } else {
                state.pc += 1;
            }
            true // Continue
        }
        AstNode::Scene { meta } => {
            // Generate EnterScene trace event
            trace_events.push(DebugTraceEvent::EnterScene {
                name: meta.name.clone(),
            });

            state.last_label = Some(meta.name.clone());
            output.add_effect(
                "scene".to_string(),
                Some(serde_json::json!({
                    "name": meta.name,
                    "ending": meta.ending
                })),
            );
            state.pc += 1;
            true // Continue
        }
        AstNode::WhenBlock { condition, body } => {
            // Evaluate condition
            let condition_result = eval_expr(condition, state).unwrap_or(false);

            if condition_result {
                // Execute body nodes
                for node in body {
                    execute_node_with_trace(state, node, output, ast, trace_events);
                }
            }

            state.pc += 1;
            true // Continue
        }
    }
}
