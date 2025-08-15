//! Markdown parser for tsumugai scenario files.

use crate::ir::{Choice, Cmp, Command, Op, Program, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum ParseMode {
    /// Standard parameter parsing
    Standard,
    /// BRANCH-specific parsing with comma-separated heuristics
    Branch,
}

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
    let parser = Parser::new(markdown);
    parser.parse()
}

struct Parser {
    lines: Vec<String>,
    current_line: usize,
    commands: Vec<Command>,
    labels: HashSet<String>,
}

impl Parser {
    fn new(markdown: &str) -> Self {
        let lines: Vec<String> = markdown.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_line: 0,
            commands: Vec::new(),
            labels: HashSet::new(),
        }
    }

    fn parse(mut self) -> Result<Program, ParseError> {
        while self.current_line < self.lines.len() {
            self.parse_line()?;
            self.current_line += 1;
        }

        self.validate_labels()?;
        Ok(Program::new(self.commands))
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

    fn parse_command(&mut self, cmd_str: &str) -> Result<Command, ParseError> {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ParseError::InvalidSyntax {
                line: self.current_line + 1,
                content: cmd_str.to_string(),
            });
        }

        let command_name = parts[0];
        let parse_mode = if command_name == "BRANCH" {
            ParseMode::Branch
        } else {
            ParseMode::Standard
        };
        let params = self.parse_params(&parts[1..], parse_mode)?;

        match command_name {
            "SAY" => {
                let speaker = self.require_param(&params, "speaker", command_name)?;
                let text = self.get_say_text()?;
                Ok(Command::Say { speaker, text })
            }
            "PLAY_BGM" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(Command::PlayBgm { name })
            }
            "PLAY_SE" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(Command::PlaySe { name })
            }
            "SHOW_IMAGE" => {
                let file = self.require_param(&params, "file", command_name)?;
                Ok(Command::ShowImage { file })
            }
            "PLAY_MOVIE" => {
                let file = self.require_param(&params, "file", command_name)?;
                Ok(Command::PlayMovie { file })
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
                Ok(Command::Wait { secs })
            }
            "BRANCH" => {
                let choices = self.parse_branch_choices(&params)?;
                Ok(Command::Branch { choices })
            }
            "LABEL" => {
                let name = self.require_param(&params, "name", command_name)?;
                if self.labels.contains(&name) {
                    return Err(ParseError::DuplicateLabel {
                        label: name,
                        line: self.current_line + 1,
                    });
                }
                self.labels.insert(name.clone());
                Ok(Command::Label { name })
            }
            "JUMP" => {
                let label = self.require_param(&params, "label", command_name)?;
                Ok(Command::Jump { label })
            }
            "SET" => {
                let name = self.require_param(&params, "name", command_name)?;
                let value =
                    self.parse_value(&self.require_param(&params, "value", command_name)?)?;
                Ok(Command::Set { name, value })
            }
            "MODIFY" => {
                let name = self.require_param(&params, "name", command_name)?;
                let op = self.parse_op(&self.require_param(&params, "op", command_name)?)?;
                let value =
                    self.parse_value(&self.require_param(&params, "value", command_name)?)?;
                Ok(Command::Modify { name, op, value })
            }
            "JUMP_IF" => {
                let var = self.require_param(&params, "var", command_name)?;
                let cmp = self.parse_cmp(&self.require_param(&params, "cmp", command_name)?)?;
                let value =
                    self.parse_value(&self.require_param(&params, "value", command_name)?)?;
                let label = self.require_param(&params, "label", command_name)?;
                Ok(Command::JumpIf {
                    var,
                    cmp,
                    value,
                    label,
                })
            }
            _ => Err(ParseError::InvalidSyntax {
                line: self.current_line + 1,
                content: cmd_str.to_string(),
            }),
        }
    }

    fn parse_params(
        &self,
        parts: &[&str],
        mode: ParseMode,
    ) -> Result<HashMap<String, String>, ParseError> {
        let mut params = HashMap::new();

        let full_params = parts.join(" ");

        // Use BRANCH-specific parsing when mode is Branch, or fallback to comma detection
        if mode == ParseMode::Branch || full_params.contains(',') {
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

        // Handle the special case where we have duplicate choice keys in comma-separated format
        // We need to parse the original command string directly for this case
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
                            choices.push(Choice { choice, label });
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

    fn parse_value(&self, s: &str) -> Result<Value, ParseError> {
        if let Ok(i) = s.parse::<i32>() {
            Ok(Value::Int(i))
        } else if let Ok(b) = s.parse::<bool>() {
            Ok(Value::Bool(b))
        } else {
            Ok(Value::Str(s.to_string()))
        }
    }

    fn parse_op(&self, s: &str) -> Result<Op, ParseError> {
        match s {
            "add" => Ok(Op::Add),
            "sub" => Ok(Op::Sub),
            _ => Err(ParseError::InvalidValue {
                param: "op".to_string(),
                value: s.to_string(),
                line: self.current_line + 1,
            }),
        }
    }

    fn parse_cmp(&self, s: &str) -> Result<Cmp, ParseError> {
        match s {
            "eq" => Ok(Cmp::Eq),
            "ne" => Ok(Cmp::Ne),
            "lt" => Ok(Cmp::Lt),
            "le" => Ok(Cmp::Le),
            "gt" => Ok(Cmp::Gt),
            "ge" => Ok(Cmp::Ge),
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

    fn validate_labels(&self) -> Result<(), ParseError> {
        for (idx, command) in self.commands.iter().enumerate() {
            match command {
                Command::Jump { label } => {
                    if !self.labels.contains(label) {
                        return Err(ParseError::UndefinedLabel {
                            label: label.clone(),
                            line: idx + 1,
                        });
                    }
                }
                Command::JumpIf { label, .. } => {
                    if !self.labels.contains(label) {
                        return Err(ParseError::UndefinedLabel {
                            label: label.clone(),
                            line: idx + 1,
                        });
                    }
                }
                Command::Branch { choices } => {
                    for choice in choices {
                        if !self.labels.contains(&choice.label) {
                            return Err(ParseError::UndefinedLabel {
                                label: choice.label.clone(),
                                line: idx + 1,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
