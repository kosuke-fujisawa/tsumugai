//! CUI runtime example for strange_encounter scenario.
//!
//! This demonstrates the minimal tsumugai runtime:
//! - Enter key advances through SAY/WAIT/PLAY_MOVIE
//! - Number selection for BRANCH choices
//! - Resource resolution results are logged

use std::fs;
use std::io::{self, Write};
use tsumugai::{BasicResolver, Directive, Engine, Step, WaitKind, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Strange Encounter Demo ===\n");

    let scenario_path = "assets/scenarios/strange_encounter.md";
    let markdown = fs::read_to_string(scenario_path)?;

    let program = parse(&markdown)?;

    let resolver = BasicResolver::new("assets");
    let mut engine = Engine::with_resolver(program, Box::new(resolver));

    loop {
        match engine.step() {
            Step::Next => {
                let directives = engine.take_emitted();
                handle_directives(&directives);
            }
            Step::Wait(WaitKind::User) => {
                let directives = engine.take_emitted();
                handle_directives(&directives);
                print!("Press Enter to continue...");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
            }
            Step::Wait(WaitKind::Branch(choices)) => {
                let directives = engine.take_emitted();
                handle_directives(&directives);

                println!("\n選択してください:");
                for (i, choice) in choices.iter().enumerate() {
                    println!("{}. {}", i + 1, choice.choice);
                }

                print!("番号を入力: ");
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();

                if let Ok(choice_num) = input.trim().parse::<usize>() {
                    if choice_num > 0 && choice_num <= choices.len() {
                        let selected = &choices[choice_num - 1];
                        println!("選択: {}\n", selected.choice);
                        engine.jump_to(&selected.label)?;
                    }
                }
            }
            Step::Jump(label) => {
                let directives = engine.take_emitted();
                handle_directives(&directives);
                engine.jump_to(&label)?;
            }
            Step::Halt => {
                println!("\n=== End of scenario ===");
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
            Directive::PlayBgm { res } => {
                if let Some(path) = &res.resolved {
                    println!(
                        "[BGM] Playing: {} (resolved: {})",
                        res.logical,
                        path.display()
                    );
                } else {
                    println!("[BGM] Playing: {} (not resolved)", res.logical);
                }
            }
            Directive::PlaySe { res } => {
                if let Some(path) = &res.resolved {
                    println!(
                        "[SE] Playing: {} (resolved: {})",
                        res.logical,
                        path.display()
                    );
                } else {
                    println!("[SE] Playing: {} (not resolved)", res.logical);
                }
            }
            Directive::ShowImage { res } => {
                if let Some(path) = &res.resolved {
                    println!(
                        "[IMAGE] Showing: {} (resolved: {})",
                        res.logical,
                        path.display()
                    );
                } else {
                    println!("[IMAGE] Showing: {} (not resolved)", res.logical);
                }
            }
            Directive::PlayMovie { res } => {
                if let Some(path) = &res.resolved {
                    println!(
                        "[MOVIE] Playing: {} (resolved: {})",
                        res.logical,
                        path.display()
                    );
                } else {
                    println!("[MOVIE] Playing: {} (not resolved)", res.logical);
                }
            }
            Directive::Wait { secs } => {
                println!("[WAIT] Waiting for {} seconds", secs);
            }
            Directive::Label { name } => {
                println!("[LABEL] Reached: {}", name);
            }
            Directive::Jump { label } => {
                println!("[JUMP] Jumping to: {}", label);
            }
            Directive::Branch { choices: _ } => {
                // Handled in the main loop
            }
        }
    }
}
