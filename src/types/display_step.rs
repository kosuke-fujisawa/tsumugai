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
    Dialogue { speaker: String, text: String },

    /// A narration line without a speaker
    Narration { text: String },

    /// A choice block that requires player input
    ChoiceBlock { choices: Vec<ChoiceItem> },

    /// A scene boundary (scene start)
    SceneBoundary { scene_name: String },
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
        self.images.is_empty() && self.bgm.is_none() && self.se.is_empty() && self.other.is_empty()
    }

    pub fn add_image(&mut self, layer: String, name: String) {
        self.images.push(ImageEffect {
            layer,
            name: Some(name),
        });
    }

    pub fn clear_layer(&mut self, layer: String) {
        self.images.push(ImageEffect { layer, name: None });
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
