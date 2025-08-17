//! Domain entities - Core business objects with identity and lifecycle

use crate::domain::errors::*;
use crate::domain::value_objects::*;
use serde::{Deserialize, Serialize};

/// Scenario represents a complete visual novel scenario with commands and metadata
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    id: ScenarioId,
    title: String,
    commands: Vec<StoryCommand>,
    metadata: ScenarioMetadata,
}

impl Scenario {
    pub fn new(id: ScenarioId, title: String, commands: Vec<StoryCommand>) -> Self {
        Self {
            id,
            title,
            commands,
            metadata: ScenarioMetadata::default(),
        }
    }

    pub fn id(&self) -> &ScenarioId {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn commands(&self) -> &[StoryCommand] {
        &self.commands
    }

    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    pub fn get_command(&self, index: usize) -> Result<&StoryCommand, DomainError> {
        self.commands
            .get(index)
            .ok_or(DomainError::InvalidCommandIndex {
                index,
                max: self.commands.len(),
            })
    }

    pub fn find_label(&self, label: &LabelName) -> Option<usize> {
        self.commands
            .iter()
            .enumerate()
            .find_map(|(idx, cmd)| match cmd {
                StoryCommand::Label { name } if name == label => Some(idx),
                _ => None,
            })
    }

    pub fn validate_labels(&self) -> Result<(), DomainError> {
        let mut defined_labels = std::collections::HashSet::new();
        let mut referenced_labels = Vec::new();

        // Collect defined and referenced labels
        for (idx, command) in self.commands.iter().enumerate() {
            match command {
                StoryCommand::Label { name } => {
                    if !defined_labels.insert(name.clone()) {
                        return Err(DomainError::DuplicateLabel {
                            label: name.clone(),
                            line: idx + 1,
                        });
                    }
                }
                StoryCommand::Jump { label } => {
                    referenced_labels.push((label.clone(), idx + 1));
                }
                StoryCommand::JumpIf { label, .. } => {
                    referenced_labels.push((label.clone(), idx + 1));
                }
                StoryCommand::Branch { choices } => {
                    for choice in choices {
                        referenced_labels.push((choice.target_label().clone(), idx + 1));
                    }
                }
                _ => {}
            }
        }

        // Validate all referenced labels exist
        for (label, line) in referenced_labels {
            if !defined_labels.contains(&label) {
                return Err(DomainError::UndefinedLabel { label, line });
            }
        }

        Ok(())
    }
}

/// StoryExecution represents the current execution state of a scenario
#[derive(Debug, Clone, PartialEq)]
pub struct StoryExecution {
    scenario: Scenario,
    state: ExecutionState,
}

impl StoryExecution {
    pub fn new(scenario: Scenario) -> Result<Self, DomainError> {
        // Validate scenario before creating execution
        scenario.validate_labels()?;

        Ok(Self {
            scenario,
            state: ExecutionState::new(),
        })
    }

    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    pub fn state(&self) -> &ExecutionState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ExecutionState {
        &mut self.state
    }

    pub fn current_command(&self) -> Result<&StoryCommand, DomainError> {
        self.scenario.get_command(self.state.program_counter())
    }

    pub fn advance_to(&mut self, position: usize) -> Result<(), DomainError> {
        if position > self.scenario.command_count() {
            return Err(DomainError::InvalidCommandIndex {
                index: position,
                max: self.scenario.command_count(),
            });
        }
        self.state.set_program_counter(position);
        Ok(())
    }

    pub fn jump_to_label(&mut self, label: &LabelName) -> Result<(), DomainError> {
        let position =
            self.scenario
                .find_label(label)
                .ok_or_else(|| DomainError::UndefinedLabel {
                    label: label.clone(),
                    line: self.state.program_counter() + 1,
                })?;
        self.advance_to(position)
    }

    pub fn is_finished(&self) -> bool {
        self.state.program_counter() >= self.scenario.command_count()
    }

    pub fn create_snapshot(&self) -> ExecutionSnapshot {
        ExecutionSnapshot {
            program_counter: self.state.program_counter(),
            variables: self.state.variables().clone(),
            metadata: SnapshotMetadata::now(),
        }
    }

    pub fn restore_from_snapshot(
        &mut self,
        snapshot: ExecutionSnapshot,
    ) -> Result<(), DomainError> {
        if snapshot.program_counter > self.scenario.command_count() {
            return Err(DomainError::InvalidCommandIndex {
                index: snapshot.program_counter,
                max: self.scenario.command_count(),
            });
        }

        self.state.set_program_counter(snapshot.program_counter);
        *self.state.variables_mut() = snapshot.variables;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioMetadata {
    pub author: Option<String>,
    pub version: String,
    pub created_at: Option<String>,
    pub tags: Vec<String>,
}

impl Default for ScenarioMetadata {
    fn default() -> Self {
        Self {
            author: None,
            version: "1.0".to_string(),
            created_at: None,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    pub program_counter: usize,
    pub variables: VariableStore,
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub created_at: String,
    pub description: Option<String>,
}

impl SnapshotMetadata {
    pub fn now() -> Self {
        Self {
            created_at: "now".to_string(), // In real impl, use proper timestamp
            description: None,
        }
    }
}
