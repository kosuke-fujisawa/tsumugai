//! CLI entry point for tsumugai
//!
//! This provides command-line interface for scenario validation and dry run.

use std::fs;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "play" => {
            if args.len() < 3 {
                eprintln!("Error: Missing scenario file path");
                eprintln!();
                print_usage();
                process::exit(1);
            }
            let file_path = PathBuf::from(&args[2]);
            let debug = args.get(3).map(|s| s == "--debug").unwrap_or(false);
            run_play(file_path, debug);
        }
        "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", command);
            eprintln!();
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("tsumugai - Visual Novel Scenario Engine");
    println!();
    println!("USAGE:");
    println!("    cargo run -- play <scenario.md> [--debug]");
    println!();
    println!("COMMANDS:");
    println!("    play <file> [--debug]    Play scenario in CUI player mode");
    println!("    --help, -h               Show this help message");
    println!();
    println!("OPTIONS:");
    println!("    --debug    Show debug information (scene, vars, flags)");
    println!();
    println!("EXAMPLES:");
    println!("    cargo run -- play scenarios/example.md");
    println!("    cargo run -- play scenarios/example.md --debug");
}

fn run_play(file_path: PathBuf, debug: bool) {
    // Read file
    let markdown = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error: Failed to read file '{}'", file_path.display());
            eprintln!("Reason: {}", err);
            process::exit(1);
        }
    };

    // Run player mode
    if let Err(err) = tsumugai::cli::play::run_play(&markdown, debug) {
        eprintln!("Error: Player mode failed");
        eprintln!("Reason: {}", err);
        process::exit(1);
    }
}
