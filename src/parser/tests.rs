//! Tests for the parser module

use super::*;
use crate::types::ast::{AstNode, Choice, Comparison, Operation};

#[test]
fn parse_single_say_returns_ast_node() {
    let markdown = r#"
[SAY speaker=Alice]
Hello, world!
"#;

    let ast = parse(markdown).unwrap();
    assert_eq!(ast.nodes.len(), 1);

    match &ast.nodes[0] {
        AstNode::Say { speaker, text } => {
            assert_eq!(speaker, "Alice");
            assert_eq!(text, "Hello, world!");
        }
        _ => panic!("Expected Say node"),
    }
}

#[test]
fn parse_bgm_command() {
    let markdown = "[PLAY_BGM name=intro.mp3]";

    let ast = parse(markdown).unwrap();
    assert_eq!(ast.nodes.len(), 1);

    match &ast.nodes[0] {
        AstNode::PlayBgm { name } => {
            assert_eq!(name, "intro.mp3");
        }
        _ => panic!("Expected PlayBgm node"),
    }
}

#[test]
fn parse_wait_with_duration() {
    let markdown = "[WAIT 1.5s]";

    let ast = parse(markdown).unwrap();
    assert_eq!(ast.nodes.len(), 1);

    match &ast.nodes[0] {
        AstNode::Wait { seconds } => {
            assert_eq!(*seconds, 1.5);
        }
        _ => panic!("Expected Wait node"),
    }
}

#[test]
fn parse_label_and_jump() {
    let markdown = r#"
[JUMP label=target]
[LABEL name=target]
[SAY speaker=Alice]
Target reached!
"#;

    let ast = parse(markdown).unwrap();
    assert_eq!(ast.nodes.len(), 3);

    // Check jump
    match &ast.nodes[0] {
        AstNode::Jump { label } => {
            assert_eq!(label, "target");
        }
        _ => panic!("Expected Jump node"),
    }

    // Check label
    match &ast.nodes[1] {
        AstNode::Label { name } => {
            assert_eq!(name, "target");
        }
        _ => panic!("Expected Label node"),
    }

    // Check that label index is recorded
    assert_eq!(ast.get_label_index("target"), Some(1));
}

#[test]
fn parse_branch_with_choices() {
    let markdown = r#"
[BRANCH choice=left choice=right]

[LABEL name=left]
[SAY speaker=Guide]
Left path.

[LABEL name=right]
[SAY speaker=Guide]
Right path.
"#;

    let ast = parse(markdown).unwrap();
    assert!(ast.nodes.len() >= 1);

    match &ast.nodes[0] {
        AstNode::Branch { choices } => {
            assert!(choices.len() >= 1);
            // The exact parsing may vary based on implementation
        }
        _ => panic!("Expected Branch node"),
    }
}

#[test]
fn parse_set_and_modify() {
    let markdown = r#"
[SET name=score value=10]
[MODIFY name=score op=add value=5]
"#;

    let ast = parse(markdown).unwrap();
    assert_eq!(ast.nodes.len(), 2);

    match &ast.nodes[0] {
        AstNode::Set { name, value } => {
            assert_eq!(name, "score");
            assert_eq!(value, "10");
        }
        _ => panic!("Expected Set node"),
    }

    match &ast.nodes[1] {
        AstNode::Modify { name, op, value } => {
            assert_eq!(name, "score");
            assert_eq!(*op, Operation::Add);
            assert_eq!(value, "5");
        }
        _ => panic!("Expected Modify node"),
    }
}

#[test]
fn parse_conditional_jump() {
    let markdown = r#"
[JUMP_IF var=score cmp=eq value=15 label=success]

[LABEL name=success]
[SAY speaker=System]
Success!
"#;

    let ast = parse(markdown).unwrap();
    assert!(ast.nodes.len() >= 1);

    match &ast.nodes[0] {
        AstNode::JumpIf {
            var,
            cmp,
            value,
            label,
        } => {
            assert_eq!(var, "score");
            assert_eq!(*cmp, Comparison::Equal);
            assert_eq!(value, "15");
            assert_eq!(label, "success");
        }
        _ => panic!("Expected JumpIf node"),
    }
}

#[test]
fn parse_invalid_command_returns_error() {
    let markdown = "[INVALID_COMMAND]";

    let result = parse(markdown);
    assert!(result.is_err());
}

#[test]
fn parse_undefined_label_returns_error() {
    let markdown = "[JUMP label=undefined]";

    let result = parse(markdown);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Undefined label"));
}

#[test]
fn parse_skip_comments_and_headers() {
    let markdown = r#"
# This is a header
<!-- This is a comment -->

[SAY speaker=Alice]
Hello!
"#;

    let ast = parse(markdown).unwrap();
    assert_eq!(ast.nodes.len(), 1);

    match &ast.nodes[0] {
        AstNode::Say { speaker, text } => {
            assert_eq!(speaker, "Alice");
            assert_eq!(text, "Hello!");
        }
        _ => panic!("Expected Say node"),
    }
}
