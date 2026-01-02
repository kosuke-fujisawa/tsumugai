//! Golden snapshot tests for output consistency
//!
//! These tests ensure that given the same input scenario and events,
//! the output remains consistent across changes.

use std::fs;
use std::path::PathBuf;
use tsumugai::{
    parser, runtime,
    types::{Event, State},
};

fn golden_path(test_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("new_api")
        .join(format!("{}.json", test_name))
}

fn compare_or_update_golden(test_name: &str, actual_json: &str) {
    let golden_file = golden_path(test_name);

    // Create parent directory if it doesn't exist
    if let Some(parent) = golden_file.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    // Check if we're in update mode (via environment variable)
    let update_mode = std::env::var("UPDATE_GOLDEN").is_ok();

    if update_mode || !golden_file.exists() {
        // Write new golden file
        fs::write(&golden_file, actual_json).unwrap();
        eprintln!("Updated golden file: {:?}", golden_file);
    } else {
        // Compare with existing golden file
        let expected = fs::read_to_string(&golden_file)
            .unwrap_or_else(|_| panic!("Failed to read golden file: {:?}", golden_file));

        if actual_json.trim() != expected.trim() {
            eprintln!("Golden test failed: {}", test_name);
            eprintln!("Expected:\n{}", expected);
            eprintln!("Actual:\n{}", actual_json);
            panic!("Golden test output mismatch. Run with UPDATE_GOLDEN=1 to update.");
        }
    }
}

#[test]
fn golden_simple_dialogue() {
    let markdown = r#"
[SAY speaker=Alice]
Hello!

[SAY speaker=Bob]
Hi there!
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // Execute first step
    let (_, output) = runtime::step(state, &ast, None);
    let output_json = serde_json::to_string_pretty(&output).unwrap();

    compare_or_update_golden("simple_dialogue_step1", &output_json);
}

#[test]
fn golden_branch_with_choices() {
    let markdown = r#"
[BRANCH choice=option_a choice=option_b choice=option_c]

[LABEL name=option_a]
[SAY speaker=System]
A

[LABEL name=option_b]
[SAY speaker=System]
B

[LABEL name=option_c]
[SAY speaker=System]
C
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // Get branch choices
    let (state, output) = runtime::step(state, &ast, None);
    let output_json = serde_json::to_string_pretty(&output).unwrap();

    compare_or_update_golden("branch_choices", &output_json);

    // Select first choice
    let event = Event::Choice {
        id: "choice_0".to_string(),
    };
    let (_, output) = runtime::step(state, &ast, Some(event));
    let output_json = serde_json::to_string_pretty(&output).unwrap();

    compare_or_update_golden("branch_after_choice_0", &output_json);
}

#[test]
fn golden_effects_output() {
    let markdown = r#"
[PLAY_BGM name=battle.mp3]
[SHOW_IMAGE layer=background name=forest.jpg]
[WAIT 2.5s]
"#;

    let ast = parser::parse(markdown).unwrap();
    let mut state = State::new();

    // Step 1: BGM
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    let output_json = serde_json::to_string_pretty(&output).unwrap();
    compare_or_update_golden("effects_bgm", &output_json);

    // Step 2: Image
    let (new_state, output) = runtime::step(state, &ast, None);
    state = new_state;
    let output_json = serde_json::to_string_pretty(&output).unwrap();
    compare_or_update_golden("effects_image", &output_json);

    // Step 3: Wait
    let (_, output) = runtime::step(state, &ast, None);
    let output_json = serde_json::to_string_pretty(&output).unwrap();
    compare_or_update_golden("effects_wait", &output_json);
}

#[test]
fn golden_variable_operations() {
    let markdown = r#"
[SET name=counter value=0]
[MODIFY name=counter op=add value=10]
[MODIFY name=counter op=mul value=2]
[SET name=message value=done]
[SAY speaker=System]
Operations complete.
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // Execute all operations and stop at SAY
    let (state, output) = runtime::step(state, &ast, None);

    // Verify state
    assert_eq!(state.get_var("counter"), Some("20".to_string()));
    assert_eq!(state.get_var("message"), Some("done".to_string()));

    let output_json = serde_json::to_string_pretty(&output).unwrap();
    compare_or_update_golden("variable_operations", &output_json);
}

#[test]
fn golden_conditional_flow() {
    let markdown = r#"
[SET name=score value=100]
[JUMP_IF var=score cmp=ge value=50 label=pass]

[LABEL name=fail]
[SAY speaker=System]
You failed.
[JUMP label=end]

[LABEL name=pass]
[SAY speaker=System]
You passed!

[LABEL name=end]
[SAY speaker=System]
Game over.
"#;

    let ast = parser::parse(markdown).unwrap();
    let state = State::new();

    // Execute - should jump to pass
    let (state, output) = runtime::step(state, &ast, None);
    let output_json = serde_json::to_string_pretty(&output).unwrap();
    compare_or_update_golden("conditional_pass", &output_json);

    // Continue to end
    let (_, output) = runtime::step(state, &ast, None);
    let output_json = serde_json::to_string_pretty(&output).unwrap();
    compare_or_update_golden("conditional_end", &output_json);
}
