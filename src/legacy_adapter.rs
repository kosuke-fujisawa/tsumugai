//! Legacy adapter - Bridges old API to new implementation for backward compatibility
//!
//! # Adapter Pattern Usage
//!
//! This module implements the Adapter pattern to maintain backward compatibility
//! while migrating from the legacy IR-based API to the new domain-driven implementation.
//!
//! ## Key Adaptations
//!
//! - **Domain → Legacy IR**: Converts domain `StoryCommand` to legacy `Command`
//! - **Resource Mapping**: Adapts `ResourceId` to string-based legacy format
//! - **Error Translation**: Maps domain/infrastructure errors to legacy `ParseError`
//!
//! ## Migration Strategy
//!
//! 1. Legacy clients continue to use `parse_legacy()` function
//! 2. Internally, we use the new domain parser and convert the result
//! 3. Future versions will deprecate legacy functions in favor of new API
//! 4. Eventually, this module can be moved to a separate compatibility crate
//!
//! ## Usage
//!
//! ```rust,ignore
//! use tsumugai::legacy_adapter::parse_legacy;
//!
//! let program = parse_legacy(markdown_content).await?;
//! // Returns legacy Program compatible with existing engine
//! ```

use crate::domain::value_objects::*;
use crate::infrastructure::parsing::{MarkdownScenarioParser, ScenarioParser, ParseError as InfraParseError};
use crate::ir::{Command, Program, Value, Choice as LegacyChoice, Op, Cmp};

/// Trait for converting domain objects to legacy format
trait ToLegacy<T> {
    fn to_legacy(&self) -> T;
}

/// Helper function for converting resource commands
fn convert_resource_command<T, F>(
    resource: &ResourceId, 
    constructor: F
) -> T 
where 
    F: FnOnce(String) -> T
{
    constructor(resource.as_str().to_string())
}

/// Legacy parse function that maintains backward compatibility
pub async fn parse_legacy(markdown: &str) -> Result<Program, crate::ParseError> {
    let parser = MarkdownScenarioParser::with_default_id_generator();
    let scenario = parser.parse(markdown)
        .await
        .map_err(convert_parse_error)?;

    // Convert domain scenario to legacy Program
    let legacy_commands = scenario.commands()
        .iter()
        .map(convert_domain_command_to_legacy)
        .collect();

    Ok(Program::new(legacy_commands))
}

/// Convert domain StoryCommand to legacy Command
fn convert_domain_command_to_legacy(cmd: &StoryCommand) -> Command {
    match cmd {
        StoryCommand::Label { name } => Command::Label {
            name: name.as_str().to_string(),
        },
        StoryCommand::Jump { label } => Command::Jump {
            label: label.as_str().to_string(),
        },
        StoryCommand::Say { speaker, text } => Command::Say {
            speaker: speaker.as_str().to_string(),
            text: text.clone(),
        },
        StoryCommand::PlayBgm { resource } => convert_resource_command(resource, |name| Command::PlayBgm { name }),
        StoryCommand::PlaySe { resource } => convert_resource_command(resource, |name| Command::PlaySe { name }),
        StoryCommand::ShowImage { resource } => convert_resource_command(resource, |name| Command::ShowImage { file: name }),
        StoryCommand::PlayMovie { resource } => convert_resource_command(resource, |name| Command::PlayMovie { file: name }),
        StoryCommand::Wait { duration_seconds } => Command::Wait {
            secs: *duration_seconds,
        },
        StoryCommand::Branch { choices } => Command::Branch {
            choices: choices.iter().map(convert_choice_to_legacy).collect(),
        },
        StoryCommand::SetVariable { name, value } => Command::Set {
            name: name.as_str().to_string(),
            value: convert_story_value_to_legacy(value),
        },
        StoryCommand::ModifyVariable { name, operation, value } => Command::Modify {
            name: name.as_str().to_string(),
            op: convert_operation_to_legacy(operation),
            value: convert_story_value_to_legacy(value),
        },
        StoryCommand::JumpIf { variable, comparison, value, label } => Command::JumpIf {
            var: variable.as_str().to_string(),
            cmp: convert_comparison_to_legacy(comparison),
            value: convert_story_value_to_legacy(value),
            label: label.as_str().to_string(),
        },
    }
}

fn convert_choice_to_legacy(choice: &Choice) -> LegacyChoice {
    LegacyChoice {
        choice: choice.text().to_string(),
        label: choice.target_label().as_str().to_string(),
    }
}

impl ToLegacy<Value> for StoryValue {
    fn to_legacy(&self) -> Value {
        match self {
            StoryValue::Integer(i) => Value::Int(*i),
            StoryValue::Boolean(b) => Value::Bool(*b),
            StoryValue::Text(s) => Value::Str(s.clone()),
        }
    }
}

impl ToLegacy<Op> for VariableOperation {
    fn to_legacy(&self) -> Op {
        match self {
            VariableOperation::Add => Op::Add,
            VariableOperation::Subtract => Op::Sub,
        }
    }
}

impl ToLegacy<Cmp> for ComparisonOperation {
    fn to_legacy(&self) -> Cmp {
        match self {
            ComparisonOperation::Equal => Cmp::Eq,
            ComparisonOperation::NotEqual => Cmp::Ne,
            ComparisonOperation::LessThan => Cmp::Lt,
            ComparisonOperation::LessThanOrEqual => Cmp::Le,
            ComparisonOperation::GreaterThan => Cmp::Gt,
            ComparisonOperation::GreaterThanOrEqual => Cmp::Ge,
        }
    }
}

fn convert_story_value_to_legacy(value: &StoryValue) -> Value {
    value.to_legacy()
}

fn convert_operation_to_legacy(op: &VariableOperation) -> Op {
    op.to_legacy()
}

fn convert_comparison_to_legacy(cmp: &ComparisonOperation) -> Cmp {
    cmp.to_legacy()
}


fn convert_parse_error(err: InfraParseError) -> crate::ParseError {
    match err {
        InfraParseError::MissingParameter { command, param, line } => {
            crate::ParseError::MissingParameter { command, param, line }
        }
        InfraParseError::InvalidValue { param, value, line } => {
            crate::ParseError::InvalidValue { param, value, line }
        }
        InfraParseError::UndefinedLabel { label, line } => {
            crate::ParseError::UndefinedLabel { 
                label: label.as_str().to_string(), 
                line 
            }
        }
        InfraParseError::DuplicateLabel { label, line } => {
            crate::ParseError::DuplicateLabel { 
                label: label.as_str().to_string(), 
                line 
            }
        }
        InfraParseError::InvalidSyntax { line, content } => {
            crate::ParseError::InvalidSyntax { line, content }
        }
        InfraParseError::ValidationError { message } => {
            // TODO: Replace string parsing with structured error types for better reliability
            log::debug!("Parsing ValidationError message: {}", message);
            
            // Check if this is an undefined label error
            if message.contains("Undefined label") {
                // Extract label name and line from the message
                // This is a workaround for the domain error -> parse error conversion
                if let Some(start) = message.find("'") {
                    if let Some(end) = message.rfind("'") {
                        if start < end {
                            let label = message[start + 1..end].to_string();
                            // Extract line number 
                            if let Some(line_start) = message.find("line ") {
                                let line_part = &message[line_start + 5..];
                                if let Some(line_num) = line_part.split_whitespace().next() {
                                    if let Ok(line) = line_num.parse::<usize>() {
                                        return crate::ParseError::UndefinedLabel { label, line };
                                    }
                                }
                            }
                        }
                    }
                }
                log::trace!("Failed to parse undefined label from ValidationError, falling back to InvalidSyntax");
            }
            crate::ParseError::InvalidSyntax { 
                line: 0, 
                content: message 
            }
        }
    }
}