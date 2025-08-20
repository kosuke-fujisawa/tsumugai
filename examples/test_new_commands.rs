use std::fs;
use std::io::{self, Write};
use tsumugai::{Engine, NextAction};

fn main() {
    println!("Testing new commands implementation with Engine::step() loop...\n");

    let markdown = fs::read_to_string("test_commands.md").expect("Failed to read test file");

    println!("=== Raw markdown content ===");
    println!("{markdown}");
    println!("================================\n");

    match Engine::from_markdown(&markdown) {
        Ok(mut engine) => {
            println!("Parsed markdown successfully!\n");
            let mut execution_log = Vec::new();

            println!("=== Executing commands with step loop ===");
            loop {
                match engine.step() {
                    Ok(step_result) => {
                        // Log all directives
                        for directive in &step_result.directives {
                            execution_log.push(format!("Emitted: {directive:?}"));
                            println!("  {directive:?}");
                        }

                        match step_result.next {
                            NextAction::Next => {
                                // Continue execution
                                continue;
                            }
                            NextAction::WaitUser => {
                                print!("Press Enter to continue...");
                                io::stdout().flush().expect("stdout flush failed");
                                let mut input = String::new();
                                io::stdin()
                                    .read_line(&mut input)
                                    .expect("stdin read failed");
                                execution_log.push("User input: Enter pressed".to_string());
                            }
                            NextAction::WaitBranch => {
                                // Handle branch selection
                                if let Some(choices) =
                                    step_result.directives.iter().find_map(|d| match d {
                                        tsumugai::Directive::Branch { choices } => Some(choices),
                                        _ => None,
                                    })
                                {
                                    println!("Choose from the following options:");
                                    for (i, choice) in choices.iter().enumerate() {
                                        println!("{}. {}", i + 1, choice);
                                    }

                                    print!("Enter choice number: ");
                                    io::stdout().flush().expect("stdout flush failed");
                                    let mut input = String::new();
                                    io::stdin()
                                        .read_line(&mut input)
                                        .expect("stdin read failed");

                                    if let Ok(choice_num) = input.trim().parse::<usize>() {
                                        if (1..=choices.len()).contains(&choice_num) {
                                            let choice_index = choice_num - 1;
                                            execution_log.push(format!(
                                                "User choice: {}",
                                                choices[choice_index]
                                            ));
                                            if let Err(e) = engine.choose(choice_index) {
                                                eprintln!("Error making choice: {e}");
                                                break;
                                            }
                                        } else {
                                            eprintln!(
                                                "Choice number out of range (1..{}).",
                                                choices.len()
                                            );
                                        }
                                    } else {
                                        eprintln!(
                                            "Invalid number. Please enter 1..{}.",
                                            choices.len()
                                        );
                                    }
                                }
                            }
                            NextAction::Halt => {
                                println!("=== Execution completed ===");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error during execution: {e}");
                        break;
                    }
                }
            }

            println!("\n=== Final execution log ===");
            for (i, entry) in execution_log.iter().enumerate() {
                println!("{}: {}", i + 1, entry);
            }
        }
        Err(e) => {
            eprintln!("Failed to parse markdown: {e}");
        }
    }
}
