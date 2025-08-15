use std::fs;
use std::io::{self, Write};
use tsumugai::{Engine, Step, WaitKind, parse};

fn main() {
    println!("Testing new commands implementation with Engine::step() loop...\n");

    let markdown = fs::read_to_string("test_commands.md").expect("Failed to read test file");

    println!("=== Raw markdown content ===");
    println!("{}", markdown);
    println!("================================\n");

    match parse(&markdown) {
        Ok(program) => {
            println!("Parsed {} commands successfully!\n", program.cmds.len());

            println!("=== Parsed commands ===");
            for (i, cmd) in program.cmds.iter().enumerate() {
                println!("{}: {:?}", i, cmd);
            }
            println!();

            let mut engine = Engine::new(program);
            let mut execution_log = Vec::new();

            println!("=== Executing commands with step loop ===");
            loop {
                match engine.step() {
                    Step::Next => {
                        // Continue execution
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }
                        continue;
                    }
                    Step::Wait(WaitKind::User) => {
                        // Handle user input
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }

                        print!("Press Enter to continue...");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        execution_log.push("User input: Enter pressed".to_string());
                    }
                    Step::Wait(WaitKind::Branch(choices)) => {
                        // Handle branch selection
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }

                        println!("Choose from the following options:");
                        for (i, choice) in choices.iter().enumerate() {
                            println!("  {}: {} (-> {})", i + 1, choice.choice, choice.label);
                        }

                        loop {
                            print!("Enter your choice (1-{}): ", choices.len());
                            io::stdout().flush().unwrap();
                            let mut input = String::new();
                            io::stdin().read_line(&mut input).unwrap();

                            if let Ok(choice_num) = input.trim().parse::<usize>() {
                                if choice_num >= 1 && choice_num <= choices.len() {
                                    let selected = &choices[choice_num - 1];
                                    execution_log.push(format!(
                                        "User selected: {} (-> {})",
                                        selected.choice, selected.label
                                    ));
                                    println!(
                                        "You selected: {} (jumping to {})",
                                        selected.choice, selected.label
                                    );

                                    if let Err(e) = engine.jump_to(&selected.label) {
                                        println!("Jump error: {}", e);
                                        execution_log.push(format!("Jump error: {}", e));
                                    }
                                    break;
                                } else {
                                    println!(
                                        "Invalid choice. Please enter a number between 1 and {}.",
                                        choices.len()
                                    );
                                    execution_log.push(format!("Invalid input: {}", input.trim()));
                                }
                            } else {
                                println!("Invalid input. Please enter a number.");
                                execution_log.push(format!("Invalid input: {}", input.trim()));
                            }
                        }
                    }
                    Step::Wait(WaitKind::Timer(secs)) => {
                        // Handle timer wait
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }

                        println!("Waiting for {:.1} seconds...", secs);
                        execution_log.push(format!("Timer wait: {:.1} seconds", secs));

                        // In a real application, you'd use proper timer/async mechanisms
                        // For demo purposes, we'll just continue immediately
                        print!("Press Enter to continue (simulating timer)...");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                    }
                    Step::Jump(label) => {
                        // Handle jump
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }

                        println!("Jumping to label: {}", label);
                        execution_log.push(format!("Jump to: {}", label));

                        if let Err(e) = engine.jump_to(&label) {
                            println!("Jump error: {}", e);
                            execution_log.push(format!("Jump error: {}", e));
                            break;
                        }
                    }
                    Step::Halt => {
                        // End of program
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }

                        println!("Program halted.");
                        execution_log.push("Program halted".to_string());
                        break;
                    }
                    Step::Wait(_) => {
                        // Catch-all for any other wait types due to non_exhaustive
                        let directives = engine.take_emitted();
                        for directive in directives {
                            execution_log.push(format!("Emitted: {:?}", directive));
                            println!("  {:?}", directive);
                        }

                        print!("Press Enter to continue...");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        execution_log.push("Fallback wait handling".to_string());
                    }
                }
            }

            println!("\n=== Final state ===");
            for (var_name, var_value) in engine.vars() {
                println!("{} = {:?}", var_name, var_value);
            }

            println!("\n=== Complete execution log ===");
            for (i, log_entry) in execution_log.iter().enumerate() {
                println!("{:3}: {}", i + 1, log_entry);
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);

            // Try to parse line by line for debugging
            println!("\n=== Line by line debug ===");
            for (line_num, line) in markdown.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    println!("Line {}: {}", line_num + 1, trimmed);
                }
            }
        }
    }
}
