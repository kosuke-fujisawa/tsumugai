//! Dump tool for generating golden test data
//! Usage: cargo run --example dump -- input.md > output.json

use std::env;
use std::fs;
use std::process;
use tsumugai::{Directive, Engine, NextAction};

#[derive(serde::Serialize)]
struct StepDump {
    step_number: usize,
    next_action: String,
    directives: Vec<Directive>,
}

#[derive(serde::Serialize)]
struct ScenarioDump {
    input_hash: String,
    steps: Vec<StepDump>,
    final_state: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <input.md>", args[0]);
        process::exit(1);
    }

    let input_file = &args[1];
    let input_content = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file {input_file}: {err}");
            process::exit(1);
        }
    };

    // Create engine from markdown
    let mut engine = match Engine::from_markdown(&input_content) {
        Ok(engine) => engine,
        Err(err) => {
            eprintln!("Parse error: {err}");
            process::exit(1);
        }
    };
    let mut steps = Vec::new();
    let mut step_number = 0;

    // Generate input hash for deterministic comparison
    let input_hash = format!("{:x}", md5::compute(input_content.as_bytes()));

    loop {
        let step_result = match engine.step() {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Step error: {err}");
                process::exit(1);
            }
        };

        let next_action_str = match &step_result.next {
            NextAction::Next => "Next".to_string(),
            NextAction::WaitUser => "WaitUser".to_string(),
            NextAction::WaitBranch => "WaitBranch".to_string(),
            NextAction::Halt => "Halt".to_string(),
        };

        let step_dump = StepDump {
            step_number,
            next_action: next_action_str,
            directives: step_result.directives.clone(),
        };

        steps.push(step_dump);
        step_number += 1;

        // Handle different step results
        match step_result.next {
            NextAction::Next => continue,
            NextAction::WaitUser => {
                // For dump purposes, automatically continue
                continue;
            }
            NextAction::WaitBranch => {
                // For dump purposes, take the first choice (index 0)
                if let Err(err) = engine.choose(0) {
                    eprintln!("Choice error: {err}");
                    process::exit(1);
                }
                continue;
            }
            NextAction::Halt => break,
        }
    }

    let scenario_dump = ScenarioDump {
        input_hash,
        steps,
        final_state: "Completed".to_string(),
    };

    match serde_json::to_string_pretty(&scenario_dump) {
        Ok(json) => println!("{json}"),
        Err(err) => {
            eprintln!("JSON serialization error: {err}");
            process::exit(1);
        }
    }
}
