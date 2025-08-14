use tsumugai::{parse, MockEngine, Engine};
use std::fs;

fn main() {
    println!("Testing new commands implementation...\n");
    
    let markdown = fs::read_to_string("test_commands.md").expect("Failed to read test file");
    
    println!("=== Raw markdown content ===");
    println!("{}", markdown);
    println!("================================\n");
    
    match parse(&markdown) {
        Ok(commands) => {
            println!("Parsed {} commands successfully!\n", commands.len());
            
            println!("=== Parsed commands ===");
            for (i, cmd) in commands.iter().enumerate() {
                println!("{}: {:?}", i, cmd);
            }
            println!();
            
            let mut engine = MockEngine::new();
            
            println!("=== Executing commands ===");
            engine.execute_all(&commands);
            
            println!("\n=== Final state ===");
            if let Some(affection) = engine.get_state("affection") {
                println!("affection = {}", affection);
            } else {
                println!("affection not set");
            }
            
            println!("\n=== Execution log ===");
            for log_entry in engine.log() {
                println!("  {}", log_entry);
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