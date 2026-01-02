//! Quality checking implementation

use crate::lint::config::LintConfig;
use crate::lint::{LintIssue, LintLevel, LintResult};
use crate::types::ast::{Ast, AstNode};

/// Check quality issues (long waits, duplicate BGM, etc.)
pub fn check(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    check_consecutive_waits(ast, result, config);
    check_duplicate_bgm(ast, result, config);
    check_text_length(ast, result, config);
}

fn check_consecutive_waits(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    let mut consecutive_wait_time = 0.0f32;

    for node in &ast.nodes {
        match node {
            AstNode::Wait { seconds } => {
                consecutive_wait_time += seconds;
            }
            _ => {
                // Reset on non-wait command
                if consecutive_wait_time > config.quality.max_consecutive_wait {
                    result.add_issue(LintIssue {
                        level: LintLevel::Warning,
                        message: format!(
                            "Consecutive WAIT commands total {:.1}s (threshold: {:.1}s)",
                            consecutive_wait_time, config.quality.max_consecutive_wait
                        ),
                        line: 0,
                        column: 0,
                        category: "quality".to_string(),
                    });
                }
                consecutive_wait_time = 0.0;
            }
        }
    }

    // Check at end
    if consecutive_wait_time > config.quality.max_consecutive_wait {
        result.add_issue(LintIssue {
            level: LintLevel::Warning,
            message: format!(
                "Consecutive WAIT commands total {:.1}s (threshold: {:.1}s)",
                consecutive_wait_time, config.quality.max_consecutive_wait
            ),
            line: 0,
            column: 0,
            category: "quality".to_string(),
        });
    }
}

fn check_duplicate_bgm(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    if !config.quality.warn_duplicate_bgm {
        return;
    }

    let mut last_bgm: Option<String> = None;

    for node in &ast.nodes {
        if let AstNode::PlayBgm { name } = node {
            if let Some(ref last) = last_bgm
                && last == name {
                    result.add_issue(LintIssue {
                        level: LintLevel::Info,
                        message: format!("Duplicate BGM '{}' played consecutively", name),
                        line: 0,
                        column: 0,
                        category: "quality".to_string(),
                    });
                }
            last_bgm = Some(name.clone());
        }
    }
}

fn check_text_length(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    for node in &ast.nodes {
        if let AstNode::Say { text, .. } = node
            && text.len() > config.quality.max_text_length {
                result.add_issue(LintIssue {
                    level: LintLevel::Info,
                    message: format!(
                        "Text length {} exceeds recommended maximum of {}",
                        text.len(),
                        config.quality.max_text_length
                    ),
                    line: 0,
                    column: 0,
                    category: "quality".to_string(),
                });
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn quality_check_consecutive_waits() {
        let markdown = r#"
[WAIT 3s]
[WAIT 3s]
[SAY speaker=A]
Done
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let config = LintConfig::default();

        check(&ast, &mut result, &config);

        assert_eq!(result.warning_count, 1);
        assert!(result.issues[0].message.contains("Consecutive WAIT"));
    }

    #[test]
    fn quality_check_duplicate_bgm() {
        let markdown = r#"
[PLAY_BGM name=intro]
[PLAY_BGM name=intro]
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let config = LintConfig::default();

        check(&ast, &mut result, &config);

        assert_eq!(result.info_count, 1);
        assert!(result.issues[0].message.contains("Duplicate BGM"));
    }
}
