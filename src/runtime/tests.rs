//! Tests for the runtime module

use super::*;
use crate::types::{
    ast::{Ast, AstNode},
    debug::DebugTraceEvent,
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
                condition: None,
            },
            crate::types::ast::Choice {
                id: "choice_1".to_string(),
                label: "Option B".to_string(),
                target: "option_b".to_string(),
                condition: None,
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

#[test]
fn runtime_debug_logging_with_jumps() {
    // This test verifies debug logging works without panicking
    // Actual log output would be to stderr and not captured here
    let mut labels = HashMap::new();
    labels.insert("target".to_string(), 2);

    let nodes = vec![
        AstNode::Set {
            name: "x".to_string(),
            value: "10".to_string(),
        },
        AstNode::Jump {
            label: "target".to_string(),
        },
        AstNode::Label {
            name: "target".to_string(),
        },
        AstNode::Say {
            speaker: "A".to_string(),
            text: "Reached!".to_string(),
        },
    ];
    let ast = Ast::new(nodes, labels);
    let state = State::new();

    let mut debug_config = debug::DebugConfig::default();
    debug_config.enabled = true;
    debug_config.categories.insert(debug::DebugCategory::Flow);
    debug_config
        .categories
        .insert(debug::DebugCategory::Variables);

    // Execute with debug logging enabled
    let (_state, output) = step_with_debug(state, &ast, None, &debug_config);

    // Should reach the SAY command after the jump
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Reached!");
}

#[test]
fn runtime_debug_logging_with_branches() {
    // Test debug logging with branch execution
    let mut labels = HashMap::new();
    labels.insert("option_a".to_string(), 1);

    let nodes = vec![
        AstNode::Branch {
            choices: vec![crate::types::ast::Choice {
                id: "choice_0".to_string(),
                label: "Option A".to_string(),
                target: "option_a".to_string(),
                condition: None,
            }],
        },
        AstNode::Label {
            name: "option_a".to_string(),
        },
        AstNode::Say {
            speaker: "A".to_string(),
            text: "A!".to_string(),
        },
    ];
    let ast = Ast::new(nodes, labels);
    let state = State::new();

    let mut debug_config = debug::DebugConfig::default();
    debug_config.enabled = true;

    // First step: present branch
    let (state1, output1) = step_with_debug(state, &ast, None, &debug_config);
    assert_eq!(output1.choices.len(), 1);
    assert!(state1.waiting_for_choice);

    // Second step: make choice
    let event = Event::Choice {
        id: "choice_0".to_string(),
    };
    let (_state2, output2) = step_with_debug(state1, &ast, Some(event), &debug_config);

    // Should reach the SAY after making choice
    assert_eq!(output2.lines.len(), 1);
    assert_eq!(output2.lines[0].text, "A!");
}

#[test]
fn runtime_debug_logging_with_variables() {
    // Test debug logging with variable operations
    let nodes = vec![
        AstNode::Set {
            name: "counter".to_string(),
            value: "0".to_string(),
        },
        AstNode::Modify {
            name: "counter".to_string(),
            op: crate::types::ast::Operation::Add,
            value: "5".to_string(),
        },
        AstNode::Say {
            speaker: "System".to_string(),
            text: "Done".to_string(),
        },
    ];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let mut debug_config = debug::DebugConfig::default();
    debug_config.enabled = true;
    debug_config
        .categories
        .insert(debug::DebugCategory::Variables);

    // Execute with variable debug logging
    let (_state, output) = step_with_debug(state, &ast, None, &debug_config);

    // Should reach the SAY command
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Done");
}

// Tests for step_with_trace function

#[test]
fn step_with_trace_generates_dialogue_event() {
    let nodes = vec![AstNode::Say {
        speaker: "Alice".to_string(),
        text: "Hello, world!".to_string(),
    }];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (_new_state, _output, trace_events) = step_with_trace(state, &ast, None);

    // Should have one Dialogue trace event
    assert_eq!(trace_events.len(), 1);
    match &trace_events[0] {
        DebugTraceEvent::Dialogue {
            speaker,
            text,
            click_wait,
        } => {
            assert_eq!(speaker.as_ref().unwrap(), "Alice");
            assert_eq!(text, "Hello, world!");
            assert!(*click_wait);
        }
        _ => panic!("Expected Dialogue event"),
    }
}

#[test]
fn step_with_trace_generates_enter_scene_event() {
    use crate::types::ast::{EndingKind, SceneMeta};

    let nodes = vec![AstNode::Scene {
        meta: SceneMeta {
            name: "opening".to_string(),
            ending: Some(EndingKind::Good),
        },
    }];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (_new_state, _output, trace_events) = step_with_trace(state, &ast, None);

    // Should have one EnterScene trace event
    assert_eq!(trace_events.len(), 1);
    match &trace_events[0] {
        DebugTraceEvent::EnterScene { name } => {
            assert_eq!(name, "opening");
        }
        _ => panic!("Expected EnterScene event"),
    }
}

#[test]
fn step_with_trace_generates_present_choices_event() {
    let nodes = vec![AstNode::Branch {
        choices: vec![
            crate::types::ast::Choice {
                id: "choice_0".to_string(),
                label: "Option A".to_string(),
                target: "option_a".to_string(),
                condition: None,
            },
            crate::types::ast::Choice {
                id: "choice_1".to_string(),
                label: "Option B".to_string(),
                target: "option_b".to_string(),
                condition: Some("flag == true".to_string()),
            },
        ],
    }];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (_new_state, _output, trace_events) = step_with_trace(state, &ast, None);

    // Should have one PresentChoices trace event
    assert_eq!(trace_events.len(), 1);
    match &trace_events[0] {
        DebugTraceEvent::PresentChoices { choices } => {
            assert_eq!(choices.len(), 2);
            assert_eq!(choices[0].label, "Option A");
            assert_eq!(choices[0].condition, None);
            assert_eq!(choices[1].label, "Option B");
            assert_eq!(choices[1].condition, Some("flag == true".to_string()));
        }
        _ => panic!("Expected PresentChoices event"),
    }
}

#[test]
fn step_with_trace_generates_effect_set_var_event() {
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
    ];
    let ast = Ast::new(nodes, HashMap::new());
    let state = State::new();

    let (_new_state, _output, trace_events) = step_with_trace(state, &ast, None);

    // Should have two EffectSetVar trace events (SET and MODIFY)
    assert_eq!(trace_events.len(), 2);

    match &trace_events[0] {
        DebugTraceEvent::EffectSetVar {
            name,
            before,
            after,
        } => {
            assert_eq!(name, "score");
            assert_eq!(before, &serde_json::Value::Null);
            assert_eq!(after, &serde_json::json!(100));
        }
        _ => panic!("Expected EffectSetVar event for SET"),
    }

    match &trace_events[1] {
        DebugTraceEvent::EffectSetVar {
            name,
            before,
            after,
        } => {
            assert_eq!(name, "score");
            assert_eq!(before, &serde_json::json!(100));
            assert_eq!(after, &serde_json::json!(150));
        }
        _ => panic!("Expected EffectSetVar event for MODIFY"),
    }
}

#[test]
fn step_with_trace_generates_jump_event_with_goto_reason() {
    let mut labels = HashMap::new();
    labels.insert("target".to_string(), 1);

    let nodes = vec![
        AstNode::Goto {
            target: "target".to_string(),
        },
        AstNode::Label {
            name: "target".to_string(),
        },
    ];
    let ast = Ast::new(nodes, labels);
    let state = State::new();

    let (_new_state, _output, trace_events) = step_with_trace(state, &ast, None);

    // Should have one Jump trace event
    assert_eq!(trace_events.len(), 1);
    match &trace_events[0] {
        DebugTraceEvent::Jump { to, reason } => {
            assert_eq!(to, "target");
            match reason {
                crate::types::debug::JumpReason::Goto => {}
                _ => panic!("Expected Goto reason"),
            }
        }
        _ => panic!("Expected Jump event"),
    }
}

#[test]
fn step_with_trace_generates_jump_event_with_when_reason() {
    let mut labels = HashMap::new();
    labels.insert("success".to_string(), 2);

    let nodes = vec![
        AstNode::Set {
            name: "x".to_string(),
            value: "10".to_string(),
        },
        AstNode::JumpIf {
            var: "x".to_string(),
            cmp: crate::types::ast::Comparison::Equal,
            value: "10".to_string(),
            label: "success".to_string(),
        },
        AstNode::Label {
            name: "success".to_string(),
        },
    ];
    let ast = Ast::new(nodes, labels);
    let state = State::new();

    let (_new_state, _output, trace_events) = step_with_trace(state, &ast, None);

    // Should have two trace events: EffectSetVar and Jump
    assert!(trace_events.len() >= 2);

    let jump_event = trace_events
        .iter()
        .find(|e| matches!(e, DebugTraceEvent::Jump { .. }));
    assert!(jump_event.is_some());

    match jump_event.unwrap() {
        DebugTraceEvent::Jump { to, reason } => {
            assert_eq!(to, "success");
            match reason {
                crate::types::debug::JumpReason::When { expr } => {
                    assert!(expr.contains("x"));
                    assert!(expr.contains("10"));
                }
                _ => panic!("Expected When reason"),
            }
        }
        _ => panic!("Expected Jump event"),
    }
}
