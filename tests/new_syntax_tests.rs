//! Unit tests for new syntax features (issue #14)
//! - GOTO command
//! - Conditional expressions (:::when)
//! - Scene with ending metadata

use tsumugai::parser::parse;
use tsumugai::types::ast::{AstNode, EndingKind, Expr};

#[cfg(test)]
mod new_syntax_tests {
    use super::*;

    /// Test: GOTO command parsing
    #[test]
    fn test_goto_command_parsing() {
        let input = r#"
[LABEL name=start]
[GOTO target=end]
[LABEL name=end]
"#;

        let result = parse(input).expect("Should parse successfully");
        assert_eq!(result.nodes.len(), 3);

        // Check GOTO node
        match &result.nodes[1] {
            AstNode::Goto { target } => {
                assert_eq!(target, "end");
            }
            _ => panic!("Expected Goto command at index 1"),
        }

        // Verify label exists in labels map
        assert!(result.labels.contains_key("end"));
    }

    /// Test: GOTO to undefined label should fail validation
    #[test]
    fn test_goto_undefined_label() {
        let input = r#"
[GOTO target=undefined_label]
"#;

        let result = parse(input);
        assert!(
            result.is_err(),
            "GOTO to undefined label should fail validation"
        );

        let error = result.unwrap_err();
        let error_msg = format!("{error}");
        assert!(error_msg.contains("undefined_label"));
    }

    /// Test: Scene header parsing
    #[test]
    fn test_scene_header_parsing() {
        let input = r#"
# scene: opening

[SAY speaker=Narrator]
Welcome!
"#;

        let result = parse(input).expect("Should parse successfully");
        assert!(result.nodes.len() >= 1);

        // Check Scene node
        match &result.nodes[0] {
            AstNode::Scene { meta } => {
                assert_eq!(meta.name, "opening");
                assert_eq!(meta.ending, None);
            }
            _ => panic!("Expected Scene command at index 0"),
        }

        // Verify scene name is registered as label
        assert!(result.labels.contains_key("opening"));
    }

    /// Test: Scene with ending metadata
    #[test]
    fn test_scene_with_ending() {
        let input = r#"
# scene: good_end
@ending GOOD

[SAY speaker=Narrator]
Congratulations!
"#;

        let result = parse(input).expect("Should parse successfully");

        // Check Scene node with ending
        match &result.nodes[0] {
            AstNode::Scene { meta } => {
                assert_eq!(meta.name, "good_end");
                assert_eq!(meta.ending, Some(EndingKind::Good));
            }
            _ => panic!("Expected Scene command"),
        }
    }

    /// Test: Scene with different ending types
    #[test]
    fn test_scene_ending_types() {
        let test_cases = vec![
            ("@ending BAD", Some(EndingKind::Bad)),
            ("@ending GOOD", Some(EndingKind::Good)),
            ("@ending TRUE", Some(EndingKind::True)),
            ("@ending NORMAL", Some(EndingKind::Normal)),
            (
                "@ending Custom",
                Some(EndingKind::Custom("CUSTOM".to_string())),
            ),
        ];

        for (ending_line, expected_ending) in test_cases {
            let input = format!(
                r#"
# scene: test_scene
{}

[SAY speaker=Test]
Test
"#,
                ending_line
            );

            let result = parse(&input).expect("Should parse successfully");

            match &result.nodes[0] {
                AstNode::Scene { meta } => {
                    assert_eq!(meta.ending, expected_ending, "Failed for: {}", ending_line);
                }
                _ => panic!("Expected Scene command"),
            }
        }
    }

    /// Test: Simple when block parsing
    #[test]
    fn test_when_block_simple() {
        let input = r#"
:::when score > 10
[SAY speaker=Narrator]
High score!
:::
"#;

        let result = parse(input).expect("Should parse successfully");
        assert_eq!(result.nodes.len(), 1);

        // Check WhenBlock node
        match &result.nodes[0] {
            AstNode::WhenBlock { condition, body } => {
                // Check condition is a comparison
                assert!(matches!(condition, Expr::GreaterThan(_, _)));
                // Check body has SAY command
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], AstNode::Say { .. }));
            }
            _ => panic!("Expected WhenBlock command"),
        }
    }

    /// Test: When block with complex expression
    #[test]
    fn test_when_block_complex_expression() {
        let input = r#"
:::when score > 5 && helped == "true"
[SAY speaker=Narrator]
You are kind!
:::
"#;

        let result = parse(input).expect("Should parse successfully");

        // Check WhenBlock with AND expression
        match &result.nodes[0] {
            AstNode::WhenBlock { condition, body } => {
                // Check condition is AND
                assert!(matches!(condition, Expr::And(_, _)));
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected WhenBlock command"),
        }
    }

    /// Test: When block with OR expression
    #[test]
    fn test_when_block_or_expression() {
        let input = r#"
:::when score < 5 || failed == "true"
[SAY speaker=Narrator]
Try again!
:::
"#;

        let result = parse(input).expect("Should parse successfully");

        match &result.nodes[0] {
            AstNode::WhenBlock { condition, .. } => {
                assert!(matches!(condition, Expr::Or(_, _)));
            }
            _ => panic!("Expected WhenBlock command"),
        }
    }

    /// Test: When block with NOT expression
    #[test]
    fn test_when_block_not_expression() {
        let input = r#"
:::when !completed
[SAY speaker=Narrator]
Not done yet!
:::
"#;

        let result = parse(input).expect("Should parse successfully");

        match &result.nodes[0] {
            AstNode::WhenBlock { condition, .. } => {
                assert!(matches!(condition, Expr::Not(_)));
            }
            _ => panic!("Expected WhenBlock command"),
        }
    }

    /// Test: Complete scenario with GOTO, Scene, and When
    #[test]
    fn test_complete_new_syntax_scenario() {
        let input = r#"
# scene: start

[SET name=score value=10]

:::when score >= 10
[GOTO target=good_end]
:::

:::when score < 10
[GOTO target=bad_end]
:::

# scene: good_end
@ending GOOD

[SAY speaker=Narrator]
Good ending!

# scene: bad_end
@ending BAD

[SAY speaker=Narrator]
Bad ending!
"#;

        let result = parse(input).expect("Should parse successfully");

        // Verify scenes are registered as labels
        assert!(result.labels.contains_key("start"));
        assert!(result.labels.contains_key("good_end"));
        assert!(result.labels.contains_key("bad_end"));

        // Count Scene nodes
        let scene_count = result
            .nodes
            .iter()
            .filter(|n| matches!(n, AstNode::Scene { .. }))
            .count();
        assert_eq!(scene_count, 3);

        // Count WhenBlock nodes
        let when_count = result
            .nodes
            .iter()
            .filter(|n| matches!(n, AstNode::WhenBlock { .. }))
            .count();
        assert_eq!(when_count, 2);
    }

    /// Test: Expression parsing - equality
    #[test]
    fn test_expression_equality() {
        let input = r#"
:::when var == "value"
[SAY speaker=Test]
Equal!
:::
"#;

        let result = parse(input).expect("Should parse successfully");

        match &result.nodes[0] {
            AstNode::WhenBlock { condition, .. } => match condition {
                Expr::Equal(left, right) => {
                    assert!(matches!(**left, Expr::Var(_)));
                    assert!(matches!(**right, Expr::String(_)));
                }
                _ => panic!("Expected Equal expression"),
            },
            _ => panic!("Expected WhenBlock"),
        }
    }

    /// Test: Expression parsing - numbers
    #[test]
    fn test_expression_numbers() {
        let input = r#"
:::when score >= 15
[SAY speaker=Test]
High score!
:::
"#;

        let result = parse(input).expect("Should parse successfully");

        match &result.nodes[0] {
            AstNode::WhenBlock { condition, .. } => match condition {
                Expr::GreaterThanOrEqual(left, right) => {
                    assert!(matches!(**left, Expr::Var(_)));
                    match **right {
                        Expr::Number(n) => assert_eq!(n, 15),
                        _ => panic!("Expected Number expression"),
                    }
                }
                _ => panic!("Expected GreaterThanOrEqual expression"),
            },
            _ => panic!("Expected WhenBlock"),
        }
    }
}
