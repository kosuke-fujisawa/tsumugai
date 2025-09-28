//! Tests for the runtime module

use super::*;
use crate::types::{
    ast::{Ast, AstNode},
    event::Event,
    state::State,
};
use std::collections::HashMap;

#[test]
fn runtime_say_waits_for_user() {
    let nodes = vec![AstNode::Say {
        speaker: "Alice".to_string(),
        text: "Hello!".to_string(),
    }];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (new_state, output) = step(state, &ast, None);

    // Should have output with one line
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].speaker, Some("Alice".to_string()));
    assert_eq!(output.lines[0].text, "Hello!");

    // PC should advance
    assert_eq!(new_state.pc, 1);
}

#[test]
fn runtime_set_modifies_variable() {
    let nodes = vec![
        AstNode::Set {
            name: "score".to_string(),
            value: "100".to_string(),
        },
        AstNode::Modify {
            name: "score".to_string(),
            op: crate::types::ast::Operation::Add,
            value: "50".to_string(),
        },
        AstNode::Say {
            speaker: "System".to_string(),
            text: "Score updated".to_string(),
        },
    ];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (new_state, output) = step(state, &ast, None);

    // Should have executed both SET and MODIFY, then stopped at SAY
    assert_eq!(new_state.get_var("score"), Some("150".to_string()));
    assert_eq!(new_state.pc, 3); // Advanced past both SET and MODIFY
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Score updated");
}

#[test]
fn runtime_jump_changes_pc() {
    let mut labels = HashMap::new();
    labels.insert("target".to_string(), 2);

    let nodes = vec![
        AstNode::Jump {
            label: "target".to_string(),
        },
        AstNode::Say {
            speaker: "A".to_string(),
            text: "Should skip".to_string(),
        },
        AstNode::Label {
            name: "target".to_string(),
        },
        AstNode::Say {
            speaker: "A".to_string(),
            text: "Target reached".to_string(),
        },
    ];
    let ast = Ast::new(nodes, labels);
    let state = State::new();

    let (new_state, output) = step(state, &ast, None);

    // Should jump to label, continue past it, and stop at SAY
    assert_eq!(new_state.pc, 4); // Advanced past label and stopped at SAY
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Target reached");
}

#[test]
fn runtime_branch_waits_for_choice() {
    let nodes = vec![AstNode::Branch {
        choices: vec![
            crate::types::ast::Choice {
                id: "choice_0".to_string(),
                label: "Option A".to_string(),
                target: "option_a".to_string(),
            },
            crate::types::ast::Choice {
                id: "choice_1".to_string(),
                label: "Option B".to_string(),
                target: "option_b".to_string(),
            },
        ],
    }];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (new_state, output) = step(state, &ast, None);

    // Should have choices in output
    assert_eq!(output.choices.len(), 2);
    assert_eq!(output.choices[0].label, "Option A");
    assert_eq!(output.choices[1].label, "Option B");

    // Should be waiting for choice
    assert!(new_state.waiting_for_choice);
    assert_eq!(new_state.pc, 1);
}

#[test]
fn runtime_conditional_jump_with_condition() {
    let mut labels = HashMap::new();
    labels.insert("success".to_string(), 2);

    let nodes = vec![
        AstNode::Set {
            name: "score".to_string(),
            value: "15".to_string(),
        },
        AstNode::JumpIf {
            var: "score".to_string(),
            cmp: crate::types::ast::Comparison::Equal,
            value: "15".to_string(),
            label: "success".to_string(),
        },
        AstNode::Label {
            name: "success".to_string(),
        },
        AstNode::Say {
            speaker: "System".to_string(),
            text: "Success!".to_string(),
        },
    ];
    let ast = Ast::new(nodes, labels);
    let state = State::new();

    // Should execute SET, JUMP_IF (which jumps), LABEL, and stop at SAY
    let (_state, output) = step(state, &ast, None);
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Success!");
}

#[test]
fn runtime_effects_for_media() {
    let nodes = vec![
        AstNode::PlayBgm {
            name: "intro.mp3".to_string(),
        },
        AstNode::ShowImage {
            layer: "background".to_string(),
            name: "forest.jpg".to_string(),
        },
        AstNode::Wait { seconds: 1.5 },
    ];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    // Play BGM (should stop here)
    let (state1, output1) = step(state, &ast, None);
    assert_eq!(output1.effects.len(), 1);
    assert_eq!(output1.effects[0].tag, "play_bgm");
    assert_eq!(state1.pc, 1);

    // Show image (should stop here)
    let (state2, output2) = step(state1, &ast, None);
    assert_eq!(output2.effects.len(), 1);
    assert_eq!(output2.effects[0].tag, "show_image");
    assert_eq!(state2.pc, 2);

    // Wait (should stop here)
    let (state3, output3) = step(state2, &ast, None);
    assert_eq!(output3.effects.len(), 1);
    assert_eq!(output3.effects[0].tag, "wait");
    assert_eq!(state3.pc, 3);
}
