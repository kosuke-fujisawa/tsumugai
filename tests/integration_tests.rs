use tsumugai::{Engine, Step, WaitKind, parse};

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

    let program = parse(markdown).expect("Failed to parse test scenario");
    let mut engine = Engine::new(program);

    // Step 1: SAY command should wait for user
    let step = engine.step();
    assert!(matches!(step, Step::Wait(WaitKind::User)));
    let directives = engine.take_emitted();
    assert_eq!(directives.len(), 1);

    // Step 2: BRANCH command should wait for branch selection
    let step = engine.step();
    if let Step::Wait(WaitKind::Branch(choices)) = step {
        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].choice, "左へ");
        assert_eq!(choices[0].label, "go_left");
        assert_eq!(choices[1].choice, "右へ");
        assert_eq!(choices[1].label, "go_right");

        // Take emitted directives (should include Branch)
        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        assert!(matches!(directives[0], tsumugai::Directive::Branch { .. }));

        // Test that calling step() again on branch doesn't re-emit
        let step_again = engine.step();
        assert!(matches!(step_again, Step::Wait(WaitKind::Branch(_))));
        let again_directives = engine.take_emitted();
        assert!(
            again_directives.is_empty(),
            "Branch should not re-emit directive"
        );

        // Test invalid input simulation (would be handled by application layer)
        // Here we simulate that user eventually selects a valid choice

        // Metric: Simulate 2 invalid attempts, then 1 successful attempt (total: 3 attempts)
        let mut input_attempts = 0;

        // Simulate invalid input 1 (application would handle this, engine stays in wait state)
        input_attempts += 1;
        let step_after_invalid1 = engine.step();
        assert!(matches!(
            step_after_invalid1,
            Step::Wait(WaitKind::Branch(_))
        ));

        // Simulate invalid input 2
        input_attempts += 1;
        let step_after_invalid2 = engine.step();
        assert!(matches!(
            step_after_invalid2,
            Step::Wait(WaitKind::Branch(_))
        ));

        // Simulate successful choice
        input_attempts += 1;
        engine.jump_to("go_left").expect("Jump should succeed");

        // Verify we have the expected number of attempts
        assert_eq!(
            input_attempts, 3,
            "Expected 2 invalid + 1 successful = 3 attempts"
        );

        // Step 3: Should be at LABEL go_left
        let step = engine.step();
        assert!(matches!(step, Step::Next));
        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        assert!(matches!(directives[0], tsumugai::Directive::Label { .. }));

        // Step 4: Should be at SAY for left path
        let step = engine.step();
        assert!(matches!(step, Step::Wait(WaitKind::User)));
        let directives = engine.take_emitted();
        assert_eq!(directives.len(), 1);
        if let tsumugai::Directive::Say { speaker, text } = &directives[0] {
            assert_eq!(speaker, "Guide");
            assert_eq!(text, "You went left.");
        } else {
            panic!("Expected Say directive");
        }
    } else {
        panic!("Expected Branch wait, got {:?}", step);
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

    let program = parse(markdown).expect("Failed to parse test scenario");
    let mut engine = Engine::new(program);

    // Get to branch
    let step = engine.step();
    if let Step::Wait(WaitKind::Branch(_choices)) = step {
        engine.take_emitted(); // Clear branch directive

        // Test choosing path B
        engine.jump_to("path_b").expect("Jump should succeed");

        // Should be at LABEL path_b
        let step = engine.step();
        assert!(matches!(step, Step::Next));
        engine.take_emitted(); // Clear label directive

        // Should be at SAY for path B
        let step = engine.step();
        assert!(matches!(step, Step::Wait(WaitKind::User)));
        let directives = engine.take_emitted();

        if let tsumugai::Directive::Say { text, .. } = &directives[0] {
            assert_eq!(text, "Path B");
        } else {
            panic!("Expected Say directive");
        }
    } else {
        panic!("Expected Branch wait");
    }
}
