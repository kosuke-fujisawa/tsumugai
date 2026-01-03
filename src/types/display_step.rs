//! Display step types for the player
//!
//! This module defines the concept of "display units" that represent
//! meaningful story progression points for the player.

use serde::{Deserialize, Serialize};

/// A single display unit in the story
///
/// DisplaySteps represent "what the player reads" rather than internal commands.
/// The player advances through the story one DisplayStep at a time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DisplayStep {
    /// A line of dialogue with a speaker
    Dialogue {
        speaker: String,
        text: String,
    },

    /// A narration line without a speaker
    Narration {
        text: String,
    },

    /// A choice block that requires player input
    ChoiceBlock {
        choices: Vec<ChoiceItem>,
    },

    /// A scene boundary (scene start)
    SceneBoundary {
        scene_name: String,
    },
}

/// A single choice option in a choice block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceItem {
    pub id: String,
    pub label: String,
    pub target: String,
}

/// Effects that were applied while advancing to a DisplayStep
///
/// Effects represent visual/audio changes (images, BGM, etc.) that
/// happen "behind the scenes" and are summarized for the player.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Effects {
    pub images: Vec<ImageEffect>,
    pub bgm: Option<String>,
    pub se: Vec<String>,
    pub other: Vec<String>,
}

/// An image effect (show or clear)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageEffect {
    pub layer: String,
    pub name: Option<String>, // None means clear
}

impl Effects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
            && self.bgm.is_none()
            && self.se.is_empty()
            && self.other.is_empty()
    }

    pub fn add_image(&mut self, layer: String, name: String) {
        self.images.push(ImageEffect {
            layer,
            name: Some(name),
        });
    }

    pub fn clear_layer(&mut self, layer: String) {
        self.images.push(ImageEffect {
            layer,
            name: None,
        });
    }

    pub fn set_bgm(&mut self, name: String) {
        self.bgm = Some(name);
    }

    pub fn add_se(&mut self, name: String) {
        self.se.push(name);
    }

    pub fn add_other(&mut self, effect: String) {
        self.other.push(effect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // DisplayStep tests
    #[test]
    fn test_display_step_dialogue_creation() {
        let step = DisplayStep::Dialogue {
            speaker: "Alice".to_string(),
            text: "Hello, world!".to_string(),
        };
        
        match step {
            DisplayStep::Dialogue { speaker, text } => {
                assert_eq!(speaker, "Alice");
                assert_eq!(text, "Hello, world!");
            }
            _ => panic!("Expected Dialogue variant"),
        }
    }

    #[test]
    fn test_display_step_narration_creation() {
        let step = DisplayStep::Narration {
            text: "The sun rose over the horizon.".to_string(),
        };
        
        match step {
            DisplayStep::Narration { text } => {
                assert_eq!(text, "The sun rose over the horizon.");
            }
            _ => panic!("Expected Narration variant"),
        }
    }

    #[test]
    fn test_display_step_choice_block_creation() {
        let choices = vec![
            ChoiceItem {
                id: "choice_0".to_string(),
                label: "Go left".to_string(),
                target: "left".to_string(),
            },
            ChoiceItem {
                id: "choice_1".to_string(),
                label: "Go right".to_string(),
                target: "right".to_string(),
            },
        ];
        
        let step = DisplayStep::ChoiceBlock { choices: choices.clone() };
        
        match step {
            DisplayStep::ChoiceBlock { choices: c } => {
                assert_eq!(c.len(), 2);
                assert_eq!(c[0].id, "choice_0");
                assert_eq!(c[0].label, "Go left");
                assert_eq!(c[1].target, "right");
            }
            _ => panic!("Expected ChoiceBlock variant"),
        }
    }

    #[test]
    fn test_display_step_scene_boundary_creation() {
        let step = DisplayStep::SceneBoundary {
            scene_name: "chapter_1".to_string(),
        };
        
        match step {
            DisplayStep::SceneBoundary { scene_name } => {
                assert_eq!(scene_name, "chapter_1");
            }
            _ => panic!("Expected SceneBoundary variant"),
        }
    }

    #[test]
    fn test_display_step_equality() {
        let step1 = DisplayStep::Dialogue {
            speaker: "Alice".to_string(),
            text: "Hi".to_string(),
        };
        let step2 = DisplayStep::Dialogue {
            speaker: "Alice".to_string(),
            text: "Hi".to_string(),
        };
        let step3 = DisplayStep::Dialogue {
            speaker: "Bob".to_string(),
            text: "Hi".to_string(),
        };
        
        assert_eq!(step1, step2);
        assert_ne!(step1, step3);
    }

    // ChoiceItem tests
    #[test]
    fn test_choice_item_creation() {
        let choice = ChoiceItem {
            id: "choice_0".to_string(),
            label: "Attack".to_string(),
            target: "battle".to_string(),
        };
        
        assert_eq!(choice.id, "choice_0");
        assert_eq!(choice.label, "Attack");
        assert_eq!(choice.target, "battle");
    }

    #[test]
    fn test_choice_item_clone() {
        let choice1 = ChoiceItem {
            id: "choice_0".to_string(),
            label: "Defend".to_string(),
            target: "defend_scene".to_string(),
        };
        
        let choice2 = choice1.clone();
        assert_eq!(choice1, choice2);
    }

    // Effects tests
    #[test]
    fn test_effects_new() {
        let effects = Effects::new();
        assert!(effects.images.is_empty());
        assert!(effects.bgm.is_none());
        assert!(effects.se.is_empty());
        assert!(effects.other.is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn test_effects_default() {
        let effects = Effects::default();
        assert!(effects.is_empty());
    }

    #[test]
    fn test_effects_add_image() {
        let mut effects = Effects::new();
        effects.add_image("bg".to_string(), "forest.png".to_string());
        
        assert_eq!(effects.images.len(), 1);
        assert_eq!(effects.images[0].layer, "bg");
        assert_eq!(effects.images[0].name, Some("forest.png".to_string()));
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_effects_add_multiple_images() {
        let mut effects = Effects::new();
        effects.add_image("bg".to_string(), "forest.png".to_string());
        effects.add_image("character".to_string(), "alice.png".to_string());
        effects.add_image("overlay".to_string(), "effect.png".to_string());
        
        assert_eq!(effects.images.len(), 3);
        assert_eq!(effects.images[0].layer, "bg");
        assert_eq!(effects.images[1].layer, "character");
        assert_eq!(effects.images[2].layer, "overlay");
    }

    #[test]
    fn test_effects_clear_layer() {
        let mut effects = Effects::new();
        effects.clear_layer("bg".to_string());
        
        assert_eq!(effects.images.len(), 1);
        assert_eq!(effects.images[0].layer, "bg");
        assert_eq!(effects.images[0].name, None);
    }

    #[test]
    fn test_effects_set_bgm() {
        let mut effects = Effects::new();
        effects.set_bgm("theme.mp3".to_string());
        
        assert_eq!(effects.bgm, Some("theme.mp3".to_string()));
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_effects_set_bgm_overwrite() {
        let mut effects = Effects::new();
        effects.set_bgm("theme1.mp3".to_string());
        effects.set_bgm("theme2.mp3".to_string());
        
        assert_eq!(effects.bgm, Some("theme2.mp3".to_string()));
    }

    #[test]
    fn test_effects_add_se() {
        let mut effects = Effects::new();
        effects.add_se("door.wav".to_string());
        
        assert_eq!(effects.se.len(), 1);
        assert_eq!(effects.se[0], "door.wav");
    }

    #[test]
    fn test_effects_add_multiple_se() {
        let mut effects = Effects::new();
        effects.add_se("door.wav".to_string());
        effects.add_se("footstep.wav".to_string());
        effects.add_se("bell.wav".to_string());
        
        assert_eq!(effects.se.len(), 3);
        assert_eq!(effects.se[0], "door.wav");
        assert_eq!(effects.se[1], "footstep.wav");
        assert_eq!(effects.se[2], "bell.wav");
    }

    #[test]
    fn test_effects_add_other() {
        let mut effects = Effects::new();
        effects.add_other("shake_screen".to_string());
        
        assert_eq!(effects.other.len(), 1);
        assert_eq!(effects.other[0], "shake_screen");
    }

    #[test]
    fn test_effects_is_empty_with_content() {
        let mut effects = Effects::new();
        assert!(effects.is_empty());
        
        effects.add_image("bg".to_string(), "test.png".to_string());
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_effects_is_empty_with_bgm() {
        let mut effects = Effects::new();
        effects.set_bgm("music.mp3".to_string());
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_effects_is_empty_with_se() {
        let mut effects = Effects::new();
        effects.add_se("sound.wav".to_string());
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_effects_is_empty_with_other() {
        let mut effects = Effects::new();
        effects.add_other("custom_effect".to_string());
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_effects_combined() {
        let mut effects = Effects::new();
        effects.add_image("bg".to_string(), "forest.png".to_string());
        effects.set_bgm("ambient.mp3".to_string());
        effects.add_se("bird.wav".to_string());
        effects.add_se("wind.wav".to_string());
        effects.add_other("weather:rain".to_string());
        
        assert_eq!(effects.images.len(), 1);
        assert_eq!(effects.bgm, Some("ambient.mp3".to_string()));
        assert_eq!(effects.se.len(), 2);
        assert_eq!(effects.other.len(), 1);
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_image_effect_with_name() {
        let image = ImageEffect {
            layer: "bg".to_string(),
            name: Some("castle.png".to_string()),
        };
        
        assert_eq!(image.layer, "bg");
        assert_eq!(image.name, Some("castle.png".to_string()));
    }

    #[test]
    fn test_image_effect_clear() {
        let image = ImageEffect {
            layer: "overlay".to_string(),
            name: None,
        };
        
        assert_eq!(image.layer, "overlay");
        assert_eq!(image.name, None);
    }

    #[test]
    fn test_image_effect_equality() {
        let img1 = ImageEffect {
            layer: "bg".to_string(),
            name: Some("test.png".to_string()),
        };
        let img2 = ImageEffect {
            layer: "bg".to_string(),
            name: Some("test.png".to_string()),
        };
        let img3 = ImageEffect {
            layer: "bg".to_string(),
            name: None,
        };
        
        assert_eq!(img1, img2);
        assert_ne!(img1, img3);
    }

    // Serialization tests
    #[test]
    fn test_display_step_serialization() {
        let step = DisplayStep::Dialogue {
            speaker: "Alice".to_string(),
            text: "Hello".to_string(),
        };
        
        let serialized = serde_json::to_string(&step).unwrap();
        let deserialized: DisplayStep = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(step, deserialized);
    }

    #[test]
    fn test_effects_serialization() {
        let mut effects = Effects::new();
        effects.add_image("bg".to_string(), "test.png".to_string());
        effects.set_bgm("music.mp3".to_string());
        
        let serialized = serde_json::to_string(&effects).unwrap();
        let deserialized: Effects = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(effects.images, deserialized.images);
        assert_eq!(effects.bgm, deserialized.bgm);
    }
}
