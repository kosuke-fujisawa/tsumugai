//! Lint module for scenario quality checking
//!
//! This module provides comprehensive linting capabilities for scenarios:
//! - Command syntax error detection
//! - Reference integrity checks (labels, variables)
//! - Deprecated/dangerous pattern detection
//! - Scenario flow analysis (reachability, loops)

use crate::types::ast::Ast;
use serde::{Deserialize, Serialize};

pub mod checks;
pub mod config;

/// Lint severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LintLevel {
    /// Error: must be fixed
    Error,
    /// Warning: should be reviewed
    Warning,
    /// Info: for your information
    Info,
}

/// A lint issue found in the scenario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintIssue {
    /// Severity level
    pub level: LintLevel,
    /// Issue message
    pub message: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Category of the issue
    pub category: String,
}

/// Result of linting a scenario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintResult {
    /// Issues found
    pub issues: Vec<LintIssue>,
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
    /// Number of info messages
    pub info_count: usize,
}

impl LintResult {
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
        }
    }

    pub fn add_issue(&mut self, issue: LintIssue) {
        match issue.level {
            LintLevel::Error => self.error_count += 1,
            LintLevel::Warning => self.warning_count += 1,
            LintLevel::Info => self.info_count += 1,
        }
        self.issues.push(issue);
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

impl Default for LintResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Lint an AST with default configuration
pub fn lint(ast: &Ast) -> LintResult {
    let config = config::LintConfig::default();
    lint_with_config(ast, &config)
}

/// Lint an AST with custom configuration
pub fn lint_with_config(ast: &Ast, config: &config::LintConfig) -> LintResult {
    let mut result = LintResult::new();

    // Run all enabled checks
    if config.syntax.enabled {
        checks::syntax::check(ast, &mut result, config);
    }

    if config.references.enabled {
        checks::references::check(ast, &mut result, config);
    }

    if config.quality.enabled {
        checks::quality::check(ast, &mut result, config);
    }

    if config.flow.enabled {
        checks::flow::check(ast, &mut result, config);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn lint_empty_scenario() {
        let markdown = "";
        let ast = parse(markdown).unwrap();
        let result = lint(&ast);

        assert!(result.is_clean());
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn lint_simple_valid_scenario() {
        let markdown = r#"
[SAY speaker=Alice]
Hello, world!

[LABEL name=end]
[SAY speaker=Alice]
Goodbye!
"#;

        let ast = parse(markdown).unwrap();
        let result = lint(&ast);

        // Should be clean or have minimal warnings
        assert!(!result.has_errors());
    }
}
