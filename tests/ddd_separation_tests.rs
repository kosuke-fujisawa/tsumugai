//! TDD tests for DDD separation
//! Ensures domain layer has no I/O dependencies and proper separation

use tsumugai::domain::{entities::*, services::*, value_objects::*};

#[cfg(test)]
mod ddd_tests {
    use super::*;

    /// Test: Domain should not depend on I/O
    /// Metric: Domain types should be constructible without file system or network
    #[test]
    fn test_domain_pure_construction() {
        // Should be able to create domain objects without I/O
        let scenario_id = ScenarioId::new("test_scenario".to_string());
        let commands = vec![
            StoryCommand::Say {
                speaker: SpeakerName::new("Hero".to_string()),
                text: "Hello world".to_string(),
            },
            StoryCommand::Label {
                name: LabelName::new("start".to_string()),
            },
        ];

        let scenario = Scenario::new(scenario_id, "Test Scenario".to_string(), commands);
        let execution = StoryExecution::new(scenario).expect("Should create execution");

        // Domain service should work without external dependencies
        let service = StoryExecutionService::new();
        let _result = service.create_save_point(&execution);

        // This should all work without any I/O
        // Test passes if no panics occur during construction
    }

    /// Test: Domain error types should be well-formed
    /// Metric: Domain errors should have proper context and be serializable
    #[test]
    fn test_domain_error_quality() {
        // Create scenario with undefined label reference
        let scenario_id = ScenarioId::new("test".to_string());
        let commands = vec![StoryCommand::Jump {
            label: LabelName::new("undefined".to_string()),
        }];

        let scenario = Scenario::new(scenario_id, "Test".to_string(), commands);
        let execution_result = StoryExecution::new(scenario);

        // Should get a proper domain error
        match execution_result {
            Err(domain_error) => {
                let error_string = format!("{}", domain_error);
                assert!(
                    error_string.contains("undefined"),
                    "Error should mention undefined label"
                );
                assert!(
                    error_string.len() > 10,
                    "Error message should be descriptive"
                );
            }
            Ok(_) => panic!("Expected domain error for undefined label"),
        }
    }

    /// Test: Execution service should handle complex scenarios
    /// Metric: Service should execute branching scenarios correctly
    #[test]
    fn test_execution_service_branching() {
        let scenario_id = ScenarioId::new("branch_test".to_string());
        let commands = vec![
            StoryCommand::Say {
                speaker: SpeakerName::new("Guide".to_string()),
                text: "Choose".to_string(),
            },
            StoryCommand::Branch {
                choices: vec![
                    Choice::new("Left".to_string(), LabelName::new("left".to_string())),
                    Choice::new("Right".to_string(), LabelName::new("right".to_string())),
                ],
            },
            StoryCommand::Label {
                name: LabelName::new("left".to_string()),
            },
            StoryCommand::Say {
                speaker: SpeakerName::new("Guide".to_string()),
                text: "You went left".to_string(),
            },
            StoryCommand::Label {
                name: LabelName::new("right".to_string()),
            },
            StoryCommand::Say {
                speaker: SpeakerName::new("Guide".to_string()),
                text: "You went right".to_string(),
            },
        ];

        let scenario = Scenario::new(scenario_id, "Branch Test".to_string(), commands);
        let mut execution = StoryExecution::new(scenario).expect("Should create execution");
        let service = StoryExecutionService::new();

        // Execute first command (SAY)
        let result1 = service
            .execute_next_command(&mut execution)
            .expect("Should execute");
        match result1 {
            ExecutionResult::WaitForUser(ExecutionDirective::Say { speaker, text }) => {
                assert_eq!(speaker.as_str(), "Guide");
                assert_eq!(text, "Choose");
            }
            _ => panic!("Expected Say directive"),
        }

        // Execute branch command
        let result2 = service
            .execute_next_command(&mut execution)
            .expect("Should execute");
        match result2 {
            ExecutionResult::WaitForBranchSelection(choices) => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0].text(), "Left");
                assert_eq!(choices[1].text(), "Right");
            }
            _ => panic!("Expected Branch selection"),
        }

        // Select first choice (Left)
        let selected_label = service
            .select_branch_choice(&mut execution, 0)
            .expect("Should select choice");
        assert_eq!(selected_label.as_str(), "left");

        // Execute next command (should be at left label)
        let result3 = service
            .execute_next_command(&mut execution)
            .expect("Should execute");
        match result3 {
            ExecutionResult::Continue(ExecutionDirective::Label) => {
                // Expected - label command
            }
            _ => panic!("Expected Label directive"),
        }

        // Execute next command (should be the Say after left label)
        let result4 = service
            .execute_next_command(&mut execution)
            .expect("Should execute");
        match result4 {
            ExecutionResult::WaitForUser(ExecutionDirective::Say { text, .. }) => {
                assert_eq!(text, "You went left");
            }
            _ => panic!("Expected Say directive for left path"),
        }
    }

    /// Test: Variable operations should be domain-pure
    /// Metric: Variable manipulation should not require external dependencies
    #[test]
    fn test_variable_operations_pure() {
        let scenario_id = ScenarioId::new("var_test".to_string());
        let commands = vec![
            StoryCommand::SetVariable {
                name: VariableName::new("score".to_string()),
                value: StoryValue::Integer(10),
            },
            StoryCommand::ModifyVariable {
                name: VariableName::new("score".to_string()),
                operation: VariableOperation::Add,
                value: StoryValue::Integer(5),
            },
            StoryCommand::JumpIf {
                variable: VariableName::new("score".to_string()),
                comparison: ComparisonOperation::GreaterThan,
                value: StoryValue::Integer(12),
                label: LabelName::new("high_score".to_string()),
            },
            StoryCommand::Say {
                speaker: SpeakerName::new("System".to_string()),
                text: "Low score".to_string(),
            },
            StoryCommand::Label {
                name: LabelName::new("high_score".to_string()),
            },
            StoryCommand::Say {
                speaker: SpeakerName::new("System".to_string()),
                text: "High score".to_string(),
            },
        ];

        let scenario = Scenario::new(scenario_id, "Variable Test".to_string(), commands);
        let mut execution = StoryExecution::new(scenario).expect("Should create execution");
        let service = StoryExecutionService::new();

        // Set variable
        let _result1 = service
            .execute_next_command(&mut execution)
            .expect("Should set variable");

        // Modify variable
        let _result2 = service
            .execute_next_command(&mut execution)
            .expect("Should modify variable");

        // Check variable value
        let score = execution
            .state()
            .get_variable(&VariableName::new("score".to_string()));
        assert_eq!(score, Some(&StoryValue::Integer(15)));

        // Execute conditional jump (should jump because 15 > 12)
        let result3 = service
            .execute_next_command(&mut execution)
            .expect("Should execute conditional");
        match result3 {
            ExecutionResult::Jump(label) => {
                assert_eq!(label.as_str(), "high_score");
            }
            _ => panic!("Expected jump to high_score"),
        }
    }

    /// Test: Snapshot functionality should be pure
    /// Metric: Save/restore should work without I/O
    #[test]
    fn test_snapshot_pure() {
        let scenario_id = ScenarioId::new("snapshot_test".to_string());
        let commands = vec![
            StoryCommand::SetVariable {
                name: VariableName::new("progress".to_string()),
                value: StoryValue::Integer(1),
            },
            StoryCommand::Say {
                speaker: SpeakerName::new("Hero".to_string()),
                text: "Checkpoint".to_string(),
            },
        ];

        let scenario = Scenario::new(scenario_id, "Snapshot Test".to_string(), commands);
        let mut execution = StoryExecution::new(scenario).expect("Should create execution");
        let service = StoryExecutionService::new();

        // Execute first command and create snapshot
        let _result1 = service
            .execute_next_command(&mut execution)
            .expect("Should set variable");
        let snapshot = service.create_save_point(&execution);

        // Execute second command
        let _result2 = service
            .execute_next_command(&mut execution)
            .expect("Should execute say");

        // Verify we're at position 2
        assert_eq!(execution.state().program_counter(), 2);

        // Restore from snapshot
        service
            .restore_save_point(&mut execution, snapshot)
            .expect("Should restore");

        // Should be back at position 1
        assert_eq!(execution.state().program_counter(), 1);

        // Variable should still be set
        let progress = execution
            .state()
            .get_variable(&VariableName::new("progress".to_string()));
        assert_eq!(progress, Some(&StoryValue::Integer(1)));
    }
}
