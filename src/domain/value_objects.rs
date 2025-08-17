//! Domain value objects - Immutable objects that describe aspects of the domain

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Macro to implement common traits for string wrapper types
macro_rules! impl_string_wrapper {
    ($type:ident) => {
        impl From<String> for $type {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $type {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// Unique identifier for a scenario
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScenarioId(String);

impl ScenarioId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl_string_wrapper!(ScenarioId);

/// Label name for jump targets
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LabelName(String);

impl LabelName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl_string_wrapper!(LabelName);

/// Speaker name for dialogue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeakerName(String);

impl SpeakerName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl_string_wrapper!(SpeakerName);

/// Resource identifier for assets
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl_string_wrapper!(ResourceId);

/// Variable name for story variables
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VariableName(String);

impl VariableName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl_string_wrapper!(VariableName);

/// Story variable value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StoryValue {
    Integer(i32),
    Boolean(bool),
    Text(String),
}

impl StoryValue {
    pub fn as_integer(&self) -> Option<i32> {
        match self {
            StoryValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            StoryValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            StoryValue::Text(s) => Some(s),
            _ => None,
        }
    }
}

impl From<i32> for StoryValue {
    fn from(i: i32) -> Self {
        StoryValue::Integer(i)
    }
}

impl From<bool> for StoryValue {
    fn from(b: bool) -> Self {
        StoryValue::Boolean(b)
    }
}

impl From<String> for StoryValue {
    fn from(s: String) -> Self {
        StoryValue::Text(s)
    }
}

impl From<&str> for StoryValue {
    fn from(s: &str) -> Self {
        StoryValue::Text(s.to_string())
    }
}

/// Store for story variables
pub type VariableStore = BTreeMap<VariableName, StoryValue>;

/// Story commands - the core business operations
#[derive(Debug, Clone, PartialEq)]
pub enum StoryCommand {
    Label {
        name: LabelName,
    },
    Jump {
        label: LabelName,
    },
    Say {
        speaker: SpeakerName,
        text: String,
    },
    PlayBgm {
        resource: ResourceId,
    },
    PlaySe {
        resource: ResourceId,
    },
    ShowImage {
        resource: ResourceId,
    },
    PlayMovie {
        resource: ResourceId,
    },
    Wait {
        duration_seconds: f32,
    },
    Branch {
        choices: Vec<Choice>,
    },
    SetVariable {
        name: VariableName,
        value: StoryValue,
    },
    ModifyVariable {
        name: VariableName,
        operation: VariableOperation,
        value: StoryValue,
    },
    JumpIf {
        variable: VariableName,
        comparison: ComparisonOperation,
        value: StoryValue,
        label: LabelName,
    },
}

/// Branch choice
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    text: String,
    target_label: LabelName,
}

impl Choice {
    pub fn new(text: String, target_label: LabelName) -> Self {
        Self { text, target_label }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn target_label(&self) -> &LabelName {
        &self.target_label
    }
}

/// Variable operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariableOperation {
    Add,
    Subtract,
}

/// Comparison operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComparisonOperation {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Current execution state
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionState {
    program_counter: usize,
    variables: VariableStore,
    branch_state: Option<BranchState>,
}

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            program_counter: 0,
            variables: BTreeMap::new(),
            branch_state: None,
        }
    }

    pub fn program_counter(&self) -> usize {
        self.program_counter
    }

    pub fn set_program_counter(&mut self, pc: usize) {
        self.program_counter = pc;
    }

    pub fn increment_program_counter(&mut self) {
        self.program_counter += 1;
    }

    pub fn variables(&self) -> &VariableStore {
        &self.variables
    }

    pub fn variables_mut(&mut self) -> &mut VariableStore {
        &mut self.variables
    }

    pub fn get_variable(&self, name: &VariableName) -> Option<&StoryValue> {
        self.variables.get(name)
    }

    pub fn set_variable(&mut self, name: VariableName, value: StoryValue) {
        self.variables.insert(name, value);
    }

    pub fn branch_state(&self) -> Option<&BranchState> {
        self.branch_state.as_ref()
    }

    pub fn set_branch_state(&mut self, state: Option<BranchState>) {
        self.branch_state = state;
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for branch execution
#[derive(Debug, Clone, PartialEq)]
pub struct BranchState {
    choices: Vec<Choice>,
    emitted: bool,
}

impl BranchState {
    pub fn new(choices: Vec<Choice>) -> Self {
        Self {
            choices,
            emitted: false,
        }
    }

    pub fn choices(&self) -> &[Choice] {
        &self.choices
    }

    pub fn is_emitted(&self) -> bool {
        self.emitted
    }

    pub fn mark_emitted(&mut self) {
        self.emitted = true;
    }
}
