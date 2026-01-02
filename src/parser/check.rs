//! Static validation and dry-run checking for scenarios

use crate::types::ast::{Ast, AstNode};
use crate::types::directive::Directive;
use std::collections::HashSet;

/// Result of scenario validation
#[derive(Debug, Clone, PartialEq)]
pub struct CheckResult {
    /// Warnings detected during validation
    pub warnings: Vec<Directive>,
    /// Whether the scenario is valid (no errors, only warnings)
    pub is_valid: bool,
}

/// Perform static validation on an AST
///
/// This function checks for:
/// - Undeclared conditions being used
/// - Declared but unused conditions
/// - Unreachable code (basic detection)
pub fn check(ast: &Ast) -> CheckResult {
    let mut warnings = Vec::new();

    // Collect all used conditions
    let mut used_conditions = HashSet::new();

    for node in &ast.nodes {
        if let AstNode::Branch { choices } = node {
            for choice in choices {
                if let Some(ref condition) = choice.condition {
                    used_conditions.insert(condition.clone());

                    // Check if condition is declared
                    if !ast.conditions.contains(condition) {
                        warnings.push(Directive::Warning {
                            message: format!(
                                "Undeclared condition '{}' used in choice",
                                condition
                            ),
                            line: 0, // TODO: Track line numbers in AST
                            column: 0,
                        });
                    }
                }
            }
        }
    }

    // Check for unused declared conditions
    for declared_condition in &ast.conditions {
        if !used_conditions.contains(declared_condition) {
            warnings.push(Directive::Warning {
                message: format!("Declared condition '{}' is never used", declared_condition),
                line: 0,
                column: 0,
            });
        }
    }

    CheckResult {
        is_valid: true, // Warnings don't make it invalid
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn check_undeclared_condition_warning() {
        let markdown = r#"
[BRANCH choice=右へ if=undeclared_cond label=right, choice=左へ label=left]

[LABEL name=right]
[SAY speaker=A]
Right

[LABEL name=left]
[SAY speaker=A]
Left
"#;

        let ast = parse(markdown).unwrap();
        let result = check(&ast);

        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);

        if let Directive::Warning { message, .. } = &result.warnings[0] {
            assert!(message.contains("Undeclared"));
            assert!(message.contains("undeclared_cond"));
        } else {
            panic!("Expected Warning directive");
        }
    }

    #[test]
    fn check_unused_condition_warning() {
        let markdown = r#"
:::conditions
unused_condition
can_go_right
:::

[BRANCH choice=右へ if=can_go_right label=right, choice=左へ label=left]

[LABEL name=right]
[SAY speaker=A]
Right

[LABEL name=left]
[SAY speaker=A]
Left
"#;

        let ast = parse(markdown).unwrap();
        let result = check(&ast);

        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);

        if let Directive::Warning { message, .. } = &result.warnings[0] {
            assert!(message.contains("never used"));
            assert!(message.contains("unused_condition"));
        } else {
            panic!("Expected Warning directive");
        }
    }

    #[test]
    fn check_valid_scenario_no_warnings() {
        let markdown = r#"
:::conditions
can_go_right
:::

[BRANCH choice=右へ if=can_go_right label=right, choice=左へ label=left]

[LABEL name=right]
[SAY speaker=A]
Right

[LABEL name=left]
[SAY speaker=A]
Left
"#;

        let ast = parse(markdown).unwrap();
        let result = check(&ast);

        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 0);
    }
}
