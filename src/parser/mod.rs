//! Markdown parser for tsumugai scenarios
//!
//! This module provides a simple parser that converts Markdown DSL to AST.

use crate::types::ast::{Ast, AstNode, Choice, Comparison, Expr, Operation};
use std::collections::HashMap;

pub mod check;

#[cfg(test)]
mod tests;

/// Parse markdown content into an AST
pub fn parse(markdown: &str) -> anyhow::Result<Ast> {
    let parser = MarkdownParser::new(markdown);
    parser.parse_with_validation(true)
}

/// Parse markdown content into an AST without label validation
/// This is useful for dry-run mode where undefined labels should be warnings, not errors
pub fn parse_unchecked(markdown: &str) -> anyhow::Result<Ast> {
    let parser = MarkdownParser::new(markdown);
    parser.parse_with_validation(false)
}

struct MarkdownParser {
    lines: Vec<String>,
    current_line: usize,
    nodes: Vec<AstNode>,
    labels: HashMap<String, usize>,
    conditions: std::collections::HashSet<String>,
}

impl MarkdownParser {
    fn new(markdown: &str) -> Self {
        let lines: Vec<String> = markdown.lines().map(|s| s.to_string()).collect();
        Self {
            lines,
            current_line: 0,
            nodes: Vec::new(),
            labels: HashMap::new(),
            conditions: std::collections::HashSet::new(),
        }
    }

    fn parse_with_validation(mut self, validate: bool) -> anyhow::Result<Ast> {
        // First pass: collect all nodes and conditions
        while self.current_line < self.lines.len() {
            self.parse_line()?;
            self.current_line += 1;
        }

        // Second pass: validate labels (only if validation is enabled)
        if validate {
            self.validate_labels()?;
        }

        Ok(Ast::with_conditions(
            self.nodes,
            self.labels,
            self.conditions,
        ))
    }

    fn parse_line(&mut self) -> anyhow::Result<()> {
        let line = self.lines[self.current_line].clone();
        let trimmed = line.trim();

        // Skip empty lines, comments, and headers (except :::conditions)
        if trimmed.is_empty() || trimmed.starts_with("<!--") {
            return Ok(());
        }

        // Check for :::conditions block
        if trimmed == ":::conditions" {
            self.parse_conditions_block()?;
            return Ok(());
        }

        // Check for :::when block
        if trimmed.starts_with(":::when") {
            let when_line = trimmed.to_string();
            self.parse_when_block(&when_line)?;
            return Ok(());
        }

        // Skip other ::: blocks (flag, vars, choices, route)
        if trimmed.starts_with(":::") {
            self.skip_block()?;
            return Ok(());
        }

        // Parse scene headers
        if trimmed.starts_with("# scene:") || trimmed.starts_with("#scene:") {
            let scene_line = trimmed.to_string();
            self.parse_scene_header(&scene_line)?;
            return Ok(());
        }

        // Skip other headers
        if trimmed.starts_with('#') {
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

    fn parse_when_block(&mut self, line: &str) -> anyhow::Result<()> {
        // Extract condition expression (after ":::when ")
        let condition_str = if let Some(expr) = line.strip_prefix(":::when") {
            expr.trim()
        } else {
            anyhow::bail!("Invalid when block at line {}", self.current_line + 1);
        };

        // Parse the condition expression
        let condition = self.parse_expr(condition_str)?;

        // Move to next line
        self.current_line += 1;

        // Collect nodes in the when block
        let mut body = Vec::new();
        let mut depth = 1;

        while self.current_line < self.lines.len() {
            let line = &self.lines[self.current_line];
            let trimmed = line.trim();

            // Check for block end
            if trimmed == ":::" {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            } else if trimmed.starts_with(":::") && trimmed != ":::" {
                depth += 1;
            }

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("<!--") {
                self.current_line += 1;
                continue;
            }

            // Parse command
            if let Some(cmd_content) = self.extract_command(trimmed) {
                let node = self.parse_command(&cmd_content)?;
                body.push(node);
            }

            self.current_line += 1;
        }

        // Add WhenBlock node
        self.nodes.push(AstNode::WhenBlock { condition, body });

        Ok(())
    }

    fn parse_scene_header(&mut self, line: &str) -> anyhow::Result<()> {
        use crate::types::ast::{EndingKind, SceneMeta};

        // Extract scene name (after "# scene:" or "#scene:")
        let scene_name = if let Some(name) = line.strip_prefix("# scene:") {
            name.trim().to_string()
        } else if let Some(name) = line.strip_prefix("#scene:") {
            name.trim().to_string()
        } else {
            anyhow::bail!("Invalid scene header at line {}", self.current_line + 1);
        };

        // Check next line for @ending directive
        let mut ending = None;
        if self.current_line + 1 < self.lines.len() {
            let next_line = self.lines[self.current_line + 1].trim();
            if let Some(ending_str) = next_line.strip_prefix("@ending") {
                let ending_type = ending_str.trim();
                ending = Some(match ending_type.to_uppercase().as_str() {
                    "GOOD" => EndingKind::Good,
                    "BAD" => EndingKind::Bad,
                    "TRUE" => EndingKind::True,
                    "NORMAL" => EndingKind::Normal,
                    "" => EndingKind::Normal, // Default if no type specified
                    custom => EndingKind::Custom(custom.to_string()),
                });
                // Skip the @ending line
                self.current_line += 1;
            }
        }

        let meta = SceneMeta {
            name: scene_name.clone(),
            ending,
        };

        // Register scene name as a label
        self.labels.insert(scene_name, self.nodes.len());

        // Add scene node to AST
        self.nodes.push(AstNode::Scene { meta });

        Ok(())
    }

    fn parse_conditions_block(&mut self) -> anyhow::Result<()> {
        // Move to next line after :::conditions
        self.current_line += 1;

        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();

            // End of conditions block
            if line == ":::" {
                return Ok(());
            }

            // Skip empty lines
            if line.is_empty() {
                self.current_line += 1;
                continue;
            }

            // Parse condition name (single word per line)
            if !line.starts_with('-') && !line.starts_with('[') {
                self.conditions.insert(line.to_string());
            }

            self.current_line += 1;
        }

        anyhow::bail!(
            "Unclosed :::conditions block starting at line {}",
            self.current_line
        )
    }

    fn skip_block(&mut self) -> anyhow::Result<()> {
        // Skip until we find the closing ::: or end of file
        self.current_line += 1;

        let mut depth = 1; // Track nested blocks

        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();

            // Check for nested block start
            if line.starts_with(":::") && line != ":::" {
                depth += 1;
            } else if line == ":::" {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }

            self.current_line += 1;
        }

        // Reached end of file without closing, that's OK
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
            "PLAY_BGM" | "PLAY_MUSIC" => {
                let name = self.require_param_or(&params, &["name", "file"], command_name)?;
                Ok(AstNode::PlayBgm { name })
            }
            "PLAY_SE" => {
                let name = self.require_param_or(&params, &["name", "file"], command_name)?;
                Ok(AstNode::PlaySe { name })
            }
            "SHOW_IMAGE" => {
                let name = self.require_param_or(&params, &["name", "file"], command_name)?;
                let layer = params
                    .get("layer")
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());
                Ok(AstNode::ShowImage { layer, name })
            }
            "PLAY_MOVIE" => {
                let name = self.require_param_or(&params, &["name", "file"], command_name)?;
                Ok(AstNode::PlayMovie { name })
            }
            // Skip unsupported commands silently (they will be ignored in dry_run)
            "POSITION" | "SHOW_CHOICE" | "HIDE_IMAGE" | "STOP_BGM" | "STOP_SE" => {
                // Return a label with a special marker to skip
                Ok(AstNode::Label {
                    name: format!("__skip_{}", command_name),
                })
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
            "GOTO" => {
                let target = self.require_param(&params, "target", command_name)?;
                Ok(AstNode::Goto { target })
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
                let value = part[eq_pos + 1..].trim_end_matches(',').trim().to_string();
                params.entry(key).or_default().push(value);
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

    /// Try to get a parameter with alternative names (e.g., "name" or "file")
    fn require_param_or(
        &self,
        params: &HashMap<String, Vec<String>>,
        keys: &[&str],
        command: &str,
    ) -> anyhow::Result<String> {
        for key in keys {
            if let Some(value) = params.get(*key).and_then(|v| v.first()).cloned() {
                return Ok(value);
            }
        }
        anyhow::bail!(
            "Missing required parameter (one of {:?}) for command '{}' at line {}",
            keys,
            command,
            self.current_line + 1
        )
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
                // Remove [c] tags (click/continue markers)
                let text = line.replace("[c]", "").trim().to_string();
                return Ok(text);
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
            if let Some(num_str) = duration_str.strip_suffix('s') {
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
        if let Some(seconds_vec) = params.get("seconds").or_else(|| params.get("duration"))
            && let Some(seconds) = seconds_vec.first()
        {
            return seconds.parse::<f32>().map_err(|_| {
                anyhow::anyhow!(
                    "Invalid duration '{}' at line {}",
                    seconds,
                    self.current_line + 1
                )
            });
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
            let labels = params.get("label");
            let conditions = params.get("if");

            for (i, choice_text) in choice_values.iter().enumerate() {
                let target = if let Some(label_vec) = labels {
                    label_vec
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| choice_text.clone())
                } else {
                    choice_text.clone() // Use choice text as target label
                };

                let condition = if let Some(cond_vec) = conditions {
                    cond_vec.get(i).cloned()
                } else {
                    None
                };

                choices.push(Choice {
                    id: format!("choice_{}", choice_num),
                    label: choice_text.clone(),
                    target,
                    condition,
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
                AstNode::Goto { target } => {
                    referenced_labels.insert(target);
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

    /// Parse an expression string into an Expr AST
    fn parse_expr(&self, expr_str: &str) -> anyhow::Result<Expr> {
        let mut tokens = self.tokenize_expr(expr_str);
        self.parse_or_expr(&mut tokens)
    }

    /// Tokenize an expression string
    fn tokenize_expr(&self, expr_str: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut chars = expr_str.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                ' ' | '\t' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                '(' | ')' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push(ch.to_string());
                }
                '=' | '!' | '<' | '>' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    current.push(ch);
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch == '=' {
                            chars.next();
                            current.push(next_ch);
                        }
                    }
                    tokens.push(current.clone());
                    current.clear();
                }
                '&' | '|' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    current.push(ch);
                    if let Some(&next_ch) = chars.peek() {
                        if (ch == '&' && next_ch == '&') || (ch == '|' && next_ch == '|') {
                            chars.next();
                            current.push(next_ch);
                        }
                    }
                    tokens.push(current.clone());
                    current.clear();
                }
                '"' => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    // Parse string literal
                    let mut string_val = String::new();
                    while let Some(str_ch) = chars.next() {
                        if str_ch == '"' {
                            break;
                        }
                        string_val.push(str_ch);
                    }
                    tokens.push(format!("\"{}\"", string_val));
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    /// Parse OR expression: and_expr ('||' and_expr)*
    fn parse_or_expr(&self, tokens: &mut Vec<String>) -> anyhow::Result<Expr> {
        let mut left = self.parse_and_expr(tokens)?;

        while !tokens.is_empty() && tokens[0] == "||" {
            tokens.remove(0); // consume '||'
            let right = self.parse_and_expr(tokens)?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    /// Parse AND expression: not_expr ('&&' not_expr)*
    fn parse_and_expr(&self, tokens: &mut Vec<String>) -> anyhow::Result<Expr> {
        let mut left = self.parse_not_expr(tokens)?;

        while !tokens.is_empty() && tokens[0] == "&&" {
            tokens.remove(0); // consume '&&'
            let right = self.parse_not_expr(tokens)?;
            left = Expr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    /// Parse NOT expression: '!' not_expr | comparison_expr
    fn parse_not_expr(&self, tokens: &mut Vec<String>) -> anyhow::Result<Expr> {
        if !tokens.is_empty() && tokens[0] == "!" {
            tokens.remove(0); // consume '!'
            let expr = self.parse_not_expr(tokens)?;
            return Ok(Expr::Not(Box::new(expr)));
        }

        self.parse_comparison_expr(tokens)
    }

    /// Parse comparison expression: primary (('==' | '!=' | '<' | '>' | '<=' | '>=') primary)?
    fn parse_comparison_expr(&self, tokens: &mut Vec<String>) -> anyhow::Result<Expr> {
        let left = self.parse_primary(tokens)?;

        if !tokens.is_empty() {
            let op = &tokens[0];
            match op.as_str() {
                "==" => {
                    tokens.remove(0);
                    let right = self.parse_primary(tokens)?;
                    return Ok(Expr::Equal(Box::new(left), Box::new(right)));
                }
                "!=" => {
                    tokens.remove(0);
                    let right = self.parse_primary(tokens)?;
                    return Ok(Expr::NotEqual(Box::new(left), Box::new(right)));
                }
                "<" => {
                    tokens.remove(0);
                    let right = self.parse_primary(tokens)?;
                    return Ok(Expr::LessThan(Box::new(left), Box::new(right)));
                }
                "<=" => {
                    tokens.remove(0);
                    let right = self.parse_primary(tokens)?;
                    return Ok(Expr::LessThanOrEqual(Box::new(left), Box::new(right)));
                }
                ">" => {
                    tokens.remove(0);
                    let right = self.parse_primary(tokens)?;
                    return Ok(Expr::GreaterThan(Box::new(left), Box::new(right)));
                }
                ">=" => {
                    tokens.remove(0);
                    let right = self.parse_primary(tokens)?;
                    return Ok(Expr::GreaterThanOrEqual(Box::new(left), Box::new(right)));
                }
                _ => {}
            }
        }

        Ok(left)
    }

    /// Parse primary: 'true' | 'false' | number | string | variable
    fn parse_primary(&self, tokens: &mut Vec<String>) -> anyhow::Result<Expr> {
        if tokens.is_empty() {
            anyhow::bail!("Unexpected end of expression");
        }

        let token = tokens.remove(0);

        // Boolean literals
        if token == "true" || token == "TRUE" {
            return Ok(Expr::Bool(true));
        }
        if token == "false" || token == "FALSE" {
            return Ok(Expr::Bool(false));
        }

        // String literals
        if token.starts_with('"') && token.ends_with('"') {
            let string_val = token[1..token.len() - 1].to_string();
            return Ok(Expr::String(string_val));
        }

        // Number literals
        if let Ok(num) = token.parse::<i64>() {
            return Ok(Expr::Number(num));
        }

        // Variable reference
        Ok(Expr::Var(token))
    }
}
