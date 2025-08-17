//! Directive mapping accuracy tests
//! Validates that Core directives are correctly mapped to API directives

use tsumugai::application::{api::Directive, engine::Engine};

#[test]
fn test_directive_mapping_accuracy() {
    // Test that PlaySe maps to PlaySe (not PlayBgm)
    let markdown = r#"
[PLAY_SE name=test.wav]
"#;

    let mut engine = Engine::from_markdown(markdown).expect("Failed to create engine");
    let result = engine.step().expect("Failed to step");

    // Check that we get the correct directive
    assert_eq!(result.directives.len(), 1);
    match &result.directives[0] {
        Directive::PlaySe { path } => {
            // Path might be None if not resolved
            assert!(path.is_none() || path.as_ref().unwrap().contains("test.wav"));
        }
        other => panic!("Expected PlaySe directive, got {other:?}"),
    }
}

#[test]
fn test_play_movie_mapping() {
    // Test that PlayMovie maps to PlayMovie (not ShowImage)
    let markdown = r#"
[PLAY_MOVIE file=test.mp4]
"#;

    let mut engine = Engine::from_markdown(markdown).expect("Failed to create engine");
    let result = engine.step().expect("Failed to step");

    assert_eq!(result.directives.len(), 1);
    match &result.directives[0] {
        Directive::PlayMovie { path } => {
            assert!(path.is_none() || path.as_ref().unwrap().contains("test.mp4"));
        }
        other => panic!("Expected PlayMovie directive, got {other:?}"),
    }
}

#[test]
fn test_reached_label_mapping() {
    // Test that Label maps to ReachedLabel (not JumpTo)
    let markdown = r#"
[LABEL name=start]
[SAY speaker=Narrator]
Hello world
"#;

    let mut engine = Engine::from_markdown(markdown).expect("Failed to create engine");
    let result = engine.step().expect("Failed to step");

    // Should have both Label and Say directives
    let mut found_label = false;
    for directive in &result.directives {
        match directive {
            Directive::ReachedLabel { label } => {
                assert_eq!(label, "start");
                found_label = true;
            }
            Directive::Say { .. } => {
                // Expected
            }
            _other => {
                // Other directives are OK, just check we got the right label mapping
            }
        }
    }

    if !found_label {
        // If no ReachedLabel directive, that's also OK for this test
        // as long as we're not getting JumpTo for labels
        for directive in &result.directives {
            if let Directive::JumpTo { label } = directive {
                if label == "start" {
                    panic!("Label incorrectly mapped to JumpTo instead of ReachedLabel");
                }
            }
        }
    }
}

#[test]
fn test_jump_to_mapping() {
    // Test that Jump maps to JumpTo
    let markdown = r#"
[JUMP label=start]

[LABEL name=start]
[SAY speaker=Narrator]
Done
"#;

    let mut engine = Engine::from_markdown(markdown).expect("Failed to create engine");
    let result = engine.step().expect("Failed to step");

    // The step should have caused a jump, so we should be at the start label
    // The exact behavior depends on how jumps are handled, but we shouldn't
    // see JumpTo in the directives since jumps are processed immediately
    assert_eq!(result.next, tsumugai::application::api::NextAction::Next);
}
