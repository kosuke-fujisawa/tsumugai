//! Integration tests for the new simplified architecture
//!
//! These tests verify that parse → runtime → storage workflow functions correctly

use tsumugai::{parser, runtime, storage, types::{event::Event, state::State}};

#[test]
fn integration_simple_scenario() {
    let markdown = r#"
[SAY speaker=Alice]
Hello, world!

[SET name=score value=100]

[PLAY_BGM name=intro.mp3]

[SAY speaker=Bob]
Welcome to the game!
"#;

    // Parse the scenario
    let ast = parser::parse(markdown).expect("Failed to parse scenario");

    // Verify AST structure
    assert_eq!(ast.nodes.len(), 4);

    // Execute step by step
    let mut state = State::new();

    // Step 1: Should execute SAY and stop
    let (state1, output1) = runtime::step(state, &ast, None);
    assert_eq!(output1.lines.len(), 1);
    assert_eq!(output1.lines[0].speaker, Some("Alice".to_string()));
    assert_eq!(output1.lines[0].text, "Hello, world!");
    assert_eq!(state1.pc, 1);

    // Step 2: Should execute SET, PLAY_BGM and stop at PLAY_BGM
    let (state2, output2) = runtime::step(state1, &ast, None);
    assert_eq!(state2.get_var("score"), Some("100".to_string()));
    assert_eq!(output2.effects.len(), 1);
    assert_eq!(output2.effects[0].tag, "play_bgm");
    assert_eq!(state2.pc, 3);

    // Step 3: Should execute final SAY
    let (state3, output3) = runtime::step(state2, &ast, None);
    assert_eq!(output3.lines.len(), 1);
    assert_eq!(output3.lines[0].speaker, Some("Bob".to_string()));
    assert_eq!(output3.lines[0].text, "Welcome to the game!");
    assert_eq!(state3.pc, 4);

    // Test save/load
    let saved_bytes = storage::save(&state3).expect("Failed to save state");
    let loaded_state = storage::load(&saved_bytes).expect("Failed to load state");
    assert_eq!(state3, loaded_state);
}

#[test]
fn integration_branch_scenario() {
    let markdown = r#"
[SAY speaker=Narrator]
Choose your path:

[BRANCH choice=left choice=right]

[LABEL name=left]
[SAY speaker=Guide]
You chose the left path.

[LABEL name=right]
[SAY speaker=Guide]
You chose the right path.
"#;

    // Parse the scenario
    let ast = parser::parse(markdown).expect("Failed to parse scenario");
    let mut state = State::new();

    // Step 1: Execute SAY
    let (state1, output1) = runtime::step(state, &ast, None);
    assert_eq!(output1.lines.len(), 1);
    assert_eq!(output1.lines[0].text, "Choose your path:");

    // Step 2: Execute BRANCH - should present choices
    let (state2, output2) = runtime::step(state1, &ast, None);
    assert_eq!(output2.choices.len(), 2);
    assert_eq!(output2.choices[0].label, "left");
    assert_eq!(output2.choices[1].label, "right");
    assert!(state2.waiting_for_choice);

    // Simulate user choosing left path
    let choice_event = Event::Choice { id: "choice_0".to_string() };
    let (state3, _output3) = runtime::step(state2, &ast, Some(choice_event));
    assert!(!state3.waiting_for_choice);

    // Continue to execute - should go to left label and execute SAY
    let (state4, output4) = runtime::step(state3, &ast, None);
    assert_eq!(output4.lines.len(), 1);
    assert_eq!(output4.lines[0].text, "You chose the left path.");
}

#[test]
fn integration_conditional_jump() {
    let markdown = r#"
[SET name=player_level value=5]

[JUMP_IF var=player_level cmp=ge value=5 label=experienced]

[SAY speaker=System]
You are a beginner.

[LABEL name=experienced]
[SAY speaker=System]
You are experienced!
"#;

    // Parse and execute
    let ast = parser::parse(markdown).expect("Failed to parse scenario");
    let state = State::new();

    // Should execute SET, JUMP_IF (which jumps), LABEL, and stop at SAY
    let (_final_state, output) = runtime::step(state, &ast, None);
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "You are experienced!");
}

#[test]
fn integration_save_load_preserves_execution_state() {
    let markdown = r#"
[SET name=progress value=1]
[SAY speaker=Narrator]
First checkpoint.

[SET name=progress value=2]
[SAY speaker=Narrator]
Second checkpoint.
"#;

    let ast = parser::parse(markdown).expect("Failed to parse scenario");
    let mut state = State::new();

    // Execute first part
    let (state1, _) = runtime::step(state, &ast, None);

    // Save state after first checkpoint
    let saved_bytes = storage::save(&state1).expect("Failed to save");
    let checkpoint_pc = state1.pc;

    // Continue execution
    let (state2, _) = runtime::step(state1, &ast, None);
    assert_eq!(state2.get_var("progress"), Some("2".to_string()));

    // Load the saved state and verify we can continue from checkpoint
    let loaded_state = storage::load(&saved_bytes).expect("Failed to load");
    assert_eq!(loaded_state.get_var("progress"), Some("1".to_string()));
    assert_eq!(loaded_state.pc, checkpoint_pc);

    // Continue from loaded state
    let (state3, output3) = runtime::step(loaded_state, &ast, None);
    assert_eq!(output3.lines[0].text, "Second checkpoint.");
    assert_eq!(state3.get_var("progress"), Some("2".to_string()));
}