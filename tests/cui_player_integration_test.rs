//! Integration tests for CUI player with ViewState

use tsumugai::{cli::view_state::ViewState, parse_scenario, player::PlayerSession, PlayerResult};

#[test]
fn test_view_state_differential_display() {
    let markdown = r#"
# scene: opening

[SHOW_IMAGE layer=bg name=school.png]
[PLAY_BGM name=morning.mp3]

[SAY speaker=Alice]
Hello!

[SAY speaker=Bob]
Hi!

[SHOW_IMAGE layer=bg name=school.png]

[SAY speaker=Alice]
How are you?
"#;

    let ast = parse_scenario(markdown).unwrap();
    let mut session = PlayerSession::new(ast);
    let mut view_state = ViewState::new();

    // Step 1: SceneBoundary
    match session.next() {
        PlayerResult::Step {
            display_step,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, Some("opening".to_string()));
            // Scene change should be detected
            assert!(delta.scene_changed);
            assert_eq!(delta.new_scene_name, Some("opening".to_string()));
        }
        _ => panic!("Expected step"),
    }

    // Step 2: First dialogue (with image and BGM)
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, None);
            // Should show new image and BGM
            assert!(delta
                .effects_added
                .iter()
                .any(|e| e.contains("ShowImage: school.png")));
            assert!(delta.effects_added.iter().any(|e| e.contains("PlayBGM")));
        }
        _ => panic!("Expected step"),
    }

    // Step 3: Second dialogue (no new effects)
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, None);
            // No new effects (image and BGM are the same)
            assert!(delta.effects_added.is_empty());
        }
        _ => panic!("Expected step"),
    }

    // Step 4: Third dialogue (same image again - should not show)
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, None);
            // Image is the same, should not be added
            assert!(!delta
                .effects_added
                .iter()
                .any(|e| e.contains("ShowImage")));
        }
        _ => panic!("Expected step"),
    }
}

#[test]
fn test_scene_change_detection() {
    let markdown = r#"
# scene: scene1

[SAY speaker=A]
In scene 1

# scene: scene2

[SAY speaker=B]
In scene 2
"#;

    let ast = parse_scenario(markdown).unwrap();
    let mut session = PlayerSession::new(ast);
    let mut view_state = ViewState::new();

    // Step 1: Scene 1 boundary
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, Some("scene1".to_string()));
            assert!(delta.scene_changed);
            assert_eq!(delta.new_scene_name, Some("scene1".to_string()));
        }
        _ => panic!("Expected step"),
    }

    // Step 2: Dialogue in scene 1
    let _ = session.next();

    // Step 3: Scene 2 boundary
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, Some("scene2".to_string()));
            assert!(delta.scene_changed);
            assert_eq!(delta.new_scene_name, Some("scene2".to_string()));
        }
        _ => panic!("Expected step"),
    }
}

#[test]
fn test_se_always_triggers() {
    let markdown = r#"
[PLAY_SE name=bell.wav]

[SAY speaker=A]
First

[PLAY_SE name=bell.wav]

[SAY speaker=A]
Second
"#;

    let ast = parse_scenario(markdown).unwrap();
    let mut session = PlayerSession::new(ast);
    let mut view_state = ViewState::new();

    // Step 1: First dialogue with SE
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, None);
            assert!(delta.effects_added.iter().any(|e| e.contains("PlaySE")));
        }
        _ => panic!("Expected step"),
    }

    // Step 2: Second dialogue with same SE (should still trigger)
    match session.next() {
        PlayerResult::Step {
            display_step: _,
            effects,
        } => {
            let delta = view_state.apply_effects(&effects, None);
            // SE should trigger again even though it's the same sound
            assert!(delta.effects_added.iter().any(|e| e.contains("PlaySE")));
        }
        _ => panic!("Expected step"),
    }
}
