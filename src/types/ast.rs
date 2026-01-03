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
    /// Declared conditions (from :::conditions blocks)
    pub conditions: std::collections::HashSet<String>,
}

impl Ast {
    pub fn new(nodes: Vec<AstNode>, labels: HashMap<String, usize>) -> Self {
        Self {
            nodes,
            labels,
            conditions: std::collections::HashSet::new(),
        }
    }

    pub fn with_conditions(
        nodes: Vec<AstNode>,
        labels: HashMap<String, usize>,
        conditions: std::collections::HashSet<String>,
    ) -> Self {
        Self {
            nodes,
            labels,
            conditions,
        }
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

    /// Build scene index (scene name -> AST index mapping)
    /// This is used for scene navigation in the debugger
    pub fn build_scene_index(&self) -> std::collections::HashMap<String, usize> {
        let mut scene_index = std::collections::HashMap::new();

        for (idx, node) in self.nodes.iter().enumerate() {
            if let AstNode::Scene { meta } = node {
                scene_index.insert(meta.name.clone(), idx);
            }
        }

        scene_index
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
    /// Unconditional jump to label (GOTO command)
    Goto { target: String },
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
    /// Scene marker with metadata
    Scene { meta: SceneMeta },
    /// Conditional block execution
    WhenBlock { condition: Expr, body: Vec<AstNode> },
}

/// A choice option in a branch command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub target: String,
    pub condition: Option<String>,
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

/// Expression type for conditional evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Expr {
    /// Boolean literal
    Bool(bool),
    /// Integer literal
    Number(i64),
    /// String literal
    String(String),
    /// Variable reference
    Var(String),
    /// Equal comparison
    Equal(Box<Expr>, Box<Expr>),
    /// Not equal comparison
    NotEqual(Box<Expr>, Box<Expr>),
    /// Less than comparison
    LessThan(Box<Expr>, Box<Expr>),
    /// Less than or equal comparison
    LessThanOrEqual(Box<Expr>, Box<Expr>),
    /// Greater than comparison
    GreaterThan(Box<Expr>, Box<Expr>),
    /// Greater than or equal comparison
    GreaterThanOrEqual(Box<Expr>, Box<Expr>),
    /// Logical AND
    And(Box<Expr>, Box<Expr>),
    /// Logical OR
    Or(Box<Expr>, Box<Expr>),
    /// Logical NOT
    Not(Box<Expr>),
}

/// Ending type for a scene
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EndingKind {
    Good,
    Bad,
    True,
    Normal,
    Custom(String),
}

/// Scene metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneMeta {
    pub name: String,
    pub ending: Option<EndingKind>,
}
