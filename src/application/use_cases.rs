//! Application use cases - High-level business operations

use std::sync::Arc;
use crate::domain::{entities::*, services::*, value_objects::*};

/// Use case for playing back scenarios
pub struct ScenarioPlaybackUseCase {
    scenario_repository: Arc<dyn ScenarioRepositoryTrait>,
    execution_service: StoryExecutionService,
}

impl ScenarioPlaybackUseCase {
    pub fn new(
        scenario_repository: Arc<dyn ScenarioRepositoryTrait>
    ) -> Self {
        Self {
            scenario_repository,
            execution_service: StoryExecutionService::new(),
        }
    }

    pub async fn start_scenario(&self, scenario_id: ScenarioId) -> Result<StoryExecution, ApplicationError> {
        let scenario = self.scenario_repository
            .load_scenario(&scenario_id)
            .await
            .map_err(ApplicationError::Repository)?;

        let execution = StoryExecution::new(scenario)
            .map_err(ApplicationError::Domain)?;

        Ok(execution)
    }

    pub fn execute_next_step(&self, execution: &mut StoryExecution) -> Result<ExecutionResult, ApplicationError> {
        self.execution_service
            .execute_next_command(execution)
            .map_err(ApplicationError::Domain)
    }

    pub fn select_choice(&self, execution: &mut StoryExecution, choice_index: usize) -> Result<LabelName, ApplicationError> {
        self.execution_service
            .select_branch_choice(execution, choice_index)
            .map_err(ApplicationError::Domain)
    }
}

/// Use case for loading scenarios from various sources
pub struct ScenarioLoadingUseCase {
    file_repository: Arc<dyn FileRepositoryTrait>,
    markdown_parser: Arc<dyn MarkdownParserTrait>,
}

impl ScenarioLoadingUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepositoryTrait>,
        markdown_parser: Arc<dyn MarkdownParserTrait>
    ) -> Self {
        Self {
            file_repository,
            markdown_parser,
        }
    }

    pub async fn load_from_file(&self, file_path: &str) -> Result<Scenario, ApplicationError> {
        let content = self.file_repository
            .read_file(file_path)
            .await
            .map_err(ApplicationError::Infrastructure)?;

        let scenario = self.markdown_parser
            .parse_scenario(&content)
            .map_err(ApplicationError::Infrastructure)?;

        Ok(scenario)
    }
}

/// Use case for saving game state
pub struct SaveGameUseCase {
    save_repository: Arc<dyn SaveRepositoryTrait>,
}

impl SaveGameUseCase {
    pub fn new(save_repository: Arc<dyn SaveRepositoryTrait>) -> Self {
        Self { save_repository }
    }

    pub async fn save_game(&self, save_id: &str, snapshot: ExecutionSnapshot) -> Result<(), ApplicationError> {
        self.save_repository
            .save_snapshot(save_id, snapshot)
            .await
            .map_err(ApplicationError::Infrastructure)
    }
}

/// Use case for loading game state
pub struct LoadGameUseCase {
    save_repository: Arc<dyn SaveRepositoryTrait>,
}

impl LoadGameUseCase {
    pub fn new(save_repository: Arc<dyn SaveRepositoryTrait>) -> Self {
        Self { save_repository }
    }

    pub async fn load_game(&self, save_id: &str) -> Result<ExecutionSnapshot, ApplicationError> {
        self.save_repository
            .load_snapshot(save_id)
            .await
            .map_err(ApplicationError::Infrastructure)
    }
}

/// Application-level errors
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    Domain(#[from] crate::domain::errors::DomainError),
    #[error("Repository error: {0}")]
    Repository(RepositoryError),
    #[error("Infrastructure error: {0}")]
    Infrastructure(InfrastructureError),
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Scenario not found: {id}")]
    ScenarioNotFound { 
        id: String, 
        #[source] 
        source: Box<dyn std::error::Error + Send + Sync> 
    },
    #[error("Access denied: {message}")]
    AccessDenied { message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum InfrastructureError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    #[error("Parse error: {message}")]
    ParseError { message: String },
    #[error("IO error: {message}")]
    IoError { message: String },
}

// Repository traits (to be implemented by infrastructure layer)
#[async_trait::async_trait]
pub trait ScenarioRepositoryTrait: Send + Sync {
    async fn load_scenario(&self, id: &ScenarioId) -> Result<Scenario, RepositoryError>;
    async fn save_scenario(&self, scenario: &Scenario) -> Result<(), RepositoryError>;
    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait FileRepositoryTrait: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<String, InfrastructureError>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), InfrastructureError>;
    async fn file_exists(&self, path: &str) -> Result<bool, InfrastructureError>;
}

#[async_trait::async_trait]
pub trait SaveRepositoryTrait: Send + Sync {
    async fn save_snapshot(&self, id: &str, snapshot: ExecutionSnapshot) -> Result<(), InfrastructureError>;
    async fn load_snapshot(&self, id: &str) -> Result<ExecutionSnapshot, InfrastructureError>;
    async fn list_saves(&self) -> Result<Vec<String>, InfrastructureError>;
}

pub trait MarkdownParserTrait: Send + Sync {
    fn parse_scenario(&self, content: &str) -> Result<Scenario, InfrastructureError>;
}

pub trait ResourceResolverTrait: Send + Sync {
    fn resolve_bgm(&self, logical_name: &str) -> Option<std::path::PathBuf>;
    fn resolve_se(&self, logical_name: &str) -> Option<std::path::PathBuf>;
    fn resolve_image(&self, logical_name: &str) -> Option<std::path::PathBuf>;
    fn resolve_movie(&self, logical_name: &str) -> Option<std::path::PathBuf>;
}