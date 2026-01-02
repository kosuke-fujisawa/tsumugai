//! Reference integrity checking implementation

use crate::lint::config::LintConfig;
use crate::lint::{LintIssue, LintLevel, LintResult};
use crate::types::ast::{Ast, AstNode};
use std::collections::HashSet;

/// Check reference integrity (labels, conditions, etc.)
pub fn check(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    if config.references.check_labels {
        check_labels(ast, result);
    }

    // Check for undeclared conditions
    check_conditions(ast, result);
}

fn check_labels(_ast: &Ast, _result: &mut LintResult) {
    // Labels are already validated during parsing
    // This function can be extended for additional label checks
}

fn check_conditions(ast: &Ast, result: &mut LintResult) {
    // Collect used conditions
    let mut used_conditions = HashSet::new();

    for node in &ast.nodes {
        if let AstNode::Branch { choices } = node {
            for choice in choices {
                if let Some(ref condition) = choice.condition {
                    used_conditions.insert(condition.clone());

                    // Check if condition is declared
                    if !ast.conditions.contains(condition) {
                        result.add_issue(LintIssue {
                            level: LintLevel::Warning,
                            message: format!("Undeclared condition '{}' used in choice", condition),
                            line: 0, // TODO: Track line numbers in AST
                            column: 0,
                            category: "references".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Check for unused declared conditions
    for declared_condition in &ast.conditions {
        if !used_conditions.contains(declared_condition) {
            result.add_issue(LintIssue {
                level: LintLevel::Info,
                message: format!("Declared condition '{}' is never used", declared_condition),
                line: 0,
                column: 0,
                category: "references".to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn references_check_undeclared_condition() {
        let markdown = r#"
[BRANCH choice=Right if=undeclared label=right, choice=Left label=left]

[LABEL name=right]
[SAY speaker=A]
Right

[LABEL name=left]
[SAY speaker=A]
Left
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let config = LintConfig::default();

        check(&ast, &mut result, &config);

        assert_eq!(result.warning_count, 1);
        assert!(result.issues[0].message.contains("Undeclared"));
    }

    #[test]
    fn references_check_unused_condition() {
        let markdown = r#"
:::conditions
unused_condition
can_go_right
:::

[BRANCH choice=Right if=can_go_right label=right, choice=Left label=left]

[LABEL name=right]
[SAY speaker=A]
Right

[LABEL name=left]
[SAY speaker=A]
Left
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let config = LintConfig::default();

        check(&ast, &mut result, &config);

        assert_eq!(result.info_count, 1);
        assert!(result.issues[0].message.contains("never used"));
    }
}
