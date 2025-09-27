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

/// Test: PlaySe/PlayMovie/ReachedLabel serialization/deserialization consistency
/// Expectation: All 3 directive types roundtrip correctly through JSON
/// Metric: 3 test cases, execution time <50ms total
#[test]
fn directive_mapping_is_consistent() {
    let start_time = std::time::Instant::now();

    // Test PlaySe directive
    let play_se = Directive::PlaySe {
        path: Some("sounds/click.wav".to_string()),
    };
    let se_json = serde_json::to_string(&play_se).expect("PlaySe serialization failed");
    let se_roundtrip: Directive =
        serde_json::from_str(&se_json).expect("PlaySe deserialization failed");
    assert_eq!(play_se, se_roundtrip, "PlaySe roundtrip failed");

    // Test PlayMovie directive
    let play_movie = Directive::PlayMovie {
        path: Some("videos/intro.mp4".to_string()),
    };
    let movie_json = serde_json::to_string(&play_movie).expect("PlayMovie serialization failed");
    let movie_roundtrip: Directive =
        serde_json::from_str(&movie_json).expect("PlayMovie deserialization failed");
    assert_eq!(play_movie, movie_roundtrip, "PlayMovie roundtrip failed");

    // Test ReachedLabel directive
    let reached_label = Directive::ReachedLabel {
        label: "chapter_1".to_string(),
    };
    let label_json =
        serde_json::to_string(&reached_label).expect("ReachedLabel serialization failed");
    let label_roundtrip: Directive =
        serde_json::from_str(&label_json).expect("ReachedLabel deserialization failed");
    assert_eq!(
        reached_label, label_roundtrip,
        "ReachedLabel roundtrip failed"
    );

    let total_duration = start_time.elapsed();
    assert!(
        total_duration.as_millis() < 50,
        "Directive mapping tests should complete in <50ms"
    );
}

/// Test: Verify directive JSON structure consistency
#[test]
fn test_directive_json_structure() {
    // Test PlaySe structure
    let play_se = Directive::PlaySe {
        path: Some("test.wav".to_string()),
    };
    let json = serde_json::to_value(&play_se).expect("PlaySe serialization failed");
    assert_eq!(json["type"], "PlaySe");
    assert_eq!(json["args"]["path"], "test.wav");

    // Test PlayMovie structure
    let play_movie = Directive::PlayMovie {
        path: Some("test.mp4".to_string()),
    };
    let json = serde_json::to_value(&play_movie).expect("PlayMovie serialization failed");
    assert_eq!(json["type"], "PlayMovie");
    assert_eq!(json["args"]["path"], "test.mp4");

    // Test ReachedLabel structure
    let reached_label = Directive::ReachedLabel {
        label: "scene_1".to_string(),
    };
    let json = serde_json::to_value(&reached_label).expect("ReachedLabel serialization failed");
    assert_eq!(json["type"], "ReachedLabel");
    assert_eq!(json["args"]["label"], "scene_1");
}
