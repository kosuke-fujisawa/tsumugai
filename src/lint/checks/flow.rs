//! Flow analysis implementation

use crate::lint::config::LintConfig;
use crate::lint::{LintIssue, LintLevel, LintResult};
use crate::types::ast::{Ast, AstNode};
use std::collections::{HashSet, VecDeque};

/// Check flow issues (unreachable code, infinite loops, etc.)
pub fn check(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    if config.flow.check_termination {
        check_unreachable_code(ast, result, config);
    }

    if config.flow.check_infinite_loops {
        check_potential_infinite_loops(ast, result, config);
    }
}

/// Check for unreachable code
fn check_unreachable_code(ast: &Ast, result: &mut LintResult, _config: &LintConfig) {
    // Build reachability graph using BFS
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::new();

    // Start from PC=0
    queue.push_back(0);
    reachable.insert(0);

    while let Some(pc) = queue.pop_front() {
        if pc >= ast.len() {
            continue;
        }

        let node = match ast.get_node(pc) {
            Some(node) => node,
            None => continue,
        };

        match node {
            AstNode::Jump { label } => {
                // Only jumps to target
                if let Some(target_pc) = ast.get_label_index(label)
                    && !reachable.contains(&target_pc)
                {
                    reachable.insert(target_pc);
                    queue.push_back(target_pc);
                }
            }
            AstNode::JumpIf { label, .. } => {
                // Can jump to target OR continue to next
                if let Some(target_pc) = ast.get_label_index(label)
                    && !reachable.contains(&target_pc)
                {
                    reachable.insert(target_pc);
                    queue.push_back(target_pc);
                }
                let next_pc = pc + 1;
                if !reachable.contains(&next_pc) {
                    reachable.insert(next_pc);
                    queue.push_back(next_pc);
                }
            }
            AstNode::Branch { choices } => {
                // Can jump to any choice target
                for choice in choices {
                    if let Some(target_pc) = ast.get_label_index(&choice.target)
                        && !reachable.contains(&target_pc)
                    {
                        reachable.insert(target_pc);
                        queue.push_back(target_pc);
                    }
                }
                // Also continues to next (to emit the branch directive)
                let next_pc = pc + 1;
                if !reachable.contains(&next_pc) {
                    reachable.insert(next_pc);
                    queue.push_back(next_pc);
                }
            }
            _ => {
                // Normal instruction, continues to next
                let next_pc = pc + 1;
                if next_pc < ast.len() && !reachable.contains(&next_pc) {
                    reachable.insert(next_pc);
                    queue.push_back(next_pc);
                }
            }
        }
    }

    // Report unreachable code (excluding labels)
    for (index, node) in ast.nodes.iter().enumerate() {
        if !reachable.contains(&index) {
            // Skip labels as they are markers
            if !matches!(node, AstNode::Label { .. }) {
                result.add_issue(LintIssue {
                    level: LintLevel::Warning,
                    message: format!("Unreachable code at index {}: {:?}", index, node),
                    line: 0,
                    column: 0,
                    category: "flow".to_string(),
                });
            }
        }
    }
}

/// Check for potential infinite loops
fn check_potential_infinite_loops(ast: &Ast, result: &mut LintResult, config: &LintConfig) {
    // Simple heuristic: detect jumps backward without conditional exits
    for (index, node) in ast.nodes.iter().enumerate() {
        if let AstNode::Jump { label } = node
            && let Some(target_pc) = ast.get_label_index(label)
            && target_pc <= index
        {
            // This is a backward jump
            // Check if there's any conditional exit between target and current
            let has_conditional_exit =
                check_has_conditional_exit(ast, target_pc, index, config.flow.max_analysis_depth);

            if !has_conditional_exit {
                result.add_issue(LintIssue {
                            level: LintLevel::Warning,
                            message: format!(
                                "Potential infinite loop: unconditional backward jump from {} to {} (label={})",
                                index, target_pc, label
                            ),
                            line: 0,
                            column: 0,
                            category: "flow".to_string(),
                        });
            }
        }
    }
}

/// Check if there's a conditional exit in the code range
fn check_has_conditional_exit(ast: &Ast, start: usize, end: usize, _max_depth: usize) -> bool {
    for i in start..=end.min(ast.len() - 1) {
        if let Some(AstNode::JumpIf { .. } | AstNode::Branch { .. }) = ast.get_node(i) {
            return true; // Has conditional exit
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn flow_check_no_issues_in_simple_scenario() {
        let markdown = r#"
[SAY speaker=A]
Hello!
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let mut config = LintConfig::default();
        config.flow.check_termination = true;
        config.flow.check_infinite_loops = true;

        check(&ast, &mut result, &config);

        // Simple scenario should have no issues
        assert!(result.is_clean());
    }

    #[test]
    fn flow_check_unreachable_code_after_jump() {
        let markdown = r#"
[JUMP label=end]

[SAY speaker=A]
This is unreachable

[LABEL name=end]
[SAY speaker=A]
End
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let mut config = LintConfig::default();
        config.flow.check_termination = true;

        check(&ast, &mut result, &config);

        // Should detect unreachable SAY command
        assert_eq!(result.warning_count, 1);
        assert!(result.issues[0].message.contains("Unreachable"));
    }

    #[test]
    fn flow_check_infinite_loop_detection() {
        let markdown = r#"
[LABEL name=loop_start]
[SAY speaker=A]
Loop

[JUMP label=loop_start]
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let mut config = LintConfig::default();
        config.flow.check_infinite_loops = true;

        check(&ast, &mut result, &config);

        // Should detect potential infinite loop
        assert_eq!(result.warning_count, 1);
        assert!(result.issues[0].message.contains("infinite loop"));
    }

    #[test]
    fn flow_check_conditional_loop_is_ok() {
        let markdown = r#"
[SET name=counter value=0]

[LABEL name=loop_start]
[SAY speaker=A]
Loop

[MODIFY name=counter op=add value=1]
[JUMP_IF var=counter cmp=lt value=10 label=loop_start]

[SAY speaker=A]
Done
"#;

        let ast = parse(markdown).unwrap();
        let mut result = LintResult::new();
        let mut config = LintConfig::default();
        config.flow.check_infinite_loops = true;

        check(&ast, &mut result, &config);

        // Should not detect infinite loop (has conditional exit)
        assert!(result.is_clean());
    }
}
