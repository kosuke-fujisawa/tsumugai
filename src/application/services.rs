//! Application services - Coordination of domain services

use std::sync::Arc;
use crate::domain::{entities::*, services::*};
use super::use_cases::*;

/// Application coordinator that orchestrates multiple domain services
pub struct ApplicationCoordinator {
    story_service: Arc<StoryExecutionService>,
    scenario_repository: Arc<dyn ScenarioRepositoryTrait>,
    #[allow(dead_code)] // Kept for future resource resolution integration
    resource_resolver: Arc<dyn ResourceResolverTrait>,
}

impl ApplicationCoordinator {
    pub fn new(
        story_service: Arc<StoryExecutionService>,
        scenario_repository: Arc<dyn ScenarioRepositoryTrait>,
        resource_resolver: Arc<dyn ResourceResolverTrait>,
    ) -> Self {
        Self {
            story_service,
            scenario_repository,
            resource_resolver,
        }
    }

    pub fn get_scenario_playback_use_case(&self) -> ScenarioPlaybackUseCase {
        ScenarioPlaybackUseCase::new(
            self.scenario_repository.clone(),
        )
    }

    pub async fn initialize_scenario(&self, scenario_id: crate::domain::value_objects::ScenarioId) -> Result<StoryExecution, ApplicationError> {
        let use_case = self.get_scenario_playback_use_case();
        use_case.start_scenario(scenario_id).await
    }

    pub fn execute_story_step(&self, execution: &mut StoryExecution) -> Result<ExecutionResult, ApplicationError> {
        self.story_service
            .execute_next_command(execution)
            .map_err(ApplicationError::Domain)
    }

    pub fn handle_user_choice(&self, execution: &mut StoryExecution, choice_index: usize) -> Result<crate::domain::value_objects::LabelName, ApplicationError> {
        self.story_service
            .select_branch_choice(execution, choice_index)
            .map_err(ApplicationError::Domain)
    }

    pub fn create_save_point(&self, execution: &StoryExecution) -> ExecutionSnapshot {
        self.story_service.create_save_point(execution)
    }

    pub fn restore_save_point(&self, execution: &mut StoryExecution, snapshot: ExecutionSnapshot) -> Result<(), ApplicationError> {
        self.story_service
            .restore_save_point(execution, snapshot)
            .map_err(ApplicationError::Domain)
    }
}