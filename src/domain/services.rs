//! Domain services - Complex business logic that doesn't naturally fit in entities

use crate::domain::entities::{ExecutionSnapshot, StoryExecution};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::{
    BranchState, Choice, ComparisonOperation, ExecutionState, LabelName, ResourceId, SpeakerName,
    StoryCommand, StoryValue, VariableName, VariableOperation,
};

/// Core domain service for story execution logic
///
/// This service encapsulates the business rules for executing story commands
/// and managing execution state transitions.
pub struct StoryExecutionService;

impl StoryExecutionService {
    pub fn new() -> Self {
        Self
    }

    /// Execute the next command in the story
    pub fn execute_next_command(
        &self,
        execution: &mut StoryExecution,
    ) -> Result<ExecutionResult, DomainError> {
        if execution.is_finished() {
            return Ok(ExecutionResult::Finished);
        }

        let command = execution.current_command()?.clone();
        let result = self.execute_command(execution, &command)?;

        // Advance program counter for most commands
        if !matches!(
            result,
            ExecutionResult::WaitForBranchSelection(_) | ExecutionResult::Jump(_)
        ) {
            execution.state_mut().increment_program_counter();
        }

        Ok(result)
    }

    /// Execute a specific command
    fn execute_command(
        &self,
        execution: &mut StoryExecution,
        command: &StoryCommand,
    ) -> Result<ExecutionResult, DomainError> {
        match command {
            StoryCommand::Say { speaker, text } => {
                Ok(ExecutionResult::WaitForUser(ExecutionDirective::Say {
                    speaker: speaker.clone(),
                    text: text.clone(),
                }))
            }

            StoryCommand::PlayBgm { resource } => {
                Ok(ExecutionResult::Continue(ExecutionDirective::PlayBgm {
                    resource: resource.clone(),
                }))
            }

            StoryCommand::PlaySe { resource } => {
                Ok(ExecutionResult::Continue(ExecutionDirective::PlaySe {
                    resource: resource.clone(),
                }))
            }

            StoryCommand::ShowImage { resource } => {
                Ok(ExecutionResult::Continue(ExecutionDirective::ShowImage {
                    resource: resource.clone(),
                }))
            }

            StoryCommand::PlayMovie { resource } => Ok(ExecutionResult::WaitForUser(
                ExecutionDirective::PlayMovie {
                    resource: resource.clone(),
                },
            )),

            StoryCommand::Wait { duration_seconds } => Ok(ExecutionResult::WaitForTimer {
                duration_seconds: *duration_seconds,
                directive: ExecutionDirective::Wait {
                    duration_seconds: *duration_seconds,
                },
            }),

            StoryCommand::Branch { choices } => {
                let state = execution.state_mut();

                // Check if we already emitted this branch
                if let Some(branch_state) = state.branch_state() {
                    if branch_state.is_emitted() {
                        return Ok(ExecutionResult::WaitForBranchSelection(
                            branch_state.choices().to_vec(),
                        ));
                    }
                }

                // Set up branch state and emit directive
                let mut branch_state = BranchState::new(choices.clone());
                branch_state.mark_emitted();
                state.set_branch_state(Some(branch_state));

                Ok(ExecutionResult::WaitForBranchSelection(choices.clone()))
            }

            StoryCommand::Jump { label } => {
                execution.jump_to_label(label)?;
                Ok(ExecutionResult::Jump(label.clone()))
            }

            StoryCommand::Label { .. } => {
                // Labels are markers, just continue
                Ok(ExecutionResult::Continue(ExecutionDirective::Label))
            }

            StoryCommand::SetVariable { name, value } => {
                execution
                    .state_mut()
                    .set_variable(name.clone(), value.clone());
                Ok(ExecutionResult::Continue(
                    ExecutionDirective::VariableChanged {
                        name: name.clone(),
                        value: value.clone(),
                    },
                ))
            }

            StoryCommand::ModifyVariable {
                name,
                operation,
                value,
            } => {
                self.modify_variable(execution.state_mut(), name, operation, value)?;
                Ok(ExecutionResult::Continue(
                    ExecutionDirective::VariableChanged {
                        name: name.clone(),
                        value: execution
                            .state()
                            .get_variable(name)
                            .expect("variable must exist after successful modify_variable")
                            .clone(),
                    },
                ))
            }

            StoryCommand::JumpIf {
                variable,
                comparison,
                value,
                label,
            } => {
                if self.evaluate_condition(execution.state(), variable, comparison, value)? {
                    execution.jump_to_label(label)?;
                    Ok(ExecutionResult::Jump(label.clone()))
                } else {
                    Ok(ExecutionResult::Continue(
                        ExecutionDirective::ConditionEvaluated {
                            variable: variable.clone(),
                            result: false,
                        },
                    ))
                }
            }
        }
    }

    /// Handle branch selection
    pub fn select_branch_choice(
        &self,
        execution: &mut StoryExecution,
        choice_index: usize,
    ) -> Result<LabelName, DomainError> {
        let branch_state = execution
            .state()
            .branch_state()
            .ok_or_else(|| DomainError::execution_state_error("No active branch"))?;

        let choice = branch_state.choices().get(choice_index).ok_or_else(|| {
            DomainError::execution_state_error(format!(
                "Invalid choice index: {choice_index}, total choices: {}",
                branch_state.choices().len()
            ))
        })?;

        let target_label = choice.target_label().clone();

        // Clear branch state and jump to target
        execution.state_mut().set_branch_state(None);
        execution.jump_to_label(&target_label)?;

        Ok(target_label)
    }

    /// Modify a variable value
    fn modify_variable(
        &self,
        state: &mut ExecutionState,
        name: &VariableName,
        operation: &VariableOperation,
        value: &StoryValue,
    ) -> Result<(), DomainError> {
        let current_value = state
            .get_variable(name)
            .ok_or_else(|| DomainError::variable_not_found(name.clone()))?;

        let new_value = match (current_value, value, operation) {
            (StoryValue::Integer(a), StoryValue::Integer(b), VariableOperation::Add) => {
                StoryValue::Integer(a + b)
            }
            (StoryValue::Integer(a), StoryValue::Integer(b), VariableOperation::Subtract) => {
                StoryValue::Integer(a - b)
            }
            _ => {
                return Err(DomainError::VariableTypeMismatch {
                    variable: name.clone(),
                    expected: "Integer".to_string(),
                    actual: format!("{current_value:?}"),
                });
            }
        };

        state.set_variable(name.clone(), new_value);
        Ok(())
    }

    /// Evaluate a conditional expression
    fn evaluate_condition(
        &self,
        state: &ExecutionState,
        variable: &VariableName,
        comparison: &ComparisonOperation,
        expected_value: &StoryValue,
    ) -> Result<bool, DomainError> {
        let actual_value = state
            .get_variable(variable)
            .ok_or_else(|| DomainError::variable_not_found(variable.clone()))?;

        let result = match (actual_value, expected_value) {
            (StoryValue::Integer(a), StoryValue::Integer(b)) => match comparison {
                ComparisonOperation::Equal => a == b,
                ComparisonOperation::NotEqual => a != b,
                ComparisonOperation::LessThan => a < b,
                ComparisonOperation::LessThanOrEqual => a <= b,
                ComparisonOperation::GreaterThan => a > b,
                ComparisonOperation::GreaterThanOrEqual => a >= b,
            },
            (StoryValue::Boolean(a), StoryValue::Boolean(b)) => match comparison {
                ComparisonOperation::Equal => a == b,
                ComparisonOperation::NotEqual => a != b,
                _ => false,
            },
            (StoryValue::Text(a), StoryValue::Text(b)) => match comparison {
                ComparisonOperation::Equal => a == b,
                ComparisonOperation::NotEqual => a != b,
                _ => false,
            },
            _ => false,
        };

        Ok(result)
    }

    /// Create a save point
    pub fn create_save_point(&self, execution: &StoryExecution) -> ExecutionSnapshot {
        execution.create_snapshot()
    }

    /// Restore from save point
    pub fn restore_save_point(
        &self,
        execution: &mut StoryExecution,
        snapshot: ExecutionSnapshot,
    ) -> Result<(), DomainError> {
        execution.restore_from_snapshot(snapshot)
    }
}

impl Default for StoryExecutionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of executing a story command
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Continue to next command immediately
    Continue(ExecutionDirective),
    /// Wait for user input (e.g., after dialogue)
    WaitForUser(ExecutionDirective),
    /// Wait for timer to complete
    WaitForTimer {
        duration_seconds: f32,
        directive: ExecutionDirective,
    },
    /// Wait for branch selection from user
    WaitForBranchSelection(Vec<Choice>),
    /// Jump to another location
    Jump(LabelName),
    /// Story execution finished
    Finished,
}

/// Directives that describe what the presentation layer should do
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionDirective {
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
    Jump {
        label: LabelName,
    },
    Label,
    VariableChanged {
        name: VariableName,
        value: StoryValue,
    },
    ConditionEvaluated {
        variable: VariableName,
        result: bool,
    },
}
