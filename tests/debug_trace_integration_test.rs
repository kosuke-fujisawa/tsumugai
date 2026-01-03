//! Integration tests for debug trace functionality
//!
//! Tests the full debug trace flow including:
//! - Scene navigation
//! - Dialogue tracking
//! - Choice presentation
//! - Variable changes
//! - Jump events

use tsumugai::{parser, runtime};
use tsumugai::types::{
    debug::DebugTraceEvent,
    event::Event,
    state::State,
};

#[test]
fn debug_trace_full_scenario_flow() {
    let markdown = r#"
# scene: opening

[SAY speaker=Narrator]
Welcome to the story.

[SET name=chapter value=1]

[SAY speaker=Guide]
Let's begin your journey.
"#;

    let ast = parser::parse_unchecked(markdown).expect("Failed to parse scenario");
    let mut state = State::new();

    // Step 1: Enter opening scene + first dialogue (Narrator)
    let (new_state, _output, trace1) = runtime::step_with_trace(state, &ast, None);
    state = new_state;

    // Should have EnterScene and Dialogue events
    assert!(trace1.iter().any(|e| matches!(e, DebugTraceEvent::EnterScene { name } if name == "opening")));
    assert!(trace1.iter().any(|e| matches!(
        e,
        DebugTraceEvent::Dialogue { speaker: Some(speaker), text, .. }
        if speaker == "Narrator" && text == "Welcome to the story."
    )));

    // Step 2: SET command + second dialogue (Guide)
    let (_new_state, _output, trace2) = runtime::step_with_trace(state, &ast, Some(Event::Continue));

    // Should have EffectSetVar and Dialogue events
    assert!(trace2.iter().any(|e| matches!(
        e,
        DebugTraceEvent::EffectSetVar { name, after, .. }
        if name == "chapter" && after == &serde_json::json!(1)
    )));
    assert!(trace2.iter().any(|e| matches!(
        e,
        DebugTraceEvent::Dialogue { speaker: Some(speaker), text, .. }
        if speaker == "Guide" && text == "Let's begin your journey."
    )));
}

#[test]
fn debug_trace_scene_index_building() {
    let markdown = r#"
# scene: intro

[SAY speaker=A]
Intro

# scene: middle

[SAY speaker=B]
Middle

# scene: ending

[SAY speaker=C]
Ending
"#;

    let ast = parser::parse_unchecked(markdown).expect("Failed to parse scenario");
    let scene_index = ast.build_scene_index();

    // Should have three scenes
    assert_eq!(scene_index.len(), 3);
    assert!(scene_index.contains_key("intro"));
    assert!(scene_index.contains_key("middle"));
    assert!(scene_index.contains_key("ending"));

    // Scene positions should be in order
    assert!(scene_index.get("intro").unwrap() < scene_index.get("middle").unwrap());
    assert!(scene_index.get("middle").unwrap() < scene_index.get("ending").unwrap());
}

#[test]
fn debug_trace_jump_with_goto() {
    let markdown = r#"
[SAY speaker=A]
Start

[GOTO target=ending]

[SAY speaker=B]
Should be skipped

[LABEL name=ending]
[SAY speaker=C]
End
"#;

    let ast = parser::parse_unchecked(markdown).expect("Failed to parse scenario");
    let mut state = State::new();

    // Step 1: First dialogue
    let (new_state, _output, _trace1) = runtime::step_with_trace(state, &ast, None);
    state = new_state;

    // Step 2: GOTO command + Label + End dialogue
    let (_new_state, _output, trace2) = runtime::step_with_trace(state, &ast, Some(Event::Continue));

    // Should have Jump event with Goto reason and ending dialogue
    assert!(trace2.iter().any(|e| matches!(
        e,
        DebugTraceEvent::Jump { to, reason }
        if to == "ending" && matches!(reason, tsumugai::types::debug::JumpReason::Goto)
    )));

    // Should have the ending dialogue in the same step
    assert!(trace2.iter().any(|e| matches!(
        e,
        DebugTraceEvent::Dialogue { text, .. }
        if text == "End"
    )));
}

#[test]
fn debug_trace_variable_change() {
    let markdown = r#"
[SET name=score value=100]

[MODIFY name=score op=add value=50]

[SAY speaker=A]
Score updated
"#;

    let ast = parser::parse_unchecked(markdown).expect("Failed to parse scenario");
    let state = State::new();

    // Execute step: SET + MODIFY + SAY
    let (_new_state, _output, trace) = runtime::step_with_trace(state, &ast, None);

    // Should have two EffectSetVar events for SET and MODIFY
    let set_events: Vec<_> = trace.iter().filter(|e| matches!(e, DebugTraceEvent::EffectSetVar { .. })).collect();
    assert_eq!(set_events.len(), 2);

    // Should have Dialogue event
    assert!(trace.iter().any(|e| matches!(
        e,
        DebugTraceEvent::Dialogue { text, .. }
        if text == "Score updated"
    )));
}

#[test]
fn debug_trace_no_internal_labels() {
    let markdown = r#"
# scene: test

[POSITION layer=bg name=forest.jpg]

[SAY speaker=A]
Hello
"#;

    let ast = parser::parse_unchecked(markdown).expect("Failed to parse scenario");
    let state = State::new();

    // Execute all steps
    let mut current_state = state;
    let mut all_traces = Vec::new();

    for _ in 0..10 {
        if current_state.pc >= ast.len() {
            break;
        }

        let (new_state, _output, traces) =
            runtime::step_with_trace(current_state, &ast, Some(Event::Continue));
        current_state = new_state;
        all_traces.extend(traces);
    }

    // Verify no trace event contains "__skip_" in any field
    for trace in &all_traces {
        match trace {
            DebugTraceEvent::Jump { to, .. } => {
                assert!(!to.contains("__skip_"), "Jump target should not contain __skip_");
            }
            DebugTraceEvent::EnterScene { name } => {
                assert!(!name.contains("__skip_"), "Scene name should not contain __skip_");
            }
            _ => {}
        }
    }

    // Should have EnterScene and Dialogue events, but no internal labels
    assert!(all_traces.iter().any(|e| matches!(e, DebugTraceEvent::EnterScene { .. })));
    assert!(all_traces.iter().any(|e| matches!(e, DebugTraceEvent::Dialogue { .. })));
}
