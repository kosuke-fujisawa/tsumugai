//! CUI runtime example for strange_encounter scenario.
//!
//! This demonstrates the minimal tsumugai runtime:
//! - Enter key advances through SAY/WAIT/PLAY_MOVIE
//! - Number selection for BRANCH choices
//! - Resource resolution results are logged

use std::fs;
use std::io::{self, Write};
use tsumugai::{BasicResolver, Directive, Engine, NextAction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Strange Encounter Demo ===\n");

    let scenario_path = "assets/scenarios/strange_encounter.md";
    let markdown = fs::read_to_string(scenario_path)?;

    let resolver = BasicResolver::new("assets");
    let mut engine = Engine::from_markdown_with_resolver(&markdown, Box::new(resolver))?;

    loop {
        match engine.step() {
            Ok(step_result) => {
                handle_directives(&step_result.directives);

                match step_result.next {
                    NextAction::Next => {
                        // Continue immediately
                    }
                    NextAction::WaitUser => {
                        print!("Press Enter to continue...");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                    }
                    NextAction::WaitBranch => {
                        // Branch choices are in the directives
                        if let Some(Directive::Branch { choices }) = step_result
                            .directives
                            .iter()
                            .find(|d| matches!(d, Directive::Branch { .. }))
                        {
                            println!("\n選択してください:");
                            for (i, choice) in choices.iter().enumerate() {
                                println!("{}. {}", i + 1, choice);
                            }

                            print!("番号を入力: ");
                            io::stdout().flush().unwrap();

                            let mut input = String::new();
                            io::stdin().read_line(&mut input).unwrap();

                            if let Ok(choice_num) = input.trim().parse::<usize>() {
                                if choice_num > 0 && choice_num <= choices.len() {
                                    let choice_index = choice_num - 1;
                                    println!("選択: {}\n", choices[choice_index]);
                                    engine.choose(choice_index)?;
                                }
                            }
                        }
                    }
                    NextAction::Halt => {
                        println!("\n=== End of scenario ===");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn handle_directives(directives: &[Directive]) {
    for directive in directives {
        match directive {
            Directive::Say { speaker, text } => {
                println!("{}: {}", speaker, text);
            }
            Directive::PlayBgm { path } => {
                if let Some(path) = path {
                    println!("[BGM] Playing: {}", path);
                } else {
                    println!("[BGM] Playing: (not resolved)");
                }
            }
            Directive::ShowImage { layer, path } => {
                if let Some(path) = path {
                    println!("[IMAGE] Showing on {}: {}", layer, path);
                } else {
                    println!("[IMAGE] Showing on {}: (not resolved)", layer);
                }
            }
            Directive::Wait { seconds } => {
                println!("[WAIT] Waiting for {} seconds", seconds);
            }
            Directive::SetVar { name, value } => {
                println!("[SET] {} = {}", name, value);
            }
            Directive::JumpTo { label } => {
                println!("[JUMP] Jumping to: {}", label);
            }
            Directive::ClearLayer { layer } => {
                println!("[CLEAR] Clearing layer: {}", layer);
            }
            Directive::Branch { choices: _ } => {
                // Handled in the main loop
            }
            Directive::PlaySe { path } => {
                if let Some(path) = path {
                    println!("[SE] Playing: {}", path);
                } else {
                    println!("[SE] Playing: (not resolved)");
                }
            }
            Directive::PlayMovie { path } => {
                if let Some(path) = path {
                    println!("[MOVIE] Playing: {}", path);
                } else {
                    println!("[MOVIE] Playing: (not resolved)");
                }
            }
            Directive::ReachedLabel { label } => {
                println!("[LABEL] Reached: {}", label);
            }
            _ => {
                println!("[UNKNOWN] Unhandled directive: {:?}", directive);
            }
        }
    }
}
