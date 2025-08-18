//! Markdown Parser - Infrastructure layer component
//!
//! This module handles the conversion from markdown text to intermediate representation.

use crate::application::api::ApiError;
use crate::ir::{Choice, Command, Program, Value};
use std::collections::{BTreeMap, HashMap};

/// Parse markdown content into a Program
pub fn parse_markdown(markdown: &str) -> Result<Program, ApiError> {
    let parser = MarkdownParser::new(markdown);
    parser.parse()
}

struct MarkdownParser {
    lines: Vec<String>,
    current_line: usize,
    commands: Vec<Command>,
    labels: BTreeMap<String, usize>,
}

impl MarkdownParser {
    fn new(markdown: &str) -> Self {
        let lines: Vec<String> = markdown.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_line: 0,
            commands: Vec::new(),
            labels: BTreeMap::new(),
        }
    }

    fn parse(mut self) -> Result<Program, ApiError> {
        // First pass: collect all commands
        while self.current_line < self.lines.len() {
            self.parse_line()?;
            self.current_line += 1;
        }

        // Second pass: validate labels
        self.validate_labels()?;

        Ok(Program {
            cmds: self.commands,
        })
    }

    fn parse_line(&mut self) -> Result<(), ApiError> {
        let line = &self.lines[self.current_line];
        let trimmed = line.trim();

        // Skip empty lines, comments, and headers
        if trimmed.is_empty() || trimmed.starts_with("<!--") || trimmed.starts_with('#') {
            return Ok(());
        }

        // Check for command syntax
        if let Some(cmd_content) = self.extract_command(trimmed) {
            let command = self.parse_command(&cmd_content)?;

            // If it's a label, record its position
            if let Command::Label { name } = &command {
                self.labels.insert(name.clone(), self.commands.len());
            }

            self.commands.push(command);
        }

        Ok(())
    }

    fn extract_command(&self, line: &str) -> Option<String> {
        if line.starts_with('[')
            && let Some(end) = line.find(']')
        {
            return Some(line[1..end].to_string());
        }
        None
    }

    fn parse_command(&mut self, cmd_str: &str) -> Result<Command, ApiError> {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ApiError::parse(
                self.current_line + 1,
                1,
                format!("Empty command: {cmd_str}"),
            ));
        }

        let command_name = parts[0];
        let params = self.parse_params(&parts[1..])?;

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
                let secs = self.parse_wait_duration(&parts, &params)?;
                Ok(Command::Wait { secs })
            }
            "BRANCH" => {
                let choices = self.parse_branch_choices()?;
                Ok(Command::Branch { choices })
            }
            "LABEL" => {
                let name = self.require_param(&params, "name", command_name)?;
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
            _ => Err(ApiError::parse(
                self.current_line + 1,
                1,
                format!("Unknown command: {command_name}"),
            )),
        }
    }

    fn parse_params(&self, parts: &[&str]) -> Result<HashMap<String, String>, ApiError> {
        let mut params = HashMap::new();
        let full_params = parts.join(" ");

        if full_params.contains(',') {
            let param_parts: Vec<&str> = full_params.split(',').collect();
            for part in param_parts {
                self.parse_single_param(part.trim(), &mut params)?;
            }
        } else {
            for part in parts {
                self.parse_single_param(part, &mut params)?;
            }
        }

        Ok(params)
    }

    fn parse_single_param(
        &self,
        part: &str,
        params: &mut HashMap<String, String>,
    ) -> Result<(), ApiError> {
        if let Some(eq_pos) = part.find('=') {
            let key = part[..eq_pos].trim().to_string();
            let value = part[eq_pos + 1..].trim().to_string();
            let clean_value = if value.starts_with('"') && value.ends_with('"') {
                value[1..value.len() - 1].to_string()
            } else {
                value
            };
            params.insert(key, clean_value);
        }
        Ok(())
    }

    fn require_param(
        &self,
        params: &HashMap<String, String>,
        param: &str,
        command: &str,
    ) -> Result<String, ApiError> {
        params.get(param).cloned().ok_or_else(|| {
            ApiError::parse(
                self.current_line + 1,
                1,
                format!("Missing required parameter '{param}' for command '{command}'"),
            )
        })
    }

    fn get_say_text(&mut self) -> Result<String, ApiError> {
        let current_line_content = &self.lines[self.current_line];

        // Check if text is on the same line after the ]
        if let Some(bracket_end) = current_line_content.find(']') {
            let after_bracket = current_line_content[bracket_end + 1..].trim();
            if !after_bracket.is_empty() {
                return Ok(after_bracket.to_string());
            }
        }

        // Check next line
        if self.current_line + 1 < self.lines.len() {
            let next_line = &self.lines[self.current_line + 1];
            if !next_line.trim().is_empty() && !next_line.starts_with('[') {
                self.current_line += 1;
                return Ok(next_line.trim().to_string());
            }
        }

        Ok(String::new())
    }

    fn parse_wait_duration(
        &self,
        parts: &[&str],
        params: &HashMap<String, String>,
    ) -> Result<f32, ApiError> {
        if let Some(secs_str) = params.get("secs") {
            self.parse_float(secs_str)
        } else if parts.len() > 1 {
            let time_str = parts[1];
            let clean_str = if let Some(stripped) = time_str.strip_suffix('s') {
                stripped
            } else {
                time_str
            };
            self.parse_float(clean_str)
        } else {
            Err(ApiError::parse(
                self.current_line + 1,
                1,
                "Missing duration for WAIT command",
            ))
        }
    }

    fn parse_branch_choices(&mut self) -> Result<Vec<Choice>, ApiError> {
        let mut choices = Vec::new();
        let current_line_content = &self.lines[self.current_line];

        if let Some(bracket_start) = current_line_content.find('[')
            && let Some(bracket_end) = current_line_content.find(']')
        {
            let command_content = &current_line_content[bracket_start + 1..bracket_end];
            let parts: Vec<&str> = command_content.split_whitespace().collect();

            if parts.len() > 1 && parts[0] == "BRANCH" {
                let params_str = parts[1..].join(" ");
                let param_pairs = self.tokenize_quoted_params(&params_str);

                for pair in param_pairs {
                    let pair = pair.trim();
                    let mut current_choice: Option<String> = None;
                    let mut current_label: Option<String> = None;

                    // Parse key=value pairs with quote support
                    let tokens = self.tokenize_key_values(pair);
                    for token in tokens {
                        if let Some(eq_pos) = token.find('=') {
                            let key = token[..eq_pos].trim();
                            let mut value = token[eq_pos + 1..].trim();

                            // Remove quotes if present
                            if (value.starts_with('"') && value.ends_with('"'))
                                || (value.starts_with('\'') && value.ends_with('\''))
                            {
                                value = &value[1..value.len() - 1];
                            }

                            match key {
                                "choice" => current_choice = Some(value.to_string()),
                                "label" => current_label = Some(value.to_string()),
                                _ => {}
                            }
                        }
                    }

                    if let (Some(choice), Some(label)) = (current_choice, current_label) {
                        choices.push(Choice { choice, label });
                    }
                }
            }
        }

        if choices.is_empty() {
            return Err(ApiError::parse(
                self.current_line + 1,
                1,
                "BRANCH command requires at least one choice",
            ));
        }

        Ok(choices)
    }

    /// Tokenize comma-separated parameters, respecting quotes
    fn tokenize_quoted_params(&self, input: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
        let chars = input.chars();

        for c in chars {
            match c {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = c;
                    current.push(c);
                }
                c if c == quote_char && in_quotes => {
                    in_quotes = false;
                    current.push(c);
                }
                ',' if !in_quotes => {
                    if !current.trim().is_empty() {
                        result.push(current.trim().to_string());
                    }
                    current.clear();
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }

    /// Tokenize key=value pairs within a single parameter group
    fn tokenize_key_values(&self, input: &str) -> Vec<String> {
        // For now, simple space-based split since key=value shouldn't contain spaces
        // unless quoted values, which are handled at the parse level
        input.split_whitespace().map(|s| s.to_string()).collect()
    }

    fn parse_value(&self, s: &str) -> Result<Value, ApiError> {
        if let Ok(i) = s.parse::<i32>() {
            Ok(Value::Int(i))
        } else if let Ok(b) = s.parse::<bool>() {
            Ok(Value::Bool(b))
        } else {
            Ok(Value::Str(s.to_string()))
        }
    }

    fn parse_op(&self, s: &str) -> Result<crate::ir::Op, ApiError> {
        match s {
            "add" => Ok(crate::ir::Op::Add),
            "sub" => Ok(crate::ir::Op::Sub),
            _ => Err(ApiError::parse(
                self.current_line + 1,
                1,
                format!("Invalid operation: {s}"),
            )),
        }
    }

    fn parse_cmp(&self, s: &str) -> Result<crate::ir::Cmp, ApiError> {
        match s {
            "eq" => Ok(crate::ir::Cmp::Eq),
            "ne" => Ok(crate::ir::Cmp::Ne),
            "lt" => Ok(crate::ir::Cmp::Lt),
            "le" => Ok(crate::ir::Cmp::Le),
            "gt" => Ok(crate::ir::Cmp::Gt),
            "ge" => Ok(crate::ir::Cmp::Ge),
            _ => Err(ApiError::parse(
                self.current_line + 1,
                1,
                format!("Invalid comparison: {s}"),
            )),
        }
    }

    fn parse_float(&self, s: &str) -> Result<f32, ApiError> {
        s.parse().map_err(|_| {
            ApiError::parse(
                self.current_line + 1,
                1,
                format!("Invalid float value: {s}"),
            )
        })
    }

    fn validate_labels(&self) -> Result<(), ApiError> {
        // Check for undefined labels in jumps and branches
        for (index, cmd) in self.commands.iter().enumerate() {
            match cmd {
                Command::Jump { label } => {
                    if !self.labels.contains_key(label) {
                        return Err(ApiError::parse(
                            index + 1,
                            1,
                            format!(
                                "Undefined label '{}'. Available labels: {:?}",
                                label,
                                self.labels.keys().collect::<Vec<_>>()
                            ),
                        ));
                    }
                }
                Command::JumpIf { label, .. } => {
                    if !self.labels.contains_key(label) {
                        return Err(ApiError::parse(
                            index + 1,
                            1,
                            format!(
                                "Undefined label '{}'. Available labels: {:?}",
                                label,
                                self.labels.keys().collect::<Vec<_>>()
                            ),
                        ));
                    }
                }
                Command::Branch { choices } => {
                    for choice in choices {
                        if !self.labels.contains_key(&choice.label) {
                            return Err(ApiError::parse(
                                index + 1,
                                1,
                                format!(
                                    "Undefined label '{}' in branch choice '{}'. Available labels: {:?}",
                                    choice.label,
                                    choice.choice,
                                    self.labels.keys().collect::<Vec<_>>()
                                ),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
