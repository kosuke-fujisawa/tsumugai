//! Infrastructure for parsing scenarios from various formats

use crate::domain::entities::Scenario;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::*;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

/// Trait for parsing scenarios from different formats
#[async_trait]
pub trait ScenarioParser: Send + Sync {
    /// Parse a scenario from text content
    async fn parse(&self, content: &str) -> Result<Scenario, ParseError>;

    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<&'static str>;
}

/// Markdown parser implementation that adapts the legacy parser
pub struct MarkdownScenarioParser {
    id_generator: Box<dyn IdGenerator + Send + Sync>,
}

impl MarkdownScenarioParser {
    pub fn new(id_generator: Box<dyn IdGenerator + Send + Sync>) -> Self {
        Self { id_generator }
    }

    pub fn with_default_id_generator() -> Self {
        Self::new(Box::new(DefaultIdGenerator))
    }
}

#[async_trait]
impl ScenarioParser for MarkdownScenarioParser {
    async fn parse(&self, content: &str) -> Result<Scenario, ParseError> {
        let parser = MarkdownParser::new(content);
        let (commands, title) = parser.parse()?;

        let id = self.id_generator.generate_id(&title);
        let scenario = Scenario::new(id, title, commands);

        // Validate the scenario using domain rules
        scenario
            .validate_labels()
            .map_err(|e| ParseError::ValidationError {
                message: e.to_string(),
            })?;

        Ok(scenario)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["md", "markdown"]
    }
}

/// Trait for generating scenario IDs
pub trait IdGenerator {
    fn generate_id(&self, title: &str) -> ScenarioId;
}

/// Default ID generator that uses the title or a hash
pub struct DefaultIdGenerator;

impl IdGenerator for DefaultIdGenerator {
    fn generate_id(&self, title: &str) -> ScenarioId {
        if title.is_empty() {
            ScenarioId::from("untitled")
        } else {
            // Simple slug generation - in real implementation, might use proper slug library
            let slug = title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();
            ScenarioId::from(slug)
        }
    }
}

/// Internal markdown parser (adapts legacy parsing logic)
struct MarkdownParser {
    lines: Vec<String>,
    current_line: usize,
    commands: Vec<StoryCommand>,
    labels: HashSet<LabelName>,
}

impl MarkdownParser {
    fn new(markdown: &str) -> Self {
        let lines: Vec<String> = markdown.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_line: 0,
            commands: Vec::new(),
            labels: HashSet::new(),
        }
    }

    fn parse(mut self) -> Result<(Vec<StoryCommand>, String), ParseError> {
        let title = self.extract_title();

        while self.current_line < self.lines.len() {
            self.parse_line()?;
            self.current_line += 1;
        }

        Ok((self.commands, title))
    }

    fn extract_title(&self) -> String {
        // Look for a title in the first few lines
        for line in self.lines.iter().take(5) {
            let line = line.trim();
            if let Some(stripped) = line.strip_prefix("# ") {
                return stripped.trim().to_string();
            }
        }
        "Untitled Scenario".to_string()
    }

    fn parse_line(&mut self) -> Result<(), ParseError> {
        let line = self.lines[self.current_line].trim();

        if line.is_empty() || line.starts_with("<!--") || line.starts_with('#') {
            return Ok(());
        }

        if let Some(cmd_str) = self.extract_command(line) {
            let command = self.parse_command(&cmd_str)?;
            self.commands.push(command);
        }

        Ok(())
    }

    fn extract_command(&self, line: &str) -> Option<String> {
        if line.starts_with('[') && line.contains(']') {
            let end = line.find(']').unwrap();
            Some(line[1..end].to_string())
        } else {
            None
        }
    }

    fn parse_command(&mut self, cmd_str: &str) -> Result<StoryCommand, ParseError> {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ParseError::InvalidSyntax {
                line: self.current_line + 1,
                content: cmd_str.to_string(),
            });
        }

        let command_name = parts[0];
        let params = self.parse_params(&parts[1..])?;

        match command_name {
            "SAY" => {
                let speaker = self.require_param(&params, "speaker", command_name)?;
                let text = self.get_say_text()?;
                Ok(StoryCommand::Say {
                    speaker: SpeakerName::from(speaker),
                    text,
                })
            }
            "PLAY_BGM" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(StoryCommand::PlayBgm {
                    resource: ResourceId::from(name),
                })
            }
            "PLAY_SE" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(StoryCommand::PlaySe {
                    resource: ResourceId::from(name),
                })
            }
            "SHOW_IMAGE" => {
                let file = self.require_param(&params, "file", command_name)?;
                Ok(StoryCommand::ShowImage {
                    resource: ResourceId::from(file),
                })
            }
            "PLAY_MOVIE" => {
                let file = self.require_param(&params, "file", command_name)?;
                Ok(StoryCommand::PlayMovie {
                    resource: ResourceId::from(file),
                })
            }
            "WAIT" => {
                let secs = if let Some(secs_str) = params.get("secs") {
                    self.parse_float(secs_str, "secs")?
                } else if parts.len() > 1 {
                    let time_str = parts[1];
                    if let Some(stripped) = time_str.strip_suffix('s') {
                        self.parse_float(stripped, "secs")?
                    } else {
                        self.parse_float(time_str, "secs")?
                    }
                } else {
                    return Err(ParseError::MissingParameter {
                        command: command_name.to_string(),
                        param: "secs".to_string(),
                        line: self.current_line + 1,
                    });
                };
                Ok(StoryCommand::Wait {
                    duration_seconds: secs,
                })
            }
            "BRANCH" => {
                let choices = self.parse_branch_choices(&params)?;
                Ok(StoryCommand::Branch { choices })
            }
            "LABEL" => {
                let name = self.require_param(&params, "name", command_name)?;
                let label_name = LabelName::from(name);
                if self.labels.contains(&label_name) {
                    return Err(ParseError::DuplicateLabel {
                        label: label_name,
                        line: self.current_line + 1,
                    });
                }
                self.labels.insert(label_name.clone());
                Ok(StoryCommand::Label { name: label_name })
            }
            "JUMP" => {
                let label = self.require_param(&params, "label", command_name)?;
                Ok(StoryCommand::Jump {
                    label: LabelName::from(label),
                })
            }
            "SET" => {
                let name = self.require_param(&params, "name", command_name)?;
                let value =
                    self.parse_value(&self.require_param(&params, "value", command_name)?)?;
                Ok(StoryCommand::SetVariable {
                    name: VariableName::from(name),
                    value,
                })
            }
            "MODIFY" => {
                let name = self.require_param(&params, "name", command_name)?;
                let op = self.parse_op(&self.require_param(&params, "op", command_name)?)?;
                let value =
                    self.parse_value(&self.require_param(&params, "value", command_name)?)?;
                Ok(StoryCommand::ModifyVariable {
                    name: VariableName::from(name),
                    operation: op,
                    value,
                })
            }
            "JUMP_IF" => {
                let var = self.require_param(&params, "var", command_name)?;
                let cmp = self.parse_cmp(&self.require_param(&params, "cmp", command_name)?)?;
                let value =
                    self.parse_value(&self.require_param(&params, "value", command_name)?)?;
                let label = self.require_param(&params, "label", command_name)?;
                Ok(StoryCommand::JumpIf {
                    variable: VariableName::from(var),
                    comparison: cmp,
                    value,
                    label: LabelName::from(label),
                })
            }
            _ => Err(ParseError::InvalidSyntax {
                line: self.current_line + 1,
                content: cmd_str.to_string(),
            }),
        }
    }

    fn parse_params(&self, parts: &[&str]) -> Result<HashMap<String, String>, ParseError> {
        let mut params = HashMap::new();
        let full_params = parts.join(" ");

        // Use comma detection for parameters
        if full_params.contains(',') {
            let param_parts: Vec<&str> = full_params.split(',').collect();
            for part in param_parts {
                let part = part.trim();
                if let Some(eq_pos) = part.find('=') {
                    let key = part[..eq_pos].trim().to_string();
                    let value = part[eq_pos + 1..].trim().to_string();
                    let value = if value.starts_with('"') && value.ends_with('"') {
                        value[1..value.len() - 1].to_string()
                    } else {
                        value
                    };
                    params.insert(key, value);
                }
            }
        } else {
            // Handle space-separated parameters
            for part in parts {
                if let Some(eq_pos) = part.find('=') {
                    let key = part[..eq_pos].to_string();
                    let value = part[eq_pos + 1..].to_string();
                    let value = if value.starts_with('"') && value.ends_with('"') {
                        value[1..value.len() - 1].to_string()
                    } else {
                        value
                    };
                    params.insert(key, value);
                }
            }
        }

        Ok(params)
    }

    fn require_param(
        &self,
        params: &HashMap<String, String>,
        param: &str,
        command: &str,
    ) -> Result<String, ParseError> {
        params
            .get(param)
            .cloned()
            .ok_or(ParseError::MissingParameter {
                command: command.to_string(),
                param: param.to_string(),
                line: self.current_line + 1,
            })
    }

    fn get_say_text(&mut self) -> Result<String, ParseError> {
        let current_line_content = &self.lines[self.current_line];

        if let Some(bracket_end) = current_line_content.find(']') {
            let after_bracket = &current_line_content[bracket_end + 1..].trim();
            if !after_bracket.is_empty() {
                return Ok(after_bracket.to_string());
            }
        }

        if self.current_line + 1 < self.lines.len() {
            let next_line = &self.lines[self.current_line + 1];
            if !next_line.trim().is_empty() && !next_line.starts_with('[') {
                self.current_line += 1;
                return Ok(next_line.trim().to_string());
            }
        }

        Ok(String::new())
    }

    fn parse_branch_choices(
        &self,
        _params: &HashMap<String, String>,
    ) -> Result<Vec<Choice>, ParseError> {
        let mut choices = Vec::new();

        // Parse from the original command string
        let current_line_content = &self.lines[self.current_line];
        if let Some(bracket_start) = current_line_content.find('[') {
            if let Some(bracket_end) = current_line_content.find(']') {
                let command_content = &current_line_content[bracket_start + 1..bracket_end];
                let parts: Vec<&str> = command_content.split_whitespace().collect();
                if parts.len() > 1 && parts[0] == "BRANCH" {
                    let params_str = parts[1..].join(" ");
                    let param_pairs: Vec<&str> = params_str.split(',').collect();

                    for pair in param_pairs {
                        let pair = pair.trim();
                        let key_value_pairs: Vec<&str> = pair.split_whitespace().collect();

                        let mut current_choice: Option<String> = None;
                        let mut current_label: Option<String> = None;

                        for kv in key_value_pairs {
                            if let Some(eq_pos) = kv.find('=') {
                                let key = kv[..eq_pos].trim();
                                let value = kv[eq_pos + 1..].trim();

                                if key == "choice" {
                                    current_choice = Some(value.to_string());
                                } else if key == "label" {
                                    current_label = Some(value.to_string());
                                }
                            }
                        }

                        if let (Some(choice), Some(label)) = (current_choice, current_label) {
                            choices.push(Choice::new(choice, LabelName::from(label)));
                        }
                    }
                }
            }
        }

        if choices.is_empty() {
            return Err(ParseError::MissingParameter {
                command: "BRANCH".to_string(),
                param: "choice".to_string(),
                line: self.current_line + 1,
            });
        }

        Ok(choices)
    }

    fn parse_value(&self, s: &str) -> Result<StoryValue, ParseError> {
        if let Ok(i) = s.parse::<i32>() {
            Ok(StoryValue::Integer(i))
        } else if let Ok(b) = s.parse::<bool>() {
            Ok(StoryValue::Boolean(b))
        } else {
            Ok(StoryValue::Text(s.to_string()))
        }
    }

    fn parse_op(&self, s: &str) -> Result<VariableOperation, ParseError> {
        match s {
            "add" => Ok(VariableOperation::Add),
            "sub" => Ok(VariableOperation::Subtract),
            _ => Err(ParseError::InvalidValue {
                param: "op".to_string(),
                value: s.to_string(),
                line: self.current_line + 1,
            }),
        }
    }

    fn parse_cmp(&self, s: &str) -> Result<ComparisonOperation, ParseError> {
        match s {
            "eq" => Ok(ComparisonOperation::Equal),
            "ne" => Ok(ComparisonOperation::NotEqual),
            "lt" => Ok(ComparisonOperation::LessThan),
            "le" => Ok(ComparisonOperation::LessThanOrEqual),
            "gt" => Ok(ComparisonOperation::GreaterThan),
            "ge" => Ok(ComparisonOperation::GreaterThanOrEqual),
            _ => Err(ParseError::InvalidValue {
                param: "cmp".to_string(),
                value: s.to_string(),
                line: self.current_line + 1,
            }),
        }
    }

    fn parse_float(&self, s: &str, param: &str) -> Result<f32, ParseError> {
        s.parse().map_err(|_| ParseError::InvalidValue {
            param: param.to_string(),
            value: s.to_string(),
            line: self.current_line + 1,
        })
    }
}

/// Parsing errors
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
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
    UndefinedLabel { label: LabelName, line: usize },
    #[error("Duplicate label '{label}' defined at line {line}")]
    DuplicateLabel { label: LabelName, line: usize },
    #[error("Invalid command syntax at line {line}: {content}")]
    InvalidSyntax { line: usize, content: String },
    #[error("Validation error: {message}")]
    ValidationError { message: String },
}

impl From<ParseError> for DomainError {
    fn from(error: ParseError) -> Self {
        match error {
            ParseError::UndefinedLabel { label, line } => {
                DomainError::UndefinedLabel { label, line }
            }
            ParseError::DuplicateLabel { label, line } => {
                DomainError::DuplicateLabel { label, line }
            }
            _ => DomainError::invalid_scenario(error.to_string()),
        }
    }
}
