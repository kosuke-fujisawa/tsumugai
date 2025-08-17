//! Markdown parser for tsumugai scenario files.

use crate::ir::Program;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Missing required parameter '{param}' for command '{command}' at line {line}")]
    MissingParameter {
        command: String,
        param: String,
        line: usize,
    },
    #[error("Invalid value '{value}' for parameter '{param}' at line {line}")]
    InvalidValue {
        param: String,
        value: String,
        line: usize,
    },
    #[error("Undefined label '{label}' referenced at line {line}")]
    UndefinedLabel { label: String, line: usize },
    #[error("Duplicate label '{label}' defined at line {line}")]
    DuplicateLabel { label: String, line: usize },
    #[error("Invalid command syntax at line {line}: {content}")]
    InvalidSyntax { line: usize, content: String },
}

pub fn parse(markdown: &str) -> Result<Program, ParseError> {
    // Use async runtime to call the new async implementation
    let rt = tokio::runtime::Runtime::new()
        .map_err(|_| ParseError::InvalidSyntax { 
            line: 0, 
            content: "Failed to create async runtime".to_string() 
        })?;
    
    rt.block_on(crate::legacy_adapter::parse_legacy(markdown))
}

// Legacy Parser implementation has been moved to infrastructure/parsing.rs
// This module now acts as a thin wrapper for backward compatibility
