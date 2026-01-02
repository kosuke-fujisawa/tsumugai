//! Integration tests for the new simplified API
//!
//! These tests verify the complete flow: parse -> step -> save/load

use tsumugai::{
    parser, runtime, storage,
    types::{Event, State},
};

#[test]
fn simple_scenario_with_say() {
    let markdown = r#"
[SAY speaker=Alice]
Hello, world!

[SAY speaker=Bob]
Nice to meet you!
"#;

    // Parse
    let ast = parser::parse(markdown).unwrap();
    assert_eq!(ast.len(), 2);

    // Execute first step
    let state = State::new();
    let (state, output) = runtime::step(state, &ast, None);

    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].speaker, Some("Alice".to_string()));
    assert_eq!(output.lines[0].text, "Hello, world!");
    assert_eq!(state.pc, 1);

    // Execute second step
    let (state, output) = runtime::step(state, &ast, None);

    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].speaker, Some("Bob".to_string()));
    assert_eq!(output.lines[0].text, "Nice to meet you!");
    assert_eq!(state.pc, 2);
}

#[test]
fn scenario_with_variables_and_conditions() {
    let markdown = r#"
[SET name=score value=10]
[MODIFY name=score op=add value=5]
[JUMP_IF var=score cmp=eq value=15 label=success]

[LABEL name=failure]
[SAY speaker=System]
Failed!

[LABEL name=success]
[SAY speaker=System]
Success!
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // Execute - should set score, modify it, check condition and jump to success
    let (state, output) = runtime::step(state, &ast, None);

    // Should have executed SET, MODIFY, JUMP_IF, LABEL and stopped at SAY
    assert_eq!(state.get_var("score"), Some("15".to_string()));
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Success!");
}

#[test]
fn scenario_with_branch_and_choice() {
    let markdown = r#"
[SAY speaker=Guide]
Choose your path.

[BRANCH choice=left choice=right]

[LABEL name=left]
[SAY speaker=Guide]
You went left.

[LABEL name=right]
[SAY speaker=Guide]
You went right.
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // First step - SAY
    let (state, output1) = runtime::step(state, &ast, None);
    assert_eq!(output1.lines.len(), 1);
    assert_eq!(output1.lines[0].text, "Choose your path.");

    // Second step - BRANCH
    let (state, output2) = runtime::step(state, &ast, None);
    assert_eq!(output2.choices.len(), 2);
    assert_eq!(output2.choices[0].label, "left");
    assert_eq!(output2.choices[1].label, "right");
    assert!(state.waiting_for_choice);

    // User chooses "left"
    let event = Event::Choice {
        id: "choice_0".to_string(),
    };
    let (state, output3) = runtime::step(state, &ast, Some(event));

    // Should jump to left label and display the message
    assert!(!state.waiting_for_choice);
    assert_eq!(output3.lines.len(), 1);
    assert_eq!(output3.lines[0].text, "You went left.");
}

#[test]
fn save_and_load_preserves_state() {
    let markdown = r#"
[SET name=progress value=5]
[SAY speaker=System]
Checkpoint reached.
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // Execute to checkpoint (SET command continues, stops at SAY)
    let (state, output) = runtime::step(state, &ast, None);
    assert_eq!(state.get_var("progress"), Some("5".to_string()));
    assert_eq!(output.lines.len(), 1);
    assert_eq!(output.lines[0].text, "Checkpoint reached.");
    assert_eq!(state.pc, 2);

    // Save state
    let save_data = storage::save(&state).unwrap();

    // Load state
    let loaded_state = storage::load(&save_data).unwrap();

    // Verify state was preserved
    assert_eq!(loaded_state.pc, state.pc);
    assert_eq!(loaded_state.get_var("progress"), Some("5".to_string()));
    assert_eq!(loaded_state, state);

    // Loaded state should be at the same position (already displayed SAY)
    // Next step would be past the end
    assert_eq!(loaded_state.pc, 2);
}

#[test]
fn complete_scenario_flow() {
    let markdown = r#"
[SAY speaker=Narrator]
Welcome to the adventure!

[SET name=health value=100]
[PLAY_BGM name=intro.mp3]

[SAY speaker=Guide]
What will you do?

[BRANCH choice=explore choice=rest]

[LABEL name=explore]
[MODIFY name=health op=sub value=10]
[SAY speaker=Narrator]
You explored and found treasure, but lost 10 health.
[JUMP label=end]

[LABEL name=rest]
[MODIFY name=health op=add value=20]
[SAY speaker=Narrator]
You rested and gained 20 health.

[LABEL name=end]
[SAY speaker=Guide]
Your adventure continues...
"#;

    let ast = parser::parse(markdown).unwrap();
    let mut state = State::new();

    // Step 1: Welcome message
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    assert_eq!(output.lines[0].text, "Welcome to the adventure!");

    // Step 2: SET health and PLAY_BGM
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    assert_eq!(state.get_var("health"), Some("100".to_string()));
    assert_eq!(output.effects.len(), 1);
    assert_eq!(output.effects[0].tag, "play_bgm");

    // Step 3: Guide question
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    assert_eq!(output.lines[0].text, "What will you do?");

    // Step 4: Branch - show choices
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    assert_eq!(output.choices.len(), 2);

    // Choose to explore
    let event = Event::Choice {
        id: "choice_0".to_string(),
    };
    let (new_state, output) = runtime::step(state, &ast, Some(event));
    state = new_state;

    // Should show explore result
    assert_eq!(
        output.lines[0].text,
        "You explored and found treasure, but lost 10 health."
    );
    assert_eq!(state.get_var("health"), Some("90".to_string()));

    // Continue to end
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    assert_eq!(output.lines[0].text, "Your adventure continues...");
}
