use tsumugai::{Engine, NextAction};

/// Integration test: Branch choice input loop
/// Verifies that BRANCH commands with invalid input allow re-input and eventually accept valid choices.
/// Metric: Count the number of input attempts until successful selection.
#[test]
fn branch_choice_input_loop() {
    let markdown = r#"
[SAY speaker=Guide]
Choose your path carefully.

[BRANCH choice=左へ label=go_left, choice=右へ label=go_right]

[LABEL name=go_left]

[SAY speaker=Guide]
You went left.

[LABEL name=go_right]

[SAY speaker=Guide]
You went right.
"#;

    let mut engine = Engine::from_markdown(markdown).expect("Failed to parse test scenario");

    // Step 1: SAY command should wait for user
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::WaitUser);
    assert_eq!(step_result.directives.len(), 1);

    // Step 2: BRANCH command should wait for branch selection
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::WaitBranch);
    assert_eq!(step_result.directives.len(), 1);
    
    match &step_result.directives[0] {
        tsumugai::Directive::Branch { choices } => {
            assert_eq!(choices.len(), 2);
            assert_eq!(choices[0], "左へ");
            assert_eq!(choices[1], "右へ");
        }
        _ => panic!("Expected Branch directive"),
    }

    // Test invalid input simulation: In new API, invalid choice would return error
    // Metric: Simulate 2 invalid attempts, then 1 successful attempt (total: 3 attempts)
    let mut input_attempts = 0;

    // Simulate invalid input 1 (out of range choice)
    input_attempts += 1;
    let invalid_result = engine.choose(5); // Invalid index
    assert!(invalid_result.is_err(), "Invalid choice should return error");

    // Simulate invalid input 2 (another out of range choice)
    input_attempts += 1;
    let invalid_result2 = engine.choose(10); // Another invalid index
    assert!(invalid_result2.is_err(), "Invalid choice should return error");

    // Simulate successful choice
    input_attempts += 1;
    let valid_result = engine.choose(0); // Choose "左へ" (index 0)
    assert!(valid_result.is_ok(), "Valid choice should succeed");

    // Verify we have the expected number of attempts
    assert_eq!(
        input_attempts, 3,
        "Expected 2 invalid + 1 successful = 3 attempts"
    );

    // Step 3: Should be at LABEL go_left
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::Next);

    // Step 4: Should be at SAY for left path
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::WaitUser);
    assert_eq!(step_result.directives.len(), 1);
    
    match &step_result.directives[0] {
        tsumugai::Directive::Say { speaker, text } => {
            assert_eq!(speaker, "Guide");
            assert_eq!(text, "You went left.");
        }
        _ => panic!("Expected Say directive"),
    }
}

/// Test that branch choices can successfully jump to different labels
#[test]
fn branch_choice_different_paths() {
    let markdown = r#"
[BRANCH choice=A label=path_a, choice=B label=path_b]

[LABEL name=path_a]
[SAY speaker=X] Path A

[LABEL name=path_b]
[SAY speaker=X] Path B
"#;

    let mut engine = Engine::from_markdown(markdown).expect("Failed to parse test scenario");

    // Get to branch
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::WaitBranch);
    assert_eq!(step_result.directives.len(), 1);

    // Test choosing path B (index 1)
    engine.choose(1).expect("Choice should succeed");

    // Should be at LABEL path_b
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::Next);

    // Should be at SAY for path B
    let step_result = engine.step().unwrap();
    assert_eq!(step_result.next, NextAction::WaitUser);
    assert_eq!(step_result.directives.len(), 1);

    match &step_result.directives[0] {
        tsumugai::Directive::Say { text, .. } => {
            assert_eq!(text, "Path B");
        }
        _ => panic!("Expected Say directive"),
    }
}
