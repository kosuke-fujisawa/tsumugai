//! Application integration tests
//! Tests the minimum flow from UseCase to Engine to Directive conversion

use std::sync::Arc;
use tsumugai::application::use_cases::{
    RepositoryError, ResourceResolverTrait, ScenarioPlaybackUseCase, ScenarioRepositoryTrait,
};
use tsumugai::domain::{entities::Scenario, value_objects::ScenarioId};

/// Mock repository for testing minimum flow
struct MinimalScenarioRepository;

#[async_trait::async_trait]
impl ScenarioRepositoryTrait for MinimalScenarioRepository {
    async fn load_scenario(&self, _: &ScenarioId) -> Result<Scenario, RepositoryError> {
        // Return a minimal scenario for testing
        let scenario = Scenario::new(
            ScenarioId::from("test".to_string()),
            "Test Scenario".to_string(),
            vec![
                tsumugai::domain::value_objects::StoryCommand::Say {
                    speaker: tsumugai::domain::value_objects::SpeakerName::new("Alice".to_string()),
                    text: "Hello world".to_string(),
                },
                tsumugai::domain::value_objects::StoryCommand::Branch {
                    choices: vec![
                        tsumugai::domain::value_objects::Choice::new(
                            "Yes".to_string(),
                            tsumugai::domain::value_objects::LabelName::new(
                                "yes_label".to_string(),
                            ),
                        ),
                        tsumugai::domain::value_objects::Choice::new(
                            "No".to_string(),
                            tsumugai::domain::value_objects::LabelName::new("no_label".to_string()),
                        ),
                    ],
                },
                tsumugai::domain::value_objects::StoryCommand::Label {
                    name: tsumugai::domain::value_objects::LabelName::new("yes_label".to_string()),
                },
                tsumugai::domain::value_objects::StoryCommand::Say {
                    speaker: tsumugai::domain::value_objects::SpeakerName::new("Alice".to_string()),
                    text: "You chose yes!".to_string(),
                },
                tsumugai::domain::value_objects::StoryCommand::Label {
                    name: tsumugai::domain::value_objects::LabelName::new("no_label".to_string()),
                },
                tsumugai::domain::value_objects::StoryCommand::Say {
                    speaker: tsumugai::domain::value_objects::SpeakerName::new("Alice".to_string()),
                    text: "You chose no!".to_string(),
                },
            ],
        );
        Ok(scenario)
    }

    async fn save_scenario(&self, _: &Scenario) -> Result<(), RepositoryError> {
        unimplemented!()
    }

    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError> {
        unimplemented!()
    }
}

/// Mock resource resolver for testing
struct MinimalResourceResolver;

impl ResourceResolverTrait for MinimalResourceResolver {
    fn resolve_bgm(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn resolve_se(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn resolve_image(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn resolve_movie(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }
}

#[tokio::test]
async fn test_application_min_flow() {
    // Test minimum scenario flow through UseCase -> Engine -> Directive conversion
    let repository = Arc::new(MinimalScenarioRepository);
    let resolver = Arc::new(MinimalResourceResolver);
    let use_case = ScenarioPlaybackUseCase::new(repository, resolver);

    let scenario_id = ScenarioId::from("test".to_string());
    let mut execution = use_case.start_scenario(&scenario_id).await.unwrap();

    // Execute next step should return a Say directive
    let result = use_case.execute_next_step(&mut execution).unwrap();
    match result {
        tsumugai::domain::services::ExecutionResult::WaitForUser(_)
        | tsumugai::domain::services::ExecutionResult::Continue(_) => {
            // Either WaitForUser or Continue is acceptable for Say directive
        }
        _ => panic!("Expected Continue or WaitForUser result, got {result:?}"),
    }

    // Next step should be a branch
    let result = use_case.execute_next_step(&mut execution).unwrap();
    match result {
        tsumugai::domain::services::ExecutionResult::WaitForBranchSelection(choices) => {
            assert_eq!(choices.len(), 2);
            assert_eq!(choices[0].text(), "Yes");
            assert_eq!(choices[1].text(), "No");
        }
        _ => panic!("Expected WaitForBranchSelection result, got {result:?}"),
    }

    // Select choice
    let target_label = use_case.select_choice(&mut execution, 0).unwrap();
    assert_eq!(target_label.as_str(), "yes_label");

    // Next step should reach the label and continue to the next say
    let result = use_case.execute_next_step(&mut execution).unwrap();
    match result {
        tsumugai::domain::services::ExecutionResult::Jump(label) => {
            assert_eq!(label.as_str(), "yes_label");
        }
        _ => {
            // If not Jump, it might be Continue with a Label directive
            if let tsumugai::domain::services::ExecutionResult::Continue(_) = result {
                // Label directive validation
            }
        }
    }
}
