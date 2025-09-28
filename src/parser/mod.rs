//! Markdown parser for tsumugai scenarios
//!
//! This module provides a simple parser that converts Markdown DSL to AST.

use crate::types::ast::{Ast, AstNode, Choice, Comparison, Operation};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// Parse markdown content into an AST
pub fn parse(markdown: &str) -> anyhow::Result<Ast> {
    let parser = MarkdownParser::new(markdown);
    parser.parse()
}

struct MarkdownParser {
    lines: Vec<String>,
    current_line: usize,
    nodes: Vec<AstNode>,
    labels: HashMap<String, usize>,
}

impl MarkdownParser {
    fn new(markdown: &str) -> Self {
        let lines: Vec<String> = markdown.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_line: 0,
            nodes: Vec::new(),
            labels: HashMap::new(),
        }
    }

    fn parse(mut self) -> anyhow::Result<Ast> {
        // First pass: collect all nodes
        while self.current_line < self.lines.len() {
            self.parse_line()?;
            self.current_line += 1;
        }

        // Second pass: validate labels
        self.validate_labels()?;

        Ok(Ast::new(self.nodes, self.labels))
    }

    fn parse_line(&mut self) -> anyhow::Result<()> {
        let line = &self.lines[self.current_line];
        let trimmed = line.trim();

        // Skip empty lines, comments, and headers
        if trimmed.is_empty() || trimmed.starts_with("<!--") || trimmed.starts_with('#') {
            return Ok(());
        }

        // Check for command syntax
        if let Some(cmd_content) = self.extract_command(trimmed) {
            let node = self.parse_command(&cmd_content)?;

            // If it's a label, record its position
            if let AstNode::Label { name } = &node {
                self.labels.insert(name.clone(), self.nodes.len());
            }

            self.nodes.push(node);
        }

        Ok(())
    }

    fn extract_command(&self, line: &str) -> Option<String> {
        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                return Some(line[1..end].to_string());
            }
        }
        None
    }

    fn parse_command(&mut self, cmd_str: &str) -> anyhow::Result<AstNode> {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty command at line {}", self.current_line + 1);
        }

        let command_name = parts[0];
        let params = self.parse_params(&parts[1..])?;

        match command_name {
            "SAY" => {
                let speaker = self.require_param(&params, "speaker", command_name)?;
                let text = self.get_say_text()?;
                Ok(AstNode::Say { speaker, text })
            }
            "PLAY_BGM" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(AstNode::PlayBgm { name })
            }
            "PLAY_SE" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(AstNode::PlaySe { name })
            }
            "SHOW_IMAGE" => {
                let name = self.require_param(&params, "name", command_name)?;
                let layer = params
                    .get("layer")
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());
                Ok(AstNode::ShowImage { layer, name })
            }
            "PLAY_MOVIE" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(AstNode::PlayMovie { name })
            }
            "WAIT" => {
                let seconds = self.parse_wait_duration(&parts, &params)?;
                Ok(AstNode::Wait { seconds })
            }
            "BRANCH" => {
                let choices = self.parse_branch_choices(&params)?;
                Ok(AstNode::Branch { choices })
            }
            "LABEL" => {
                let name = self.require_param(&params, "name", command_name)?;
                Ok(AstNode::Label { name })
            }
            "JUMP" => {
                let label = self.require_param(&params, "label", command_name)?;
                Ok(AstNode::Jump { label })
            }
            "JUMP_IF" => {
                let var = self.require_param(&params, "var", command_name)?;
                let cmp =
                    self.parse_comparison(&self.require_param(&params, "cmp", command_name)?)?;
                let value = self.require_param(&params, "value", command_name)?;
                let label = self.require_param(&params, "label", command_name)?;
                Ok(AstNode::JumpIf {
                    var,
                    cmp,
                    value,
                    label,
                })
            }
            "SET" => {
                let name = self.require_param(&params, "name", command_name)?;
                let value = self.require_param(&params, "value", command_name)?;
                Ok(AstNode::Set { name, value })
            }
            "MODIFY" => {
                let name = self.require_param(&params, "name", command_name)?;
                let op =
                    self.parse_operation(&self.require_param(&params, "op", command_name)?)?;
                let value = self.require_param(&params, "value", command_name)?;
                Ok(AstNode::Modify { name, op, value })
            }
            "CLEAR_LAYER" => {
                let layer = self.require_param(&params, "layer", command_name)?;
                Ok(AstNode::ClearLayer { layer })
            }
            _ => anyhow::bail!(
                "Unknown command '{}' at line {}",
                command_name,
                self.current_line + 1
            ),
        }
    }

    fn parse_params(&self, parts: &[&str]) -> anyhow::Result<HashMap<String, Vec<String>>> {
        let mut params: HashMap<String, Vec<String>> = HashMap::new();

        for part in parts {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].trim().to_string();
                let value = part[eq_pos + 1..].trim().to_string();
                params.entry(key).or_insert_with(Vec::new).push(value);
            }
        }

        Ok(params)
    }

    fn require_param(
        &self,
        params: &HashMap<String, Vec<String>>,
        key: &str,
        command: &str,
    ) -> anyhow::Result<String> {
        params
            .get(key)
            .and_then(|v| v.first())
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing required parameter '{}' for command '{}' at line {}",
                    key,
                    command,
                    self.current_line + 1
                )
            })
    }

    fn get_say_text(&mut self) -> anyhow::Result<String> {
        // Look ahead to next non-empty line for the actual text
        let mut text_line = self.current_line + 1;
        while text_line < self.lines.len() {
            let line = self.lines[text_line].trim();
            if !line.is_empty()
                && !line.starts_with('[')
                && !line.starts_with('#')
                && !line.starts_with("<!--")
            {
                return Ok(line.to_string());
            }
            text_line += 1;
        }
        anyhow::bail!("SAY command missing text at line {}", self.current_line + 1)
    }

    fn parse_wait_duration(
        &self,
        parts: &[&str],
        params: &HashMap<String, Vec<String>>,
    ) -> anyhow::Result<f32> {
        // Check if duration is in the command itself (e.g., "WAIT 1.5s")
        if parts.len() > 1 {
            let duration_str = parts[1];
            if duration_str.ends_with('s') {
                let num_str = &duration_str[..duration_str.len() - 1];
                return num_str.parse::<f32>().map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid duration '{}' at line {}",
                        duration_str,
                        self.current_line + 1
                    )
                });
            }
        }

        // Check params for "seconds" or "duration"
        if let Some(seconds_vec) = params.get("seconds").or_else(|| params.get("duration")) {
            if let Some(seconds) = seconds_vec.first() {
                return seconds.parse::<f32>().map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid duration '{}' at line {}",
                        seconds,
                        self.current_line + 1
                    )
                });
            }
        }

        anyhow::bail!(
            "WAIT command missing duration at line {}",
            self.current_line + 1
        )
    }

    fn parse_branch_choices(
        &self,
        params: &HashMap<String, Vec<String>>,
    ) -> anyhow::Result<Vec<Choice>> {
        let mut choices = Vec::new();
        let mut choice_num = 0;

        // Get all choice values
        if let Some(choice_values) = params.get("choice") {
            for choice_text in choice_values {
                choices.push(Choice {
                    id: format!("choice_{}", choice_num),
                    label: choice_text.clone(),
                    target: choice_text.clone(), // Use choice text as target label
                });
                choice_num += 1;
            }
        }

        if choices.is_empty() {
            anyhow::bail!(
                "BRANCH command missing choices at line {}",
                self.current_line + 1
            );
        }

        Ok(choices)
    }

    fn parse_comparison(&self, cmp_str: &str) -> anyhow::Result<Comparison> {
        match cmp_str {
            "eq" | "==" => Ok(Comparison::Equal),
            "ne" | "!=" => Ok(Comparison::NotEqual),
            "lt" | "<" => Ok(Comparison::LessThan),
            "le" | "<=" => Ok(Comparison::LessThanOrEqual),
            "gt" | ">" => Ok(Comparison::GreaterThan),
            "ge" | ">=" => Ok(Comparison::GreaterThanOrEqual),
            _ => anyhow::bail!(
                "Invalid comparison operator '{}' at line {}",
                cmp_str,
                self.current_line + 1
            ),
        }
    }

    fn parse_operation(&self, op_str: &str) -> anyhow::Result<Operation> {
        match op_str {
            "add" | "+" => Ok(Operation::Add),
            "sub" | "-" => Ok(Operation::Subtract),
            "mul" | "*" => Ok(Operation::Multiply),
            "div" | "/" => Ok(Operation::Divide),
            _ => anyhow::bail!(
                "Invalid operation '{}' at line {}",
                op_str,
                self.current_line + 1
            ),
        }
    }

    fn validate_labels(&self) -> anyhow::Result<()> {
        // Collect all referenced labels
        let mut referenced_labels = std::collections::HashSet::new();

        for node in &self.nodes {
            match node {
                AstNode::Jump { label } => {
                    referenced_labels.insert(label);
                }
                AstNode::JumpIf { label, .. } => {
                    referenced_labels.insert(label);
                }
                AstNode::Branch { choices } => {
                    for choice in choices {
                        referenced_labels.insert(&choice.target);
                    }
                }
                _ => {}
            }
        }

        // Check that all referenced labels exist
        for label in referenced_labels {
            if !self.labels.contains_key(label) {
                anyhow::bail!("Undefined label '{}' referenced in scenario", label);
            }
        }

        Ok(())
    }
}
