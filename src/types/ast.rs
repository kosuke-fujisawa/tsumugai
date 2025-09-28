//! Abstract Syntax Tree representation of parsed scenarios

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Abstract syntax tree representation of a parsed scenario
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ast {
    /// Sequence of commands to execute
    pub nodes: Vec<AstNode>,
    /// Label to index mapping for jumps
    pub labels: HashMap<String, usize>,
}

impl Ast {
    pub fn new(nodes: Vec<AstNode>, labels: HashMap<String, usize>) -> Self {
        Self { nodes, labels }
    }

    pub fn get_label_index(&self, label: &str) -> Option<usize> {
        self.labels.get(label).copied()
    }

    pub fn get_node(&self, index: usize) -> Option<&AstNode> {
        self.nodes.get(index)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// A single node in the AST representing a command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AstNode {
    /// Display spoken text
    Say { speaker: String, text: String },
    /// Show image on specified layer
    ShowImage { layer: String, name: String },
    /// Play background music
    PlayBgm { name: String },
    /// Play sound effect
    PlaySe { name: String },
    /// Play movie/video
    PlayMovie { name: String },
    /// Wait for specified duration
    Wait { seconds: f32 },
    /// Present choices to user with jump targets
    Branch { choices: Vec<Choice> },
    /// Unconditional jump to label
    Jump { label: String },
    /// Conditional jump
    JumpIf {
        var: String,
        cmp: Comparison,
        value: String,
        label: String,
    },
    /// Set variable value
    Set { name: String, value: String },
    /// Modify variable value
    Modify {
        name: String,
        op: Operation,
        value: String,
    },
    /// Label marker (no-op during execution)
    Label { name: String },
    /// Clear specified image layer
    ClearLayer { layer: String },
}

/// A choice option in a branch command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub target: String,
}

/// Comparison operators for conditional jumps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Operations for variable modification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}
