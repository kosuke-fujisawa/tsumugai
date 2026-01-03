//! View state management for CUI player
//!
//! This module manages the visual state of the CUI player and calculates
//! rendering deltas to only show what has changed.

use crate::types::display_step::{Effects, ImageEffect};
use std::collections::HashMap;

/// Clear the terminal screen (cross-platform)
pub fn clear_screen() {
    // Try ANSI escape codes first (works on most terminals)
    print!("\x1b[2J\x1b[H");

    // Fallback: print newlines
    // This is a simple fallback that works everywhere
    if std::io::Write::flush(&mut std::io::stdout()).is_err() {
        for _ in 0..50 {
            println!();
        }
    }
}

/// Represents the current visual state of the CUI player
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// Current scene name
    pub scene_name: Option<String>,
    /// Images currently displayed (layer -> ImageEffect)
    pub images: HashMap<String, ImageEffect>,
    /// Currently playing BGM
    pub bgm: Option<String>,
    /// Last played SE (for display purposes)
    pub se_last: Option<String>,
}

impl ViewState {
    /// Create a new empty ViewState
    pub fn new() -> Self {
        Self {
            scene_name: None,
            images: HashMap::new(),
            bgm: None,
            se_last: None,
        }
    }

    /// Apply effects to this view state and return the rendering delta
    pub fn apply_effects(&mut self, effects: &Effects, scene_name: Option<String>) -> RenderDelta {
        let mut delta = RenderDelta::new();

        // Check scene change
        if scene_name != self.scene_name {
            delta.scene_changed = scene_name.is_some();
            delta.new_scene_name = scene_name.clone();
            self.scene_name = scene_name;
        }

        // Process image changes
        for image in &effects.images {
            let layer = image.layer.clone();

            if let Some(name) = &image.name {
                // Check if this is a new or different image
                let is_new = match self.images.get(&layer) {
                    Some(existing) => existing.name.as_ref() != Some(name),
                    None => true,
                };

                if is_new {
                    delta.effects_added.push(format!("ShowImage: {} ({})", name, layer));
                    self.images.insert(layer.clone(), image.clone());
                }
            } else {
                // Clear layer
                if self.images.remove(&layer).is_some() {
                    delta.effects_added.push(format!("ClearLayer: {}", layer));
                }
            }
        }

        // Check BGM change
        if let Some(bgm) = &effects.bgm {
            if self.bgm.as_ref() != Some(bgm) {
                delta.effects_added.push(format!("PlayBGM: {}", bgm));
                self.bgm = Some(bgm.clone());
            }
        }

        // SE always triggers (not persistent state)
        for se in &effects.se {
            delta.effects_added.push(format!("PlaySE: {}", se));
            self.se_last = Some(se.clone());
        }

        // Other effects
        for other in &effects.other {
            delta.effects_added.push(other.clone());
        }

        delta
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents what needs to be rendered (the delta from previous state)
#[derive(Debug, Clone, PartialEq)]
pub struct RenderDelta {
    /// Whether the scene changed
    pub scene_changed: bool,
    /// New scene name (if changed)
    pub new_scene_name: Option<String>,
    /// Effects that were added (human-readable strings)
    pub effects_added: Vec<String>,
}

impl RenderDelta {
    /// Create a new empty RenderDelta
    pub fn new() -> Self {
        Self {
            scene_changed: false,
            new_scene_name: None,
            effects_added: Vec::new(),
        }
    }

    /// Check if this delta has any changes
    pub fn is_empty(&self) -> bool {
        !self.scene_changed && self.effects_added.is_empty()
    }
}

impl Default for RenderDelta {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a delta to the console
pub fn render_delta(delta: &RenderDelta) {
    // Show scene change if any
    if delta.scene_changed {
        if let Some(scene_name) = &delta.new_scene_name {
            println!("=== Scene: {} ===", scene_name);
            println!();
        }
    }

    // Show effects if any
    if !delta.effects_added.is_empty() {
        println!("[Effects]");
        for effect in &delta.effects_added {
            println!("  {}", effect);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_view_state() {
        let view = ViewState::new();
        assert_eq!(view.scene_name, None);
        assert!(view.images.is_empty());
        assert_eq!(view.bgm, None);
        assert_eq!(view.se_last, None);
    }

    #[test]
    fn test_scene_change() {
        let mut view = ViewState::new();
        let effects = Effects::new();

        let delta = view.apply_effects(&effects, Some("scene1".to_string()));

        assert!(delta.scene_changed);
        assert_eq!(delta.new_scene_name, Some("scene1".to_string()));
        assert_eq!(view.scene_name, Some("scene1".to_string()));
    }

    #[test]
    fn test_no_scene_change_on_same_scene() {
        let mut view = ViewState::new();
        view.scene_name = Some("scene1".to_string());
        let effects = Effects::new();

        let delta = view.apply_effects(&effects, Some("scene1".to_string()));

        assert!(!delta.scene_changed);
    }

    #[test]
    fn test_bgm_change() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        effects.bgm = Some("bgm1.mp3".to_string());

        let delta = view.apply_effects(&effects, None);

        assert!(delta.effects_added.contains(&"PlayBGM: bgm1.mp3".to_string()));
        assert_eq!(view.bgm, Some("bgm1.mp3".to_string()));
    }

    #[test]
    fn test_bgm_no_change_on_same_bgm() {
        let mut view = ViewState::new();
        view.bgm = Some("bgm1.mp3".to_string());

        let mut effects = Effects::new();
        effects.bgm = Some("bgm1.mp3".to_string());

        let delta = view.apply_effects(&effects, None);

        // Should not add BGM to effects_added since it's the same
        assert!(!delta.effects_added.iter().any(|e| e.contains("PlayBGM")));
    }

    #[test]
    fn test_image_addition() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        effects.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("bg1.png".to_string()),
        });

        let delta = view.apply_effects(&effects, None);

        assert!(delta.effects_added.contains(&"ShowImage: bg1.png (bg)".to_string()));
        assert!(view.images.contains_key("bg"));
    }

    #[test]
    fn test_image_no_change_on_same_image() {
        let mut view = ViewState::new();
        view.images.insert("bg".to_string(), ImageEffect {
            layer: "bg".to_string(),
            name: Some("bg1.png".to_string()),
        });

        let mut effects = Effects::new();
        effects.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("bg1.png".to_string()),
        });

        let delta = view.apply_effects(&effects, None);

        // Should not add image to effects_added since it's the same
        assert!(!delta.effects_added.iter().any(|e| e.contains("ShowImage")));
    }

    #[test]
    fn test_se_always_triggers() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        effects.se.push("se1.wav".to_string());

        let delta1 = view.apply_effects(&effects, None);
        assert!(delta1.effects_added.contains(&"PlaySE: se1.wav".to_string()));

        // SE should trigger again even with same sound
        let delta2 = view.apply_effects(&effects, None);
        assert!(delta2.effects_added.contains(&"PlaySE: se1.wav".to_string()));
    }

    #[test]
    fn test_view_state_default() {
        let view = ViewState::default();
        assert_eq!(view.scene_name, None);
        assert!(view.images.is_empty());
        assert_eq!(view.bgm, None);
        assert_eq!(view.se_last, None);
    }

    #[test]
    fn test_multiple_scene_changes() {
        let mut view = ViewState::new();
        let effects = Effects::new();

        let delta1 = view.apply_effects(&effects, Some("scene1".to_string()));
        assert!(delta1.scene_changed);
        assert_eq!(view.scene_name, Some("scene1".to_string()));

        let delta2 = view.apply_effects(&effects, Some("scene2".to_string()));
        assert!(delta2.scene_changed);
        assert_eq!(view.scene_name, Some("scene2".to_string()));

        let delta3 = view.apply_effects(&effects, Some("scene3".to_string()));
        assert!(delta3.scene_changed);
        assert_eq!(view.scene_name, Some("scene3".to_string()));
    }

    #[test]
    fn test_image_layer_overwrite() {
        let mut view = ViewState::new();
        
        // Add first image to bg layer
        let mut effects1 = Effects::new();
        effects1.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("forest.png".to_string()),
        });
        
        let delta1 = view.apply_effects(&effects1, None);
        assert!(delta1.effects_added.iter().any(|e| e.contains("forest.png")));
        
        // Replace with different image on same layer
        let mut effects2 = Effects::new();
        effects2.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("castle.png".to_string()),
        });
        
        let delta2 = view.apply_effects(&effects2, None);
        assert!(delta2.effects_added.iter().any(|e| e.contains("castle.png")));
        
        // View should have the new image
        assert_eq!(view.images.get("bg").unwrap().name, Some("castle.png".to_string()));
    }

    #[test]
    fn test_multiple_image_layers() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        
        effects.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("bg.png".to_string()),
        });
        effects.images.push(ImageEffect {
            layer: "character".to_string(),
            name: Some("alice.png".to_string()),
        });
        effects.images.push(ImageEffect {
            layer: "overlay".to_string(),
            name: Some("effect.png".to_string()),
        });
        
        let delta = view.apply_effects(&effects, None);
        assert_eq!(delta.effects_added.len(), 3);
        assert_eq!(view.images.len(), 3);
        
        assert!(view.images.contains_key("bg"));
        assert!(view.images.contains_key("character"));
        assert!(view.images.contains_key("overlay"));
    }

    #[test]
    fn test_clear_layer_removes_image() {
        let mut view = ViewState::new();
        
        // First add an image
        let mut effects1 = Effects::new();
        effects1.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("bg.png".to_string()),
        });
        view.apply_effects(&effects1, None);
        
        assert!(view.images.contains_key("bg"));
        
        // Now clear it
        let mut effects2 = Effects::new();
        effects2.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: None,
        });
        
        let delta = view.apply_effects(&effects2, None);
        assert!(delta.effects_added.iter().any(|e| e.contains("ClearLayer")));
        assert!(!view.images.contains_key("bg"));
    }

    #[test]
    fn test_clear_nonexistent_layer() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        
        effects.images.push(ImageEffect {
            layer: "nonexistent".to_string(),
            name: None,
        });
        
        let delta = view.apply_effects(&effects, None);
        // Should not add anything since layer didn't exist
        assert!(!delta.effects_added.iter().any(|e| e.contains("ClearLayer")));
    }

    #[test]
    fn test_bgm_change_sequence() {
        let mut view = ViewState::new();
        
        let mut effects1 = Effects::new();
        effects1.bgm = Some("theme1.mp3".to_string());
        let delta1 = view.apply_effects(&effects1, None);
        assert!(delta1.effects_added.iter().any(|e| e.contains("theme1")));
        
        let mut effects2 = Effects::new();
        effects2.bgm = Some("theme2.mp3".to_string());
        let delta2 = view.apply_effects(&effects2, None);
        assert!(delta2.effects_added.iter().any(|e| e.contains("theme2")));
        
        assert_eq!(view.bgm, Some("theme2.mp3".to_string()));
    }

    #[test]
    fn test_se_tracks_last_played() {
        let mut view = ViewState::new();
        
        let mut effects1 = Effects::new();
        effects1.se.push("door.wav".to_string());
        view.apply_effects(&effects1, None);
        assert_eq!(view.se_last, Some("door.wav".to_string()));
        
        let mut effects2 = Effects::new();
        effects2.se.push("bell.wav".to_string());
        view.apply_effects(&effects2, None);
        assert_eq!(view.se_last, Some("bell.wav".to_string()));
    }

    #[test]
    fn test_multiple_se_in_one_effect() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        
        effects.se.push("sound1.wav".to_string());
        effects.se.push("sound2.wav".to_string());
        effects.se.push("sound3.wav".to_string());
        
        let delta = view.apply_effects(&effects, None);
        assert_eq!(delta.effects_added.iter().filter(|e| e.contains("PlaySE")).count(), 3);
        
        // Last SE should be tracked
        assert_eq!(view.se_last, Some("sound3.wav".to_string()));
    }

    #[test]
    fn test_render_delta_new() {
        let delta = RenderDelta::new();
        assert!(!delta.scene_changed);
        assert_eq!(delta.new_scene_name, None);
        assert!(delta.effects_added.is_empty());
        assert!(delta.is_empty());
    }

    #[test]
    fn test_render_delta_default() {
        let delta = RenderDelta::default();
        assert!(delta.is_empty());
    }

    #[test]
    fn test_render_delta_is_empty() {
        let mut delta = RenderDelta::new();
        assert!(delta.is_empty());
        
        delta.scene_changed = true;
        assert!(!delta.is_empty());
        
        delta.scene_changed = false;
        delta.effects_added.push("test".to_string());
        assert!(!delta.is_empty());
    }

    #[test]
    fn test_combined_scene_and_effects() {
        let mut view = ViewState::new();
        let mut effects = Effects::new();
        
        effects.images.push(ImageEffect {
            layer: "bg".to_string(),
            name: Some("scene1_bg.png".to_string()),
        });
        effects.bgm = Some("scene1_theme.mp3".to_string());
        effects.se.push("door_open.wav".to_string());
        
        let delta = view.apply_effects(&effects, Some("scene1".to_string()));
        
        assert!(delta.scene_changed);
        assert_eq!(delta.new_scene_name, Some("scene1".to_string()));
        assert!(delta.effects_added.iter().any(|e| e.contains("ShowImage")));
        assert!(delta.effects_added.iter().any(|e| e.contains("PlayBGM")));
        assert!(delta.effects_added.iter().any(|e| e.contains("PlaySE")));
    }

    #[test]
    fn test_view_state_clone() {
        let mut view1 = ViewState::new();
        view1.scene_name = Some("test".to_string());
        view1.bgm = Some("music.mp3".to_string());
        
        let view2 = view1.clone();
        assert_eq!(view1, view2);
    }

    #[test]
    fn test_view_state_equality() {
        let mut view1 = ViewState::new();
        let mut view2 = ViewState::new();
        
        assert_eq!(view1, view2);
        
        view1.scene_name = Some("test".to_string());
        assert_ne!(view1, view2);
        
        view2.scene_name = Some("test".to_string());
        assert_eq!(view1, view2);
    }
}
