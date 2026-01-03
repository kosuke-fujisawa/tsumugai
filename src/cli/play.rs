//! CUI player mode for running scenarios
//!
//! This module provides an interactive player mode where users can
//! experience scenarios in the terminal.

use crate::{
    cli::view_state::{clear_screen, render_delta, ViewState},
    player::{PlayerResult, PlayerSession},
    types::{display_step::DisplayStep, State},
};
use std::io::{self, Write};

/// Run the player mode
pub fn run_play(markdown: &str, debug: bool) -> anyhow::Result<()> {
    // Parse scenario
    let ast = crate::parser::parse(markdown)?;

    // Create player session
    let mut session = PlayerSession::new(ast);

    // Create view state for tracking visual changes
    let mut view_state = ViewState::new();

    println!("=== tsumugai Scenario Player ===");
    println!();
    println!("Controls:");
    println!("  Enter: next");
    println!("  1-9:   select choice");
    println!("  b:     back");
    println!("  q:     quit");
    println!();
    println!("Press Enter to start...");
    wait_input()?;

    // Main game loop
    loop {
        let result = session.next();

        match result {
            PlayerResult::Step {
                display_step,
                effects,
            } => {
                // Extract scene name if this is a scene boundary
                let scene_name = match &display_step {
                    DisplayStep::SceneBoundary { scene_name } => Some(scene_name.clone()),
                    _ => None,
                };

                // Apply effects to view state and get render delta
                let delta = view_state.apply_effects(&effects, scene_name);

                // Render delta (only show what changed)
                render_delta(&delta);

                // Display the step
                show_display_step(&display_step);

                // Display debug info if requested
                if debug {
                    display_debug_info(session.current_state());
                }

                // Check if we need to wait for choice
                if is_choice_block(&display_step) {
                    // Clear screen for choice display
                    clear_screen();

                    // Re-display the choice (after clearing)
                    show_display_step(&display_step);
                    loop {
                        let input = get_input("Select (1-9):")?;

                        if input == "q" {
                            println!("Goodbye!");
                            return Ok(());
                        }

                        if input == "b" {
                            if let Some((prev_step, prev_effects)) = session.undo() {
                                println!("(back)");
                                println!();

                                // Reset view state on undo (simplest approach)
                                view_state = ViewState::new();

                                // Re-display from the restored state
                                if let Some(step) = prev_step {
                                    // Apply effects with reset view state
                                    let scene_name = match &step {
                                        DisplayStep::SceneBoundary { scene_name } => {
                                            Some(scene_name.clone())
                                        }
                                        _ => None,
                                    };
                                    let delta = view_state.apply_effects(&prev_effects, scene_name);
                                    render_delta(&delta);

                                    show_display_step(&step);
                                    if debug {
                                        display_debug_info(session.current_state());
                                    }
                                    if is_choice_block(&step) {
                                        // Clear screen for choice after undo
                                        clear_screen();
                                        show_display_step(&step);
                                        // Continue waiting for choice
                                        continue;
                                    } else {
                                        // After undo to non-choice, break and wait for Enter
                                        break;
                                    }
                                } else {
                                    // Undone to initial state
                                    break;
                                }
                            } else {
                                println!("[Cannot undo]");
                                continue;
                            }
                        }

                        // Try to parse as choice index
                        if let Ok(choice_index) = input.parse::<usize>() {
                            if choice_index > 0 && choice_index <= 9 {
                                // Convert 1-based to 0-based index
                                let result = session.choose(choice_index - 1);

                                match result {
                                    PlayerResult::Step {
                                        display_step,
                                        effects,
                                    } => {
                                        // Extract scene name
                                        let scene_name = match &display_step {
                                            DisplayStep::SceneBoundary { scene_name } => {
                                                Some(scene_name.clone())
                                            }
                                            _ => None,
                                        };

                                        // Apply effects to view state
                                        let delta = view_state.apply_effects(&effects, scene_name);

                                        // Render delta
                                        render_delta(&delta);

                                        show_display_step(&display_step);
                                        if debug {
                                            display_debug_info(session.current_state());
                                        }
                                        break;
                                    }
                                    PlayerResult::Ended => {
                                        println!();
                                        println!("== THE END ==");
                                        return Ok(());
                                    }
                                }
                            } else {
                                println!("Invalid choice. Enter 1-9.");
                            }
                        } else {
                            println!("Invalid input. Enter a number, 'b', or 'q'.");
                        }
                    }
                } else {
                    // Wait for Enter or command
                    loop {
                        let input = get_input("")?;

                        if input == "q" {
                            println!("Goodbye!");
                            return Ok(());
                        }

                        if input == "b" {
                            if let Some((prev_step, prev_effects)) = session.undo() {
                                println!("(back)");
                                println!();

                                // Reset view state on undo
                                view_state = ViewState::new();

                                // Re-display from the restored state
                                if let Some(step) = prev_step {
                                    let scene_name = match &step {
                                        DisplayStep::SceneBoundary { scene_name } => {
                                            Some(scene_name.clone())
                                        }
                                        _ => None,
                                    };
                                    let delta = view_state.apply_effects(&prev_effects, scene_name);
                                    render_delta(&delta);

                                    show_display_step(&step);
                                    if debug {
                                        display_debug_info(session.current_state());
                                    }
                                }
                                // Continue waiting for input
                                continue;
                            } else {
                                println!("[Cannot undo]");
                                continue;
                            }
                        }

                        if input.is_empty() {
                            // Enter pressed, continue
                            break;
                        }

                        println!("Press Enter to continue, 'b' to go back, or 'q' to quit.");
                    }
                }
            }
            PlayerResult::Ended => {
                println!();
                println!("== THE END ==");
                break;
            }
        }
    }

    Ok(())
}

/// Display a single display step
fn show_display_step(step: &DisplayStep) {
    match step {
        DisplayStep::Dialogue { speaker, text } => {
            println!("{}:", speaker);
            println!("{}", text);
            println!();
        }
        DisplayStep::Narration { text } => {
            println!("{}", text);
            println!();
        }
        DisplayStep::ChoiceBlock { choices } => {
            println!("--- Choice ---");
            for (i, choice) in choices.iter().enumerate() {
                println!("{}. {}", i + 1, choice.label);
            }
            println!();
        }
        DisplayStep::SceneBoundary { scene_name } => {
            println!("[Scene: {}]", scene_name);
            println!();
        }
    }
}


/// Check if step is a choice block
fn is_choice_block(step: &DisplayStep) -> bool {
    matches!(step, DisplayStep::ChoiceBlock { .. })
}

/// Display debug information (only when --debug is set)
fn display_debug_info(state: &State) {
    println!("[debug]");

    // Show current scene/label
    if let Some(label) = &state.last_label {
        println!("scene={}", label);
    }

    // Show variables (flags) - only show non-empty
    if !state.flags.is_empty() {
        let vars_str = serde_json::to_string(&state.flags).unwrap_or_else(|_| "{}".to_string());
        println!("vars={}", vars_str);
    } else {
        println!("vars={{}}");
    }

    println!();
}

/// Wait for Enter key
fn wait_input() -> io::Result<()> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(())
}

/// Get user input with an optional prompt
fn get_input(prompt: &str) -> io::Result<String> {
    if !prompt.is_empty() {
        print!("{} ", prompt);
        io::stdout().flush()?;
    }

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
