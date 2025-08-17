//! High-level story engine API - Facade pattern for easy library usage

use crate::application::use_cases::ScenarioPlaybackUseCase;
use crate::contracts::{StepResult, StoryEngineError};
use crate::domain::{
    entities::{Scenario, StoryExecution},
    services::ExecutionResult,
};

/// High-level story engine that provides a simple API for visual novel execution
pub struct StoryEngine {
    playback_use_case: ScenarioPlaybackUseCase,
    execution: StoryExecution,
    // Bridge to new API for consistency - kept for future integration
    #[allow(dead_code)]
    new_engine: Option<crate::application::engine::Engine>,
}

impl StoryEngine {
    /// Create a new story engine from a scenario and use cases
    pub fn new(execution: StoryExecution, playback_use_case: ScenarioPlaybackUseCase) -> Self {
        Self {
            playback_use_case,
            execution,
            new_engine: None,
        }
    }

    /// Create a new story engine with both old and new API engines for consistency
    pub fn with_new_engine(
        execution: StoryExecution,
        playback_use_case: ScenarioPlaybackUseCase,
        new_engine: crate::application::engine::Engine,
    ) -> Self {
        Self {
            playback_use_case,
            execution,
            new_engine: Some(new_engine),
        }
    }

    /// Create a story engine from markdown content with default setup
    ///
    /// Note: This is a placeholder implementation that creates an empty scenario.
    /// TODO: Future integration should pass the actual parsed scenario from new_engine to domain layer.
    pub async fn from_markdown(content: &str) -> Result<Self, StoryEngineError> {
        use crate::application::engine::Engine as NewEngine;
        use crate::infrastructure::repositories::InMemoryScenarioRepository;
        use crate::infrastructure::resource_resolution::InMemoryResourceResolver;
        use std::sync::Arc;

        // Create engine using new API
        let new_engine = NewEngine::from_markdown(content)
            .map_err(|e| StoryEngineError::parsing(format!("Failed to parse markdown: {e:?}")))?;

        // Create default dependencies for domain layer
        // Create an adapter from domain ScenarioRepository to application ScenarioRepositoryTrait
        struct ScenarioRepositoryAdapter {
            inner: InMemoryScenarioRepository,
        }

        impl ScenarioRepositoryAdapter {
            fn new() -> Self {
                Self {
                    inner: InMemoryScenarioRepository::new(),
                }
            }
        }

        #[async_trait::async_trait]
        impl crate::application::use_cases::ScenarioRepositoryTrait for ScenarioRepositoryAdapter {
            async fn load_scenario(
                &self,
                id: &crate::domain::value_objects::ScenarioId,
            ) -> Result<
                crate::domain::entities::Scenario,
                crate::application::use_cases::RepositoryError,
            > {
                use crate::domain::repositories::ScenarioRepository;
                self.inner.load_scenario(id).await.map_err(|e| {
                    crate::application::use_cases::RepositoryError::ScenarioNotFound {
                        id: id.as_str().to_string(),
                        source: Box::new(e),
                    }
                })
            }

            async fn save_scenario(
                &self,
                scenario: &crate::domain::entities::Scenario,
            ) -> Result<(), crate::application::use_cases::RepositoryError> {
                use crate::domain::repositories::ScenarioRepository;
                self.inner.save_scenario(scenario).await.map_err(|_| {
                    crate::application::use_cases::RepositoryError::AccessDenied {
                        message: "Save not supported".to_string(),
                    }
                })
            }

            async fn list_scenarios(
                &self,
            ) -> Result<
                Vec<crate::domain::value_objects::ScenarioId>,
                crate::application::use_cases::RepositoryError,
            > {
                use crate::domain::repositories::ScenarioRepository;
                self.inner.list_scenarios().await.map_err(|_| {
                    crate::application::use_cases::RepositoryError::AccessDenied {
                        message: "List not supported".to_string(),
                    }
                })
            }
        }

        let scenario_repository = Arc::new(ScenarioRepositoryAdapter::new());

        // Create an adapter from infrastructure ResourceResolver to application ResourceResolverTrait
        struct ResourceResolverAdapter {
            inner: Box<dyn crate::infrastructure::resource_resolution::ResourceResolver>,
        }

        impl ResourceResolverAdapter {
            fn new() -> Self {
                Self {
                    inner: Box::new(InMemoryResourceResolver::new()),
                }
            }
        }

        impl crate::application::use_cases::ResourceResolverTrait for ResourceResolverAdapter {
            fn resolve_bgm(&self, logical_name: &str) -> Option<std::path::PathBuf> {
                let resource_id = crate::domain::value_objects::ResourceId::from(logical_name);
                self.inner.resolve_bgm(&resource_id)
            }

            fn resolve_se(&self, logical_name: &str) -> Option<std::path::PathBuf> {
                let resource_id = crate::domain::value_objects::ResourceId::from(logical_name);
                self.inner.resolve_se(&resource_id)
            }

            fn resolve_image(&self, logical_name: &str) -> Option<std::path::PathBuf> {
                let resource_id = crate::domain::value_objects::ResourceId::from(logical_name);
                self.inner.resolve_image(&resource_id)
            }

            fn resolve_movie(&self, logical_name: &str) -> Option<std::path::PathBuf> {
                let resource_id = crate::domain::value_objects::ResourceId::from(logical_name);
                self.inner.resolve_movie(&resource_id)
            }
        }

        let resource_resolver = Arc::new(ResourceResolverAdapter::new());
        let playback_use_case =
            ScenarioPlaybackUseCase::new(scenario_repository, resource_resolver);

        // Create a dummy execution for compatibility - this will need proper integration
        let scenario = crate::domain::entities::Scenario::new(
            crate::domain::value_objects::ScenarioId::new("temp".to_string()),
            "Temporary Scenario".to_string(),
            Vec::new(),
        );
        let execution = StoryExecution::new(scenario)
            .map_err(|e| StoryEngineError::domain(format!("Failed to create execution: {e:?}")))?;

        Ok(Self::with_new_engine(
            execution,
            playback_use_case,
            new_engine,
        ))
    }

    /// Execute the next step in the story
    pub async fn step(&mut self) -> Result<StepResult, StoryEngineError> {
        let result = self
            .playback_use_case
            .execute_next_step(&mut self.execution)
            .map_err(|e| StoryEngineError::domain(e.to_string()))?;

        Ok(match result {
            ExecutionResult::Continue(directive) => StepResult::Continue(directive),
            ExecutionResult::WaitForUser(directive) => StepResult::WaitForUser(directive),
            ExecutionResult::WaitForTimer { directive, .. } => StepResult::WaitForUser(directive),
            ExecutionResult::WaitForBranchSelection(choices) => StepResult::WaitForChoice(choices),
            ExecutionResult::Jump(_) => {
                // After a jump, immediately execute the next command
                Box::pin(self.step()).await?
            }
            ExecutionResult::Finished => StepResult::Finished,
        })
    }

    /// Choose an option when in a branch state
    pub async fn choose(&mut self, choice_index: usize) -> Result<(), StoryEngineError> {
        self.playback_use_case
            .select_choice(&mut self.execution, choice_index)
            .map_err(|e| StoryEngineError::domain(e.to_string()))?;
        Ok(())
    }

    /// Save the current execution state (placeholder for full implementation)
    pub async fn save(&self) -> Result<(), StoryEngineError> {
        // Full implementation requires save use case
        Err(StoryEngineError::configuration(
            "Save functionality requires proper use case setup",
        ))
    }

    /// Load a previously saved execution state (placeholder for full implementation)
    pub async fn load(&mut self) -> Result<bool, StoryEngineError> {
        // Full implementation requires load use case
        Err(StoryEngineError::configuration(
            "Load functionality requires proper use case setup",
        ))
    }

    /// Get the current scenario
    pub fn scenario(&self) -> &Scenario {
        self.execution.scenario()
    }

    /// Get the current execution state (read-only)
    pub fn execution_state(&self) -> &crate::domain::value_objects::ExecutionState {
        self.execution.state()
    }

    /// Check if the story is finished
    pub fn is_finished(&self) -> bool {
        self.execution.is_finished()
    }
}
