//! Narrative event types for player mode
//!
//! This module provides types for converting runtime output into
//! human-readable narrative events suitable for CUI player mode.

use serde::{Deserialize, Serialize};

/// A narrative event representing what should be presented to the player
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NarrativeEvent {
    /// Dialogue or narration text
    Dialogue {
        speaker: Option<String>,
        text: String,
    },
    /// Player choices
    Choices { choices: Vec<ChoiceOption> },
    /// Visual or audio effect (images, BGM, SE, etc.)
    Effect {
        kind: String,
        data: Option<serde_json::Value>,
    },
    /// End of scenario
    End,
}

/// A single choice option
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChoiceOption {
    /// Unique identifier for this choice (e.g., "choice_0")
    pub id: String,
    /// Display text for the choice
    pub label: String,
}

impl NarrativeEvent {
    /// Create a dialogue event
    pub fn dialogue(speaker: Option<String>, text: String) -> Self {
        Self::Dialogue { speaker, text }
    }

    /// Create a choices event
    pub fn choices(choices: Vec<ChoiceOption>) -> Self {
        Self::Choices { choices }
    }

    /// Create an effect event
    pub fn effect(kind: String, data: Option<serde_json::Value>) -> Self {
        Self::Effect { kind, data }
    }

    /// Create an end event
    pub fn end() -> Self {
        Self::End
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NarrativeEvent constructor tests
    #[test]
    fn test_dialogue_constructor_with_speaker() {
        let event = NarrativeEvent::dialogue(Some("Alice".to_string()), "Hello!".to_string());
        
        match event {
            NarrativeEvent::Dialogue { speaker, text } => {
                assert_eq!(speaker, Some("Alice".to_string()));
                assert_eq!(text, "Hello!");
            }
            _ => panic!("Expected Dialogue variant"),
        }
    }

    #[test]
    fn test_dialogue_constructor_without_speaker() {
        let event = NarrativeEvent::dialogue(None, "The wind blows...".to_string());
        
        match event {
            NarrativeEvent::Dialogue { speaker, text } => {
                assert_eq!(speaker, None);
                assert_eq!(text, "The wind blows...");
            }
            _ => panic!("Expected Dialogue variant"),
        }
    }

    #[test]
    fn test_choices_constructor() {
        let choices = vec![
            ChoiceOption {
                id: "choice_0".to_string(),
                label: "Yes".to_string(),
            },
            ChoiceOption {
                id: "choice_1".to_string(),
                label: "No".to_string(),
            },
        ];
        
        let event = NarrativeEvent::choices(choices.clone());
        
        match event {
            NarrativeEvent::Choices { choices: c } => {
                assert_eq!(c.len(), 2);
                assert_eq!(c[0].id, "choice_0");
                assert_eq!(c[0].label, "Yes");
                assert_eq!(c[1].label, "No");
            }
            _ => panic!("Expected Choices variant"),
        }
    }

    #[test]
    fn test_choices_constructor_empty() {
        let event = NarrativeEvent::choices(vec![]);
        
        match event {
            NarrativeEvent::Choices { choices } => {
                assert_eq!(choices.len(), 0);
            }
            _ => panic!("Expected Choices variant"),
        }
    }

    #[test]
    fn test_effect_constructor_with_data() {
        let data = serde_json::json!({
            "layer": "bg",
            "name": "forest.png"
        });
        
        let event = NarrativeEvent::effect("show_image".to_string(), Some(data.clone()));
        
        match event {
            NarrativeEvent::Effect { kind, data: d } => {
                assert_eq!(kind, "show_image");
                assert_eq!(d, Some(data));
            }
            _ => panic!("Expected Effect variant"),
        }
    }

    #[test]
    fn test_effect_constructor_without_data() {
        let event = NarrativeEvent::effect("play_bgm".to_string(), None);
        
        match event {
            NarrativeEvent::Effect { kind, data } => {
                assert_eq!(kind, "play_bgm");
                assert_eq!(data, None);
            }
            _ => panic!("Expected Effect variant"),
        }
    }

    #[test]
    fn test_end_constructor() {
        let event = NarrativeEvent::end();
        assert!(matches!(event, NarrativeEvent::End));
    }

    // NarrativeEvent equality tests
    #[test]
    fn test_dialogue_equality() {
        let event1 = NarrativeEvent::dialogue(Some("Alice".to_string()), "Hi".to_string());
        let event2 = NarrativeEvent::dialogue(Some("Alice".to_string()), "Hi".to_string());
        let event3 = NarrativeEvent::dialogue(Some("Bob".to_string()), "Hi".to_string());
        
        assert_eq!(event1, event2);
        assert_ne!(event1, event3);
    }

    #[test]
    fn test_end_equality() {
        let event1 = NarrativeEvent::end();
        let event2 = NarrativeEvent::end();
        assert_eq!(event1, event2);
    }

    // ChoiceOption tests
    #[test]
    fn test_choice_option_creation() {
        let choice = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Attack".to_string(),
        };
        
        assert_eq!(choice.id, "choice_0");
        assert_eq!(choice.label, "Attack");
    }

    #[test]
    fn test_choice_option_clone() {
        let choice1 = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Defend".to_string(),
        };
        
        let choice2 = choice1.clone();
        assert_eq!(choice1, choice2);
    }

    #[test]
    fn test_choice_option_equality() {
        let choice1 = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Go left".to_string(),
        };
        let choice2 = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Go left".to_string(),
        };
        let choice3 = ChoiceOption {
            id: "choice_1".to_string(),
            label: "Go left".to_string(),
        };
        
        assert_eq!(choice1, choice2);
        assert_ne!(choice1, choice3);
    }

    // Serialization tests
    #[test]
    fn test_dialogue_serialization() {
        let event = NarrativeEvent::dialogue(Some("Alice".to_string()), "Hello".to_string());
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: NarrativeEvent = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_choices_serialization() {
        let choices = vec![
            ChoiceOption {
                id: "choice_0".to_string(),
                label: "Yes".to_string(),
            },
        ];
        let event = NarrativeEvent::choices(choices);
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: NarrativeEvent = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_effect_serialization() {
        let data = serde_json::json!({"key": "value"});
        let event = NarrativeEvent::effect("custom".to_string(), Some(data));
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: NarrativeEvent = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_end_serialization() {
        let event = NarrativeEvent::end();
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: NarrativeEvent = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_choice_option_serialization() {
        let choice = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Test".to_string(),
        };
        
        let serialized = serde_json::to_string(&choice).unwrap();
        let deserialized: ChoiceOption = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(choice, deserialized);
    }

    // Edge case tests
    #[test]
    fn test_dialogue_with_empty_text() {
        let event = NarrativeEvent::dialogue(Some("Alice".to_string()), "".to_string());
        
        match event {
            NarrativeEvent::Dialogue { speaker, text } => {
                assert_eq!(speaker, Some("Alice".to_string()));
                assert_eq!(text, "");
            }
            _ => panic!("Expected Dialogue variant"),
        }
    }

    #[test]
    fn test_dialogue_with_multiline_text() {
        let text = "Line 1\nLine 2\nLine 3".to_string();
        let event = NarrativeEvent::dialogue(None, text.clone());
        
        match event {
            NarrativeEvent::Dialogue { speaker, text: t } => {
                assert_eq!(speaker, None);
                assert_eq!(t, text);
            }
            _ => panic!("Expected Dialogue variant"),
        }
    }

    #[test]
    fn test_choice_with_special_characters() {
        let choice = ChoiceOption {
            id: "choice_0".to_string(),
            label: "Go to \"room\"!".to_string(),
        };
        
        assert_eq!(choice.label, "Go to \"room\"!");
    }

    #[test]
    fn test_effect_with_complex_json() {
        let data = serde_json::json!({
            "nested": {
                "array": [1, 2, 3],
                "object": {"key": "value"}
            }
        });
        
        let event = NarrativeEvent::effect("complex".to_string(), Some(data.clone()));
        
        match event {
            NarrativeEvent::Effect { kind, data: d } => {
                assert_eq!(kind, "complex");
                assert_eq!(d, Some(data));
            }
            _ => panic!("Expected Effect variant"),
        }
    }
}
